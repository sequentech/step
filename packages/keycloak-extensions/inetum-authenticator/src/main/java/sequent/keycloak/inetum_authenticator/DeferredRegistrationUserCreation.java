// SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
// SPDX-FileCopyrightText: 2016 Red Hat, Inc. and/or its affiliates
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.inetum_authenticator;

import static java.util.Arrays.asList;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.UriBuilder;
import java.net.URI;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.Map.Entry;
import java.util.Optional;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.Config;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.authentication.AuthenticationFlowException;
import org.keycloak.authentication.FormAction;
import org.keycloak.authentication.FormActionFactory;
import org.keycloak.authentication.FormContext;
import org.keycloak.authentication.ValidationContext;
import org.keycloak.authentication.forms.RegistrationPage;
import org.keycloak.events.Details;
import org.keycloak.events.Errors;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.utils.FormMessage;
import org.keycloak.policy.PasswordPolicyManagerProvider;
import org.keycloak.policy.PolicyError;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.services.messages.Messages;
import org.keycloak.services.validation.Validation;
import org.keycloak.userprofile.AttributeMetadata;
import org.keycloak.userprofile.Attributes;
import org.keycloak.userprofile.UserProfile;
import org.keycloak.userprofile.UserProfileContext;
import org.keycloak.userprofile.UserProfileProvider;
import org.keycloak.userprofile.ValidationException;

@JBossLog
@AutoService(FormActionFactory.class)
public class DeferredRegistrationUserCreation implements FormAction, FormActionFactory {

  public static final String PROVIDER_ID = "deferred-registration-user-creation";
  public static final String SEARCH_ATTRIBUTES = "search-attributes";
  public static final String UNSET_ATTRIBUTES = "unset-attributes";
  public static final String UNIQUE_ATTRIBUTES = "unique-attributes";
  public static final String PASSWORD_REQUIRED = "password-required";
  public static final String FORM_MODE = "form-mode";

  // define the form modes as an enum with string values:
  public enum FormMode {
    REGISTRATION("REGISTRATION"),
    LOGIN("LOGIN");

    private final String value;

    FormMode(String value) {
      this.value = value;
    }

    public String getValue() {
      return value;
    }
  }

  public static final String VERIFIED_VALUE = "VERIFIED";
  public static final String VERIFIED_DEFAULT_ID = "sequent.read-only.id-card-number-validated";
  public static final String ID_NUMBER = "sequent.read-only.id-card-number";
  public static final String PHONE_NUMBER = "sequent.read-only.mobile-number";
  public static final String MISSING_FIELDS = "Missing Fields";
  public static final String PASSWORD_NOT_MATCHED = "Passwords not matched";
  public static final String PASSWORD_NOT_STRONG = "Passwords not strong enough";
  public static final String INVALID_EMAIL = "Invalid email";
  public static final String INVALID_REGISTRATION = "Invalid registation";
  public static final String INVALID_INPUT = "Invalid input";

  public static final String MISSING_FIELDS_ERROR = "error_user_attribute_required";

  @Override
  public String getHelpText() {
    return "Sequent: This action must always be first! Validates the username and user profile of the user in validation phase.  In success phase, this will save the info necessary in auth notes to create the user - or attach to a pre-registered user.";
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {

    ProviderConfigProperty formMode =
        new ProviderConfigProperty(
            FORM_MODE,
            "Form Mode",
            "Show the form in Registration or Login Mode.",
            ProviderConfigProperty.LIST_TYPE,
            FormMode.REGISTRATION.name());
    formMode.setOptions(asList(FormMode.REGISTRATION.name(), FormMode.LOGIN.name()));

    // Define configuration properties
    return List.of(
        new ProviderConfigProperty(
            Utils.USER_STATUS_ATTRIBUTE,
            "User Status Attribute",
            "The name of the user validation status attribute.",
            ProviderConfigProperty.STRING_TYPE,
            VERIFIED_DEFAULT_ID),
        new ProviderConfigProperty(
            SEARCH_ATTRIBUTES,
            "Search Attributes",
            "Comma-separated list of attributes to use for searching the user in auth notes.",
            ProviderConfigProperty.STRING_TYPE,
            ""),
        new ProviderConfigProperty(
            UNSET_ATTRIBUTES,
            "Unset Attributes",
            "Comma-separated list of attributes that the user needs to have unset and otherwise the authenticator should fail.",
            ProviderConfigProperty.STRING_TYPE,
            ""),
        new ProviderConfigProperty(
            UNIQUE_ATTRIBUTES,
            "Unique Attributes",
            "Comma-separated list of attributes that should not be set to other users and otherwise the authenticator should fail.",
            ProviderConfigProperty.STRING_TYPE,
            ""),
        new ProviderConfigProperty(
            PASSWORD_REQUIRED,
            "Password Required",
            "Define if the password will be shown in the form.",
            ProviderConfigProperty.BOOLEAN_TYPE,
            "true"),
        formMode);
  }

  @Override
  public void validate(ValidationContext context) {
    log.info("validate: start");

    // Retrieve the configuration
    AuthenticatorConfigModel config = context.getAuthenticatorConfig();
    final Map<String, String> configMap = config.getConfig();

    // Extract the attributes to search and update from the configuration
    final String searchAttributes = configMap.get(SEARCH_ATTRIBUTES);
    final String unsetAttributes = configMap.get(UNSET_ATTRIBUTES);
    final String uniqueAttributes = configMap.get(UNIQUE_ATTRIBUTES);
    final String formMode = configMap.get(FORM_MODE);
    final String verifiedAttributeId =
        Optional.ofNullable(configMap.get(UNIQUE_ATTRIBUTES)).orElse(VERIFIED_DEFAULT_ID);
    boolean passwordRequired =
        Boolean.parseBoolean(Optional.ofNullable(configMap.get(PASSWORD_REQUIRED)).orElse("true"));

    // Parse attributes lists
    List<String> searchAttributesList = parseAttributesList(searchAttributes);
    List<String> unsetAttributesList = parseAttributesList(unsetAttributes);
    List<String> uniqueAttributesList = parseAttributesList(uniqueAttributes);

    // Get the form data
    MultivaluedMap<String, String> formData = context.getHttpRequest().getDecodedFormParameters();
    context.getEvent().detail(Details.REGISTER_METHOD, "form");
    UserProfile profile = getOrCreateUserProfile(context, formData);

    UserModel user = null;
    if (!searchAttributesList.isEmpty()) {
      user = Utils.lookupUserByFormData(context, searchAttributesList, formData);
    }
    buildEventDetails(formData, context, user);
    Attributes attributes = profile.getAttributes();
    String email = attributes.getFirst(UserModel.EMAIL);

    if (context.getRealm().isRegistrationEmailAsUsername()) {
      context.getEvent().detail(Details.USERNAME, email);
    }

    try {
      profile.validate();
      // If email validation exception was not raised and an email was
      // provided, validation should have thrown an Messages.EMAIL_EXISTS
      // exception, so show invalid email error here
      if (email != null && !email.isBlank()) {
        log.info("validate: validation exception was not raised and an email was provided");
        context.error(INVALID_EMAIL);
        List<FormMessage> errors = new ArrayList<>();
        errors.add(new FormMessage(RegistrationPage.FIELD_EMAIL, Messages.INVALID_EMAIL));
        context.validationError(formData, errors);
        return;
      }
    } catch (ValidationException pve) {

      log.info("validate: Entering validation errors:" + pve.getErrors());

      // Filter email exists and username exists - this is to be expected
      // If username is hidden ignore the missing username validation error.
      List<ValidationException.Error> filteredErrors =
          pve.getErrors().stream()
              .filter(
                  error ->
                      ((!context.getRealm().isRegistrationEmailAsUsername()
                              && !Messages.USERNAME_EXISTS.equals(error.getMessage()))
                          && !Messages.EMAIL_EXISTS.equals(error.getMessage())
                          // If username is hidden ignore the missing username validation error.
                          && !(Messages.MISSING_USERNAME.equals(error.getMessage())
                              && "true"
                                  .equals(
                                      getAnnotationValueFromProfile(
                                          profile, UserModel.USERNAME, "hidden")))))
              .collect(Collectors.toList());
      List<FormMessage> errors = Validation.getFormErrorsFromValidation(filteredErrors);

      if (pve.hasError(Messages.INVALID_EMAIL)) {
        context.getEvent().detail(Details.EMAIL, attributes.getFirst(UserModel.EMAIL));
      }
      // if error is empty but we are here, then the exception was related
      // to error to be ignored (username/email exists), so we ignore them
      // and continue
      if (errors.isEmpty()) {

        // if errors is not empty, show them
      } else {
        if (checkMissingFields(context, errors)) {
          log.error("some missing fields");
        } else if (!pve.hasError(Messages.EMAIL_EXISTS)) {
          context.error(INVALID_EMAIL);
        } else {
          context.error(INVALID_REGISTRATION);
        }
        log.info(errors);
        context.validationError(formData, errors);
        return;
      }
    }

    // Lookup user by attributes using form data
    if (!searchAttributesList.isEmpty()) {
      if (user == null) {
        String sessionId = context.getAuthenticationSession().getParentSession().getId();
        log.errorv("validate(): User could not be found. Error code: {0}", sessionId);
        // Display what the user set in formData for the search attributes
        for (String attribute : searchAttributesList) {
          log.errorv(
              "validate(): Register form data {0}: {1}", attribute, formData.getFirst(attribute));
        }
        context.error(Utils.ERROR_MESSAGE_USER_NOT_FOUND);
        List<FormMessage> errors = new ArrayList<>();
        errors.add(new FormMessage(null, Utils.ERROR_USER_NOT_FOUND, sessionId));
        context.validationError(formData, errors);
        return;
      }

      if (formMode.equals(FormMode.LOGIN.getValue())) {
        context.setUser(user);
      }

      // Check if the voter has already been validated
      log.infov("validate: Is user validated id {0}", verifiedAttributeId);
      var verifiedAttributeValue = user.getFirstAttribute(verifiedAttributeId);

      log.infov("validate: Is user validated? {0} == {1}", VERIFIED_VALUE, verifiedAttributeValue);
      if (VERIFIED_VALUE.equalsIgnoreCase(verifiedAttributeValue)) {
        log.infov("validate: Is user validated? true");
        context.getAuthenticationSession().setAuthNote(verifiedAttributeId, verifiedAttributeValue);
        context.success();
        return;
      }

      // Check that the user doesn't have set any of the unset attributes
      Optional<String> unsetAttributesChecked = checkUnsetAttributes(user, unsetAttributesList);

      if (unsetAttributesChecked.isPresent()) {
        String sessionId = context.getAuthenticationSession().getParentSession().getId();
        log.errorv("validate(): Some user unset attributes are set. Error code: {0}", sessionId);
        context.error(Utils.ERROR_USER_ATTRIBUTES_NOT_UNSET + ": " + unsetAttributesChecked.get());
        List<FormMessage> errors = new ArrayList<>();
        errors.add(new FormMessage(null, Utils.ERROR_USER_ATTRIBUTES_NOT_UNSET, sessionId));
        context.validationError(formData, errors);
        return;
      }

      // Verify the unique atrributes
      Optional<String> uniqueAttributesChecked =
          checkUniqueAttributes(context, uniqueAttributesList, formData);

      if (uniqueAttributesChecked.isPresent()) {
        String sessionId = context.getAuthenticationSession().getParentSession().getId();
        log.errorv(
            "validate(): Unique attributes present in more than one user. Error code: {0}",
            sessionId);
        context.error(
            Utils.ERROR_USER_ATTRIBUTES_NOT_UNIQUE + ": " + uniqueAttributesChecked.get());
        List<FormMessage> errors = new ArrayList<>();
        errors.add(new FormMessage(null, Utils.ERROR_USER_ATTRIBUTES_NOT_UNSET, sessionId));
        context.validationError(formData, errors);
      }
    }

    List<FormMessage> errors = new ArrayList<>();
    context.getEvent().detail(Details.REGISTER_METHOD, "form");
    if (passwordRequired
        && Validation.isBlank(formData.getFirst(RegistrationPage.FIELD_PASSWORD))) {
      errors.add(new FormMessage(RegistrationPage.FIELD_PASSWORD, Messages.MISSING_PASSWORD));
    } else if (passwordRequired
        && !formData
            .getFirst(RegistrationPage.FIELD_PASSWORD)
            .equals(formData.getFirst(RegistrationPage.FIELD_PASSWORD_CONFIRM))) {
      context.error(PASSWORD_NOT_MATCHED);
      errors.add(
          new FormMessage(
              RegistrationPage.FIELD_PASSWORD_CONFIRM, Messages.INVALID_PASSWORD_CONFIRM));
    }
    if (passwordRequired && formData.getFirst(RegistrationPage.FIELD_PASSWORD) != null) {
      PolicyError err =
          context
              .getSession()
              .getProvider(PasswordPolicyManagerProvider.class)
              .validate(
                  context.getRealm().isRegistrationEmailAsUsername()
                      ? formData.getFirst(RegistrationPage.FIELD_EMAIL)
                      : formData.getFirst(RegistrationPage.FIELD_USERNAME),
                  formData.getFirst(RegistrationPage.FIELD_PASSWORD));
      if (err != null)
        errors.add(
            new FormMessage(
                RegistrationPage.FIELD_PASSWORD, err.getMessage(), err.getParameters()));
    }

    // Check for confirm values
    for (Entry<String, List<String>> entry : formData.entrySet()) {
      log.infov("validate: checking {0} for confirm", entry.getKey());

      if (entry.getKey().endsWith("-confirm")
          && !entry.getKey().equals(RegistrationPage.FIELD_PASSWORD_CONFIRM)) {
        log.info("validate: confirm found");
        String confirmKey = entry.getKey();
        String confirmValue = entry.getValue().stream().findFirst().orElse(null);

        String key = confirmKey.substring(0, confirmKey.indexOf("-confirm"));
        String value = formData.getFirst(key);
        if (!value.equals(confirmValue)) {
          log.errorv(
              "validate: confirm value invalid key:{0} values {1} != {2}",
              key, value, confirmValue);
          context.error(INVALID_INPUT);
          errors.add(new FormMessage(confirmKey, "invalidConfirmationValue"));
          context.validationError(formData, errors);
        }
      }
    }

    if (errors.size() > 0) {
      for (FormMessage formMessage : errors) {
        if (formMessage.getField() == RegistrationPage.FIELD_PASSWORD_CONFIRM) {
          context.error(PASSWORD_NOT_MATCHED);
        } else if (formMessage.getField() == RegistrationPage.FIELD_PASSWORD) {
          context.error(PASSWORD_NOT_STRONG + ": " + formMessage.getMessage());
        } else {
          context.error(Errors.INVALID_REGISTRATION);
        }
      }
      formData.remove(RegistrationPage.FIELD_PASSWORD);
      formData.remove(RegistrationPage.FIELD_PASSWORD_CONFIRM);
      context.validationError(formData, errors);
      return;
    }

    // log formMode variable:
    log.infov(
        "validate: formMode={0} vs FormMode.LOGIN.getValue()={1}",
        formMode, FormMode.LOGIN.getValue());
    if (formMode.equals(FormMode.LOGIN.getValue())) {
      log.info("validate: setting authenticated user " + user.getUsername());
      context.getAuthenticationSession().setAuthenticatedUser(user);
    } else {
      log.info("validate: formMode is different!");
    }

    log.info("validate: success");
    context.success();
  }

  private String getAnnotationValueFromProfile(
      UserProfile profile, String attribute, String annotation) {
    if (profile == null || attribute == null || annotation == null) {
      return null;
    }

    Attributes attributes = profile.getAttributes();

    if (attributes == null) {
      return null;
    }

    AttributeMetadata metadata = attributes.getMetadata(attribute);

    if (metadata == null) {
      return null;
    }

    Map<String, Object> annotations = metadata.getAnnotations();

    if (annotations == null) {
      return null;
    }

    Object value = annotations.get(annotation);

    if (value instanceof String) {
      return (String) value;
    }

    return null;
  }

  private Optional<String> checkUniqueAttributes(
      ValidationContext context, List<String> attributes, MultivaluedMap<String, String> formData) {
    log.info("lookupUserByFormData(): checkUniqueAttributes start" + attributes);
    KeycloakSession session = context.getSession();
    RealmModel realm = context.getRealm();
    for (String attribute : attributes) {
      String value = formData.getFirst(attribute);
      log.infov(
          "lookupUserByFormData(): checkUniqueAttributes attribute {0} with value {1}",
          attribute, value);
      if (value != null && !value.isBlank()) {
        Stream<UserModel> currentStream =
            session
                .users()
                .searchForUserStream(realm, Collections.singletonMap(attribute, value.trim()));

        // Invalid if there's more than one user with specified attributes.
        if (currentStream.count() > 1) {
          String formattedErrorMessage =
              String.format(
                  "Unique attribute %s with value=%s present in more than one user",
                  attribute, value);
          log.infov(
              "lookupUserByFormData(): checkUniqueAttributes attribute {0} with value {0} present in other users",
              attribute, value);
          return Optional.of(formattedErrorMessage);
        }
      }
    }
    log.info("checkUniqueAttributes(): success");
    return Optional.empty();
  }

  @Override
  public void buildPage(FormContext context, LoginFormsProvider form) {
    // Retrieve the configuration
    AuthenticatorConfigModel config = context.getAuthenticatorConfig();
    Map<String, String> configMap = config.getConfig();
    final String formMode = configMap.get(FORM_MODE);
    final boolean passwordRequired =
        Boolean.parseBoolean(Optional.ofNullable(configMap.get(PASSWORD_REQUIRED)).orElse("true"));

    // When operating in LOGIN mode, we must manually construct and override the
    // form action URL to ensure it points back to this FormAction for validation.
    if (FormMode.LOGIN.name().equals(formMode)) {
        AuthenticationSessionModel authSession = context.getAuthenticationSession();

        // 1. Get the base URI builder from the current request context.
        // This ensures we use the correct hostname, port, and context path.
        UriBuilder builder = context.getUriInfo().getBaseUriBuilder()
                .path("realms").path(context.getRealm().getName())
                .path("login-actions/authenticate");

        // 2. Add the necessary query parameters from the authentication session
        // to route the request correctly back to this specific execution.
        builder.queryParam("session_code", authSession.getParentSession().getId());
        builder.queryParam("execution", context.getExecution().getId());
        builder.queryParam("client_id", authSession.getClient().getClientId());
        builder.queryParam("tab_id", authSession.getTabId());

        // 3. Build the final URI.
        URI actionUrl = builder.build();

        // 4. Set the fully-qualified action URI on the form provider. This will
        // override the default and be used for the form's "action" attribute.
        form.setActionUri(actionUrl);
    }
    // In REGISTRATION mode, the default behavior is correct, so no override is needed.

    form.setAttribute("passwordRequired", passwordRequired);
    form.setAttribute("formMode", formMode);
    log.infov("buildPage(): formMode = {0}", formMode);
    checkNotOtherUserAuthenticating(context);
  }

  @Override
  public void success(FormContext context) {
    log.info("DeferredRegistrationUserCreation: success");
    context.getEvent().success();

    // Retrieve the configuration
    AuthenticatorConfigModel config = context.getAuthenticatorConfig();
    Map<String, String> configMap = config.getConfig();
    final String formMode = configMap.get(FORM_MODE);

    if (!formMode.equals(FormMode.LOGIN.getValue())) {
      checkNotOtherUserAuthenticating(context);
    }

    // Extract the attributes to search and update from the configuration
    String searchAttributes = configMap.get(SEARCH_ATTRIBUTES);

    // Parse attributes lists
    List<String> searchAttributesList = parseAttributesList(searchAttributes);

    // Following successful filling of the form, we store the required user
    // information in the authentication session notes. This stored
    // information is then retrieved at a later time to create the user
    // account.
    Utils.storeUserDataInAuthSessionNotes(context, searchAttributesList);
  }

  private void checkNotOtherUserAuthenticating(FormContext context) {
    if (context.getUser() != null) {
      // the user probably did some back navigation in the browser,
      // hitting this page in a strange state
      context.getEvent().detail(Details.EXISTING_USER, context.getUser().getUsername());
      throw new AuthenticationFlowException(
          AuthenticationFlowError.GENERIC_AUTHENTICATION_ERROR,
          Errors.DIFFERENT_USER_AUTHENTICATING,
          Messages.EXPIRED_ACTION);
    }
  }

  @Override
  public boolean requiresUser() {
    return false;
  }

  @Override
  public boolean configuredFor(KeycloakSession session, RealmModel realm, UserModel user) {
    return true;
  }

  @Override
  public void setRequiredActions(KeycloakSession session, RealmModel realm, UserModel user) {}

  @Override
  public boolean isUserSetupAllowed() {
    return false;
  }

  @Override
  public void close() {}

  @Override
  public String getDisplayType() {
    return "Deferred Registration User Profile Creation";
  }

  @Override
  public String getReferenceCategory() {
    return null;
  }

  @Override
  public boolean isConfigurable() {
    return true;
  }

  private static AuthenticationExecutionModel.Requirement[] REQUIREMENT_CHOICES = {
    AuthenticationExecutionModel.Requirement.REQUIRED,
    AuthenticationExecutionModel.Requirement.DISABLED
  };

  @Override
  public AuthenticationExecutionModel.Requirement[] getRequirementChoices() {
    return REQUIREMENT_CHOICES;
  }

  @Override
  public FormAction create(KeycloakSession session) {
    return this;
  }

  @Override
  public void init(Config.Scope config) {}

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  private MultivaluedMap<String, String> normalizeFormParameters(
      MultivaluedMap<String, String> formParams) {
    MultivaluedHashMap<String, String> copy = new MultivaluedHashMap<>(formParams);

    // Remove "password" and "password-confirm" to avoid leaking them in the
    // user-profile data
    copy.remove(RegistrationPage.FIELD_PASSWORD);
    copy.remove(RegistrationPage.FIELD_PASSWORD_CONFIRM);

    return copy;
  }

  /**
   * Get user profile instance for current HTTP request (KeycloakSession) and for given context.
   * This assumes that there is single user registered within HTTP request, which is always the case
   * in Keycloak
   */
  public UserProfile getOrCreateUserProfile(
      FormContext formContext, MultivaluedMap<String, String> formData) {
    KeycloakSession session = formContext.getSession();
    UserProfile profile = (UserProfile) session.getAttribute("UP_REGISTER");
    if (profile == null) {
      formData = normalizeFormParameters(formData);
      UserProfileProvider profileProvider = session.getProvider(UserProfileProvider.class);
      profile = profileProvider.create(UserProfileContext.REGISTRATION, formData);
      session.setAttribute("UP_REGISTER", profile);
    }
    return profile;
  }

  private List<String> parseAttributesList(String attributes) {
    if (attributes == null || attributes.trim().isEmpty()) {
      return Collections.emptyList();
    }
    return List.of(attributes.split(","));
  }

  private Optional<String> checkUnsetAttributes(UserModel user, List<String> attributes) {
    Map<String, List<String>> userAttributes = user.getAttributes();
    for (String attributeName : attributes) {
      if (userAttributes.containsKey(attributeName)
          && userAttributes.get(attributeName) != null
          && userAttributes.get(attributeName).size() > 0
          && userAttributes.get(attributeName).get(0) != null
          && !userAttributes.get(attributeName).get(0).isBlank()) {
        String formattedErrorMessage =
            "User has attribute "
                + attributeName
                + " with value="
                + userAttributes.get(attributeName)
                + " but it should be unset";
        log.info(formattedErrorMessage);
        return Optional.of(formattedErrorMessage);
      }
    }
    return Optional.empty();
  }

  private boolean checkMissingFields(ValidationContext context, List<FormMessage> errors) {
    List<String> missingFields = new ArrayList<>();
    for (FormMessage error : errors) {
      if (error.getMessage().equals(MISSING_FIELDS_ERROR)) {
        missingFields.add(error.getField());
      }
    }
    if (missingFields.isEmpty()) {
      return false;
    }
    log.info("checkMissingFields(): missingFields = " + missingFields);
    String missingFieldsErrorMessage = MISSING_FIELDS + ": " + String.join(", ", missingFields);
    context.error(missingFieldsErrorMessage);
    return true;
  }

  private void buildEventDetails(
      MultivaluedMap<String, String> formData, ValidationContext context, UserModel user) {
    formData = normalizeFormParameters(formData);
    formData.forEach(
        (key, value) -> {
          if (value != null) {
            context.getEvent().detail(key, value);
          }
        });
    if (user != null) {
      context.getEvent().user(user.getId());
      context.getEvent().detail("user_attributes", Utils.getUserAttributesString(user));
    }
    context.getEvent().detail(Utils.AUTHENTICATOR_CLASS_NAME, this.getClass().getSimpleName());
  }
}
