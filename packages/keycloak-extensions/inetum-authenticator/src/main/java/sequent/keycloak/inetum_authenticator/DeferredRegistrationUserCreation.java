// SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
// SPDX-FileCopyrightText: 2016 Red Hat, Inc. and/or its affiliates
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.inetum_authenticator;

import static java.util.Arrays.asList;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
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
import org.keycloak.common.util.Time;
import org.keycloak.events.Details;
import org.keycloak.events.Errors;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserCredentialModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.utils.FormMessage;
import org.keycloak.policy.PasswordPolicyManagerProvider;
import org.keycloak.policy.PolicyError;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.representations.idm.CredentialRepresentation;
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
  public static final String PASSWORD_EXPIRATION_USER_ATTRIBUTE =
      "password-expiration-user-attribute";
  public static final String PASSWORD_EXPIRATION_USER_ATTRIBUTE_DEFAULT =
      "sequent.read-only.expirationDate";

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
        new ProviderConfigProperty(
            PASSWORD_EXPIRATION_USER_ATTRIBUTE,
            "Password Expiration User Attribute",
            "User attribute to use for storing the Password Expiration Date. Should be read-only. If the attribute is set and the password has expired, login will fail.",
            ProviderConfigProperty.STRING_TYPE,
            PASSWORD_EXPIRATION_USER_ATTRIBUTE_DEFAULT),
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
        // Validate password in LOGIN mode
        if (passwordRequired) {
          if (!validatePasswordForLogin(context, user, formData)) {
            return;
          }

          // Check password expiration after successful password validation
          if (!checkPasswordExpiration(context, user, formData, configMap)) {
            return;
          }
        }
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

    // Initialize a list to hold form validation errors.
    List<FormMessage> errors = new ArrayList<>();
    context.getEvent().detail(Details.REGISTER_METHOD, "form");

    // Validate password if it's required for the form.
    if (passwordRequired) {
      String password = formData.getFirst(RegistrationPage.FIELD_PASSWORD);
      String passwordConfirm = formData.getFirst(RegistrationPage.FIELD_PASSWORD_CONFIRM);

      // Check if the password field is blank.
      if (Validation.isBlank(password)) {
        errors.add(new FormMessage(RegistrationPage.FIELD_PASSWORD, Messages.MISSING_PASSWORD));
      } else if (!formMode.equals(FormMode.LOGIN.getValue()) && !password.equals(passwordConfirm)) {
        // In registration mode, check if the password and confirmation match.
        context.error(PASSWORD_NOT_MATCHED);
        errors.add(
            new FormMessage(
                RegistrationPage.FIELD_PASSWORD_CONFIRM, Messages.INVALID_PASSWORD_CONFIRM));
      }

      // If a password is provided, validate it against the realm's password policy.
      if (password != null) {
        PolicyError err =
            context
                .getSession()
                .getProvider(PasswordPolicyManagerProvider.class)
                .validate(
                    context.getRealm().isRegistrationEmailAsUsername()
                        ? formData.getFirst(RegistrationPage.FIELD_EMAIL)
                        : formData.getFirst(RegistrationPage.FIELD_USERNAME),
                    password);
        if (err != null) {
          errors.add(
              new FormMessage(
                  RegistrationPage.FIELD_PASSWORD, err.getMessage(), err.getParameters()));
        }
      }
    }

    // Check for other confirmation fields (e.g., 'email-confirm').
    for (Entry<String, List<String>> entry : formData.entrySet()) {
      String formKey = entry.getKey();
      log.infov("validate: checking {0} for confirm", formKey);

      // Identify fields that are confirmation fields but not the password confirmation.
      if (formKey.endsWith("-confirm")
          && !formKey.equals(RegistrationPage.FIELD_PASSWORD_CONFIRM)) {
        log.info("validate: confirm found");
        String confirmValue = entry.getValue().stream().findFirst().orElse(null);

        // Derive the original field key from the confirmation key.
        String originalKey = formKey.substring(0, formKey.indexOf("-confirm"));
        String originalValue = formData.getFirst(originalKey);

        // Compare the original value with its confirmation.
        if (!originalValue.equals(confirmValue)) {
          log.errorv(
              "validate: confirm value invalid key:{0} values {1} != {2}",
              originalKey, originalValue, confirmValue);
          context.error(INVALID_INPUT);
          errors.add(new FormMessage(formKey, "invalidConfirmationValue"));
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
      if (user != null) {
        log.info("validate: setting authenticated user " + user.getUsername());
        context.getAuthenticationSession().setAuthenticatedUser(user);
        context.setUser(user);
      }
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

  /**
   * Validates the password for LOGIN mode with security considerations including: - Constant-time
   * password comparison via Keycloak's credential manager - Brute force detection - Proper error
   * handling
   *
   * @param context the validation context
   * @param user the user model
   * @param formData the form data containing the password
   * @return true if password is valid, false otherwise
   */
  private boolean validatePasswordForLogin(
      ValidationContext context, UserModel user, MultivaluedMap<String, String> formData) {
    log.info("validatePasswordForLogin: start");

    String password = formData.getFirst(CredentialRepresentation.PASSWORD);

    // Check for empty password
    if (password == null || password.isEmpty()) {
      log.info("validatePasswordForLogin: empty password");
      return handleBadPassword(context, user, formData, true);
    }

    // Check for brute force protection
    if (isDisabledByBruteForce(context, user)) {
      log.info("validatePasswordForLogin: user disabled by brute force");
      return false;
    }

    // Validate password using Keycloak's credential manager
    // This uses constant-time comparison internally for security
    if (user.credentialManager().isValid(UserCredentialModel.password(password))) {
      log.info("validatePasswordForLogin: password valid");
      return true;
    } else {
      log.info("validatePasswordForLogin: password invalid");
      return handleBadPassword(context, user, formData, false);
    }
  }

  /**
   * Handles bad password scenarios with proper error reporting.
   *
   * @param context the validation context
   * @param user the user model
   * @param formData the form data
   * @param isEmptyPassword whether the password was empty
   * @return always false
   */
  private boolean handleBadPassword(
      ValidationContext context,
      UserModel user,
      MultivaluedMap<String, String> formData,
      boolean isEmptyPassword) {
    log.info("handleBadPassword: isEmptyPassword=" + isEmptyPassword);

    context.getEvent().user(user);
    context.getEvent().error(Errors.INVALID_USER_CREDENTIALS);

    List<FormMessage> errors = new ArrayList<>();
    if (isEmptyPassword) {
      errors.add(new FormMessage(RegistrationPage.FIELD_PASSWORD, Messages.MISSING_PASSWORD));
      context.error(MISSING_FIELDS);
    } else {
      errors.add(new FormMessage(RegistrationPage.FIELD_PASSWORD, Messages.INVALID_PASSWORD));
      context.error(PASSWORD_NOT_MATCHED);
    }

    // Remove password from form data for security
    formData.remove(RegistrationPage.FIELD_PASSWORD);
    formData.remove(RegistrationPage.FIELD_PASSWORD_CONFIRM);

    context.validationError(formData, errors);
    return false;
  }

  /**
   * Checks if the user is disabled by brute force protection.
   *
   * <p>Note: ValidationContext doesn't provide direct access to AuthenticationFlowContext, so we
   * use the session's brute force protector directly. This provides the same security guarantees as
   * getDisabledByBruteForceEventError but works with ValidationContext.
   *
   * @param context the validation context
   * @param user the user model
   * @return true if user is disabled by brute force, false otherwise
   */
  private boolean isDisabledByBruteForce(ValidationContext context, UserModel user) {
    // Check if brute force protection is enabled
    if (!context.getRealm().isBruteForceProtected()) {
      return false;
    }

    // Check if user is temporarily disabled due to brute force
    KeycloakSession session = context.getSession();
    org.keycloak.models.UserLoginFailureModel loginFailure =
        session.loginFailures().getUserLoginFailure(context.getRealm(), user.getId());

    if (loginFailure != null) {
      int failureFactor = context.getRealm().getFailureFactor();
      int waitSeconds = context.getRealm().getWaitIncrementSeconds();
      int maxWaitSeconds = context.getRealm().getMaxFailureWaitSeconds();
      int maxDeltaTimeSeconds = context.getRealm().getMaxDeltaTimeSeconds();

      // Check if account is temporarily disabled
      if (loginFailure.getNumFailures() >= failureFactor) {
        long currentTime = System.currentTimeMillis();
        long lastFailure = loginFailure.getLastFailure();
        long deltaTime = currentTime - lastFailure;

        // Calculate wait time with exponential backoff
        int waitTime = waitSeconds;
        if (loginFailure.getNumFailures() > failureFactor) {
          int multiplier = loginFailure.getNumFailures() - failureFactor;
          waitTime = waitSeconds * (int) Math.pow(2, multiplier);
          if (waitTime > maxWaitSeconds) {
            waitTime = maxWaitSeconds;
          }
        }

        if (deltaTime < waitTime * 1000L) {
          log.infov(
              "isDisabledByBruteForce: user {0} temporarily disabled. Failures: {1}, Wait time: {2}s",
              user.getUsername(), loginFailure.getNumFailures(), waitTime);
          context.getEvent().user(user);
          context.getEvent().error(Errors.USER_TEMPORARILY_DISABLED);

          List<FormMessage> errors = new ArrayList<>();
          errors.add(new FormMessage(null, Messages.INVALID_USER));
          context.error(Messages.INVALID_USER);

          MultivaluedMap<String, String> formData =
              context.getHttpRequest().getDecodedFormParameters();
          formData.remove(RegistrationPage.FIELD_PASSWORD);
          formData.remove(RegistrationPage.FIELD_PASSWORD_CONFIRM);

          context.validationError(formData, errors);
          return true;
        }
      }

      // Check if account permanently disabled due to max delta time
      if (maxDeltaTimeSeconds > 0) {
        long currentTime = System.currentTimeMillis();
        long lastFailure = loginFailure.getLastFailure();
        if (loginFailure.getNumFailures() >= failureFactor
            && (currentTime - lastFailure) < maxDeltaTimeSeconds * 1000L) {
          log.infov(
              "isDisabledByBruteForce: user {0} disabled by max delta time", user.getUsername());
          context.getEvent().user(user);
          context.getEvent().error(Errors.USER_TEMPORARILY_DISABLED);

          List<FormMessage> errors = new ArrayList<>();
          errors.add(new FormMessage(null, Messages.INVALID_USER));
          context.error(Messages.INVALID_USER);

          MultivaluedMap<String, String> formData =
              context.getHttpRequest().getDecodedFormParameters();
          formData.remove(RegistrationPage.FIELD_PASSWORD);
          formData.remove(RegistrationPage.FIELD_PASSWORD_CONFIRM);

          context.validationError(formData, errors);
          return true;
        }
      }
    }

    return false;
  }

  /**
   * Checks if the user's password has expired based on the configured password expiration
   * attribute.
   *
   * @param context the validation context
   * @param user the user model
   * @param formData the form data
   * @param configMap the authenticator configuration map
   * @return true if password is not expired or expiration is not configured, false if expired
   */
  private boolean checkPasswordExpiration(
      ValidationContext context,
      UserModel user,
      MultivaluedMap<String, String> formData,
      Map<String, String> configMap) {
    log.info("checkPasswordExpiration: start");

    // Get the password expiration user attribute name from configuration
    String passwordExpirationUserAttribute =
        Optional.ofNullable(configMap.get(PASSWORD_EXPIRATION_USER_ATTRIBUTE))
            .orElse(PASSWORD_EXPIRATION_USER_ATTRIBUTE_DEFAULT);

    if (passwordExpirationUserAttribute == null) {
      // shouldn't happen since we have a fall-back attribute name
      log.info(
          "checkPasswordExpiration: password expiration user attribute configuration is null - return true");
      return true;
    }

    String passwordExpiration = user.getFirstAttribute(passwordExpirationUserAttribute);
    if (passwordExpiration == null) {
      // if password expiration is null it means the user doesn't have this
      // attribute set, and thus we can ignore and return true
      log.info("checkPasswordExpiration: password expiration not set - return true");
      return true;
    }

    try {
      int passwordExpirationInt = Integer.parseInt(passwordExpiration);
      int currentTime = Time.currentTime();

      if (currentTime > passwordExpirationInt) {
        // the user has an expired password
        log.infov(
            "checkPasswordExpiration: expired password, currentTime[{0}] > passwordExpirationInt[{1}]",
            currentTime, passwordExpirationInt);

        context.getEvent().user(user);
        context.getEvent().error(Errors.EXPIRED_CODE);

        List<FormMessage> errors = new ArrayList<>();
        errors.add(new FormMessage(RegistrationPage.FIELD_PASSWORD, Messages.INVALID_PASSWORD));
        context.error("Password has expired");

        // Remove password from form data for security
        formData.remove(RegistrationPage.FIELD_PASSWORD);
        formData.remove(RegistrationPage.FIELD_PASSWORD_CONFIRM);

        context.validationError(formData, errors);
        return false;
      }

      log.infov(
          "checkPasswordExpiration: password not expired, currentTime[{0}] <= passwordExpirationInt[{1}]",
          currentTime, passwordExpirationInt);
      return true;

    } catch (NumberFormatException e) {
      log.errorv(
          "checkPasswordExpiration: invalid password expiration format: {0}", passwordExpiration);
      // If the format is invalid, we'll allow the login to proceed
      // This is a graceful degradation rather than blocking the user
      return true;
    }
  }
}
