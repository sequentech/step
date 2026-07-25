// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
// SPDX-FileCopyrightText: 2016 Red Hat, Inc. and/or its affiliates
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

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
import java.util.Set;
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
import org.keycloak.authentication.authenticators.browser.AbstractUsernameFormAuthenticator;
import org.keycloak.authentication.forms.RegistrationPage;
import org.keycloak.common.util.Time;
import org.keycloak.credential.hash.PasswordHashProvider;
import org.keycloak.events.Details;
import org.keycloak.events.Errors;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.PasswordPolicy;
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
  public static final String CREDENTIAL_INPUT_POLICY_REALM_ATTRIBUTE = "credential-input-policy";
  public static final String STRUCTURED_POLICY = "structured";
  public static final String STRUCTURED_CREDENTIAL_ERROR = "structuredCredentialError";
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
  public static final String HIDDEN_PROFILE_ATTRIBUTES = "hidden-profile-attributes";
  public static final String HIDDEN_PROFILE_ATTRIBUTES_DEFAULT = UserModel.LOCALE;

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
        new ProviderConfigProperty(
            HIDDEN_PROFILE_ATTRIBUTES,
            "Hidden Profile Attributes",
            "Comma-separated list of profile attributes to hide from the form and ignore if Keycloak marks them as required.",
            ProviderConfigProperty.STRING_TYPE,
            HIDDEN_PROFILE_ATTRIBUTES_DEFAULT),
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
    final boolean loginMode = FormMode.LOGIN.getValue().equals(formMode);
    final boolean passwordRequired =
        Boolean.parseBoolean(Optional.ofNullable(configMap.get(PASSWORD_REQUIRED)).orElse("true"));
    final boolean structuredCredentialLogin =
        isStructuredCredentialLogin(formMode, passwordRequired, context.getRealm().getAttributes());
    final String verifiedAttributeId =
        Optional.ofNullable(configMap.get(UNIQUE_ATTRIBUTES)).orElse(VERIFIED_DEFAULT_ID);

    if (loginMode) {
      context
          .getAuthenticationSession()
          .removeAuthNote(AbstractUsernameFormAuthenticator.ATTEMPTED_USERNAME);
    }

    // Parse attributes lists
    List<String> searchAttributesList = parseAttributesList(searchAttributes);
    List<String> unsetAttributesList = parseAttributesList(unsetAttributes);
    List<String> uniqueAttributesList = parseAttributesList(uniqueAttributes);

    // Get the form data
    MultivaluedMap<String, String> formData = context.getHttpRequest().getDecodedFormParameters();
    context.getEvent().detail(Details.REGISTER_METHOD, "form");
    Set<String> hiddenProfileAttributes = getHiddenProfileAttributes(configMap);
    UserProfile profile = getOrCreateUserProfile(context, formData, hiddenProfileAttributes);

    UserModel user = null;
    if (!searchAttributesList.isEmpty()) {
      user = Utils.lookupUserByFormData(context, searchAttributesList, formData);
    }
    buildEventDetails(formData, context, user, hiddenProfileAttributes);
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
        if (loginMode && passwordRequired) {
          performDummyHash(context);
        }
        reportValidationError(context, formData, errors, structuredCredentialLogin);
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
                          // If an attribute is hidden ignore its missing field validation error.
                          && !isRequiredErrorForHiddenAttribute(error, hiddenProfileAttributes)
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
        if (loginMode && passwordRequired) {
          performDummyHash(context);
        }
        reportValidationError(context, formData, errors, structuredCredentialLogin);
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
        if (loginMode && passwordRequired) {
          performDummyHash(context);
        }
        context.error(Utils.ERROR_MESSAGE_USER_NOT_FOUND);
        List<FormMessage> errors = new ArrayList<>();
        errors.add(new FormMessage(null, Utils.ERROR_USER_NOT_FOUND, sessionId));
        reportValidationError(context, formData, errors, structuredCredentialLogin);
        return;
      }

      if (loginMode) {
        // Validate password in LOGIN mode
        if (passwordRequired) {
          context
              .getAuthenticationSession()
              .setAuthNote(
                  AbstractUsernameFormAuthenticator.ATTEMPTED_USERNAME, user.getUsername());
          if (!validatePasswordForLogin(context, user, formData, structuredCredentialLogin)) {
            return;
          }
          context
              .getAuthenticationSession()
              .removeAuthNote(AbstractUsernameFormAuthenticator.ATTEMPTED_USERNAME);

          // Check password expiration after successful password validation
          if (!checkPasswordExpiration(
              context, user, formData, configMap, structuredCredentialLogin)) {
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
        reportValidationError(context, formData, errors, structuredCredentialLogin);
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
        errors.add(new FormMessage(null, Utils.ERROR_USER_ATTRIBUTES_NOT_UNIQUE, sessionId));
        reportValidationError(context, formData, errors, structuredCredentialLogin);
        return;
      }
    }

    // Initialize a list to hold form validation errors.
    List<FormMessage> errors = new ArrayList<>();
    context.getEvent().detail(Details.REGISTER_METHOD, "form");

    // Validate password if it's required for the form.
    if (passwordRequired && shouldValidatePasswordCreationPolicy(formMode)) {
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
          reportValidationError(context, formData, errors, structuredCredentialLogin);
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
      reportValidationError(context, formData, errors, structuredCredentialLogin);
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

  static boolean isStructuredCredentialLogin(
      String formMode, boolean passwordRequired, Map<String, String> realmAttributes) {
    return FormMode.LOGIN.getValue().equals(formMode)
        && passwordRequired
        && realmAttributes != null
        && STRUCTURED_POLICY.equals(realmAttributes.get(CREDENTIAL_INPUT_POLICY_REALM_ATTRIBUTE));
  }

  static boolean shouldValidatePasswordCreationPolicy(String formMode) {
    return !FormMode.LOGIN.getValue().equals(formMode);
  }

  private void reportValidationError(
      ValidationContext context,
      MultivaluedMap<String, String> formData,
      List<FormMessage> errors,
      boolean structuredCredentialLogin) {
    removeCredentialValues(formData);

    if (!structuredCredentialLogin) {
      context.validationError(formData, errors);
      return;
    }

    context.excludeOtherErrors();
    context.validationError(
        formData,
        List.of(new FormMessage(RegistrationPage.FIELD_PASSWORD, STRUCTURED_CREDENTIAL_ERROR)));
  }

  static void removeCredentialValues(MultivaluedMap<String, String> formData) {
    formData.remove(RegistrationPage.FIELD_PASSWORD);
    formData.remove(RegistrationPage.FIELD_PASSWORD_CONFIRM);
  }

  /**
   * Performs the equivalent dummy password hash to Keycloak's username/password authenticator.
   *
   * <p>{@code AuthenticatorUtils.dummyHash} only accepts an {@code AuthenticationFlowContext}, so
   * deferred registration login needs the equivalent operation for its {@code ValidationContext}.
   * If a configured named provider is unavailable, the default provider supplies a best-effort
   * dummy hash rather than turning an authentication failure into a server error.
   */
  static void performDummyHash(ValidationContext context) {
    PasswordPolicy policy = context.getRealm().getPasswordPolicy();
    PasswordHashProvider provider;
    int iterations;
    if (policy != null && policy.getHashAlgorithm() != null) {
      provider =
          context.getSession().getProvider(PasswordHashProvider.class, policy.getHashAlgorithm());
      iterations = policy.getHashIterations();
      if (provider == null) {
        log.warnv(
            "Password hash provider {0} is unavailable; using the default provider for dummy hashing",
            policy.getHashAlgorithm());
        provider = context.getSession().getProvider(PasswordHashProvider.class);
        iterations = -1;
      }
    } else {
      provider = context.getSession().getProvider(PasswordHashProvider.class);
      iterations = policy == null ? -1 : policy.getHashIterations();
    }
    if (provider == null) {
      throw new IllegalStateException("No password hash provider is available for dummy hashing");
    }
    provider.encodedCredential("SlightlyLongerDummyPassword", iterations);
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
    form.setAttribute("hiddenProfileAttributes", getHiddenProfileAttributes(configMap));
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
      MultivaluedMap<String, String> formParams, Set<String> hiddenProfileAttributes) {
    MultivaluedHashMap<String, String> copy = new MultivaluedHashMap<>(formParams);

    // Remove "password" and "password-confirm" to avoid leaking them in the
    // user-profile data
    copy.remove(RegistrationPage.FIELD_PASSWORD);
    copy.remove(RegistrationPage.FIELD_PASSWORD_CONFIRM);
    hiddenProfileAttributes.forEach(copy::remove);

    return copy;
  }

  static boolean isRequiredErrorForHiddenAttribute(
      ValidationException.Error error, Set<String> hiddenProfileAttributes) {
    return hiddenProfileAttributes.contains(error.getAttribute())
        && MISSING_FIELDS_ERROR.equals(error.getMessage());
  }

  /**
   * Returns configured hidden profile attributes, defaulting to locale when the option is not set.
   *
   * @param configMap authenticator configuration values
   * @return hidden profile attribute names
   */
  static Set<String> getHiddenProfileAttributes(Map<String, String> configMap) {
    String hiddenProfileAttributes =
        Optional.ofNullable(configMap.get(HIDDEN_PROFILE_ATTRIBUTES))
            .orElse(HIDDEN_PROFILE_ATTRIBUTES_DEFAULT);
    return parseAttributesSet(hiddenProfileAttributes);
  }

  /**
   * Get user profile instance for current HTTP request (KeycloakSession) and for given context.
   * This assumes that there is single user registered within HTTP request, which is always the case
   * in Keycloak
   */
  public UserProfile getOrCreateUserProfile(
      FormContext formContext,
      MultivaluedMap<String, String> formData,
      Set<String> hiddenProfileAttributes) {
    KeycloakSession session = formContext.getSession();
    UserProfile profile = (UserProfile) session.getAttribute("UP_REGISTER");
    if (profile == null) {
      formData = normalizeFormParameters(formData, hiddenProfileAttributes);
      UserProfileProvider profileProvider = session.getProvider(UserProfileProvider.class);
      profile = profileProvider.create(UserProfileContext.REGISTRATION, formData);
      session.setAttribute("UP_REGISTER", profile);
    }
    return profile;
  }

  private List<String> parseAttributesList(String attributes) {
    return parseAttributes(attributes).collect(Collectors.toList());
  }

  /**
   * Parses a comma-separated attribute list into trimmed, non-empty attribute names.
   *
   * @param attributes comma-separated attribute names
   * @return parsed attribute names
   */
  private static Set<String> parseAttributesSet(String attributes) {
    return parseAttributes(attributes).collect(Collectors.toUnmodifiableSet());
  }

  private static Stream<String> parseAttributes(String attributes) {
    if (attributes == null || attributes.trim().isEmpty()) {
      return Stream.empty();
    }
    return Stream.of(attributes.split(","))
        .map(String::trim)
        .filter(attribute -> !attribute.isEmpty());
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
      MultivaluedMap<String, String> formData,
      ValidationContext context,
      UserModel user,
      Set<String> hiddenProfileAttributes) {
    formData = normalizeFormParameters(formData, hiddenProfileAttributes);
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
   * @param structuredCredentialLogin whether structured login errors must use the generic PIN
   *     message
   * @return true if password is valid, false otherwise
   */
  private boolean validatePasswordForLogin(
      ValidationContext context,
      UserModel user,
      MultivaluedMap<String, String> formData,
      boolean structuredCredentialLogin) {
    log.info("validatePasswordForLogin: start");

    String password = formData.getFirst(CredentialRepresentation.PASSWORD);

    if (!user.isEnabled()) {
      log.info("validatePasswordForLogin: user disabled");
      performDummyHash(context);
      context.getEvent().user(user);
      context.getEvent().error(Errors.USER_DISABLED);
      context.error(PASSWORD_NOT_MATCHED);
      reportValidationError(
          context,
          formData,
          List.of(new FormMessage(RegistrationPage.FIELD_PASSWORD, Messages.INVALID_PASSWORD)),
          structuredCredentialLogin);
      return false;
    }

    // Check for empty password
    if (password == null || password.isEmpty()) {
      log.info("validatePasswordForLogin: empty password");
      performDummyHash(context);
      return handleBadPassword(context, user, formData, true, structuredCredentialLogin);
    }

    // Check for brute force protection
    if (isDisabledByBruteForce(context, user, structuredCredentialLogin)) {
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
      return handleBadPassword(context, user, formData, false, structuredCredentialLogin);
    }
  }

  /**
   * Handles bad password scenarios with proper error reporting.
   *
   * @param context the validation context
   * @param user the user model
   * @param formData the form data
   * @param isEmptyPassword whether the password was empty
   * @param structuredCredentialLogin whether structured login errors must use the generic PIN
   *     message
   * @return always false
   */
  private boolean handleBadPassword(
      ValidationContext context,
      UserModel user,
      MultivaluedMap<String, String> formData,
      boolean isEmptyPassword,
      boolean structuredCredentialLogin) {
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

    reportValidationError(context, formData, errors, structuredCredentialLogin);
    return false;
  }

  /**
   * Checks if the user is disabled by brute force protection.
   *
   * <p>Note: We cannot directly use AuthenticatorUtils.getDisabledByBruteForceEventError() because
   * it requires AuthenticationFlowContext, but we have ValidationContext. These are separate
   * Keycloak interfaces with no conversion mechanism. Instead, we use Keycloak's
   * BruteForceProtector service which provides the same brute force detection logic through a
   * provider interface that works with our context.
   *
   * @param context the validation context
   * @param user the user model
   * @param structuredCredentialLogin whether structured login errors must use the generic PIN
   *     message
   * @return true if user is disabled by brute force, false otherwise
   */
  private boolean isDisabledByBruteForce(
      ValidationContext context, UserModel user, boolean structuredCredentialLogin) {
    RealmModel realm = context.getRealm();

    // Check if brute force protection is enabled
    if (!realm.isBruteForceProtected()) {
      return false;
    }

    // Use Keycloak's BruteForceProtector service to check if user is disabled
    // This delegates to Keycloak's built-in brute force detection logic
    KeycloakSession session = context.getSession();
    org.keycloak.services.managers.BruteForceProtector protector =
        session.getProvider(org.keycloak.services.managers.BruteForceProtector.class);

    if (protector == null) {
      log.warn("BruteForceProtector provider not available, skipping brute force check");
      return false;
    }

    // Permanent disablement is handled before password validation. Check the
    // brute-force protector for temporary disablement here.
    boolean isDisabled = protector.isTemporarilyDisabled(session, realm, user);

    if (isDisabled) {
      log.infov(
          "isDisabledByBruteForce: user {0} is disabled by brute force protection",
          user.getUsername());
      performDummyHash(context);
      context.getEvent().user(user);
      context.getEvent().error(Errors.USER_TEMPORARILY_DISABLED);

      List<FormMessage> errors = new ArrayList<>();
      errors.add(new FormMessage(null, Messages.INVALID_USER));
      context.error(Messages.INVALID_USER);

      MultivaluedMap<String, String> formData = context.getHttpRequest().getDecodedFormParameters();

      reportValidationError(context, formData, errors, structuredCredentialLogin);
      return true;
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
   * @param structuredCredentialLogin whether structured login errors must use the generic PIN
   *     message
   * @return true if password is not expired or expiration is not configured, false if expired
   */
  private boolean checkPasswordExpiration(
      ValidationContext context,
      UserModel user,
      MultivaluedMap<String, String> formData,
      Map<String, String> configMap,
      boolean structuredCredentialLogin) {
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
        reportValidationError(context, formData, errors, structuredCredentialLogin);
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
