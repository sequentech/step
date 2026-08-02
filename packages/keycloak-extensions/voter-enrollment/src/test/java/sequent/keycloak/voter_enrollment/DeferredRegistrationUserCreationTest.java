// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyList;
import static org.mockito.ArgumentMatchers.argThat;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.inOrder;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.mockStatic;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.Response;
import java.lang.reflect.Method;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.stream.Stream;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowException;
import org.keycloak.authentication.FormContext;
import org.keycloak.authentication.ValidationContext;
import org.keycloak.authentication.authenticators.browser.AbstractUsernameFormAuthenticator;
import org.keycloak.authentication.forms.RegistrationPage;
import org.keycloak.credential.CredentialInput;
import org.keycloak.credential.hash.PasswordHashProvider;
import org.keycloak.events.EventBuilder;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.http.HttpRequest;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.PasswordPolicy;
import org.keycloak.models.RealmModel;
import org.keycloak.models.SubjectCredentialManager;
import org.keycloak.models.UserModel;
import org.keycloak.models.UserProvider;
import org.keycloak.protocol.oidc.endpoints.AuthorizationEndpoint;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.services.messages.Messages;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.sessions.RootAuthenticationSessionModel;
import org.keycloak.userprofile.AttributeMetadata;
import org.keycloak.userprofile.Attributes;
import org.keycloak.userprofile.UserProfile;
import org.keycloak.userprofile.UserProfileContext;
import org.keycloak.userprofile.UserProfileProvider;
import org.keycloak.userprofile.ValidationException;
import org.keycloak.validate.ValidationError;
import org.mockito.ArgumentMatchers;
import org.mockito.InOrder;
import org.mockito.MockedStatic;

class DeferredRegistrationUserCreationTest {

  private static final String CUSTOM_HIDDEN_ATTRIBUTE = "customHidden";

  @Test
  void structuredCredentialIsEnabledOnlyForLoginModeAndRealmPolicy() {
    assertTrue(
        DeferredRegistrationUserCreation.isStructuredCredentialLogin(
            DeferredRegistrationUserCreation.FormMode.LOGIN.getValue(),
            true,
            Map.of(
                DeferredRegistrationUserCreation.CREDENTIAL_INPUT_POLICY_REALM_ATTRIBUTE,
                DeferredRegistrationUserCreation.STRUCTURED_POLICY)));
    assertFalse(
        DeferredRegistrationUserCreation.isStructuredCredentialLogin(
            DeferredRegistrationUserCreation.FormMode.REGISTRATION.getValue(),
            true,
            Map.of(
                DeferredRegistrationUserCreation.CREDENTIAL_INPUT_POLICY_REALM_ATTRIBUTE,
                DeferredRegistrationUserCreation.STRUCTURED_POLICY)));
    assertFalse(
        DeferredRegistrationUserCreation.isStructuredCredentialLogin(
            DeferredRegistrationUserCreation.FormMode.LOGIN.getValue(),
            true,
            Map.of(
                DeferredRegistrationUserCreation.CREDENTIAL_INPUT_POLICY_REALM_ATTRIBUTE,
                "standard")));
    assertFalse(
        DeferredRegistrationUserCreation.isStructuredCredentialLogin(
            DeferredRegistrationUserCreation.FormMode.LOGIN.getValue(), true, Map.of()));
    assertFalse(
        DeferredRegistrationUserCreation.isStructuredCredentialLogin(
            DeferredRegistrationUserCreation.FormMode.LOGIN.getValue(), true, null));
    assertFalse(
        DeferredRegistrationUserCreation.isStructuredCredentialLogin(
            DeferredRegistrationUserCreation.FormMode.LOGIN.getValue(),
            false,
            Map.of(
                DeferredRegistrationUserCreation.CREDENTIAL_INPUT_POLICY_REALM_ATTRIBUTE,
                DeferredRegistrationUserCreation.STRUCTURED_POLICY)));
  }

  @Test
  void passwordCreationPolicyIsNotAppliedInLoginMode() {
    assertFalse(
        DeferredRegistrationUserCreation.shouldValidatePasswordCreationPolicy(
            DeferredRegistrationUserCreation.FormMode.LOGIN.getValue()));
    assertTrue(
        DeferredRegistrationUserCreation.shouldValidatePasswordCreationPolicy(
            DeferredRegistrationUserCreation.FormMode.REGISTRATION.getValue()));
  }

  @Test
  void dummyHashUsesTheRealmPasswordPolicy() {
    ValidationContext context = mock(ValidationContext.class);
    KeycloakSession session = mock(KeycloakSession.class);
    RealmModel realm = mock(RealmModel.class);
    PasswordPolicy policy = mock(PasswordPolicy.class);
    PasswordHashProvider provider = mock(PasswordHashProvider.class);
    when(context.getSession()).thenReturn(session);
    when(context.getRealm()).thenReturn(realm);
    when(realm.getPasswordPolicy()).thenReturn(policy);
    when(policy.getHashAlgorithm()).thenReturn("pbkdf2-sha256");
    when(policy.getHashIterations()).thenReturn(27500);
    when(session.getProvider(PasswordHashProvider.class, "pbkdf2-sha256")).thenReturn(provider);

    DeferredRegistrationUserCreation.performDummyHash(context);

    verify(provider).encodedCredential("SlightlyLongerDummyPassword", 27500);
  }

  @Test
  void dummyHashUsesDefaultProviderWithoutAPasswordPolicy() {
    ValidationContext context = mock(ValidationContext.class);
    KeycloakSession session = mock(KeycloakSession.class);
    RealmModel realm = mock(RealmModel.class);
    PasswordHashProvider provider = mock(PasswordHashProvider.class);
    when(context.getSession()).thenReturn(session);
    when(context.getRealm()).thenReturn(realm);
    when(realm.getPasswordPolicy()).thenReturn(null);
    when(session.getProvider(PasswordHashProvider.class)).thenReturn(provider);

    DeferredRegistrationUserCreation.performDummyHash(context);

    verify(provider).encodedCredential("SlightlyLongerDummyPassword", -1);
  }

  @Test
  void dummyHashFallsBackWhenTheConfiguredProviderIsUnavailable() {
    ValidationContext context = mock(ValidationContext.class);
    KeycloakSession session = mock(KeycloakSession.class);
    RealmModel realm = mock(RealmModel.class);
    PasswordPolicy policy = mock(PasswordPolicy.class);
    PasswordHashProvider provider = mock(PasswordHashProvider.class);
    when(context.getSession()).thenReturn(session);
    when(context.getRealm()).thenReturn(realm);
    when(realm.getPasswordPolicy()).thenReturn(policy);
    when(policy.getHashAlgorithm()).thenReturn("missing-provider");
    when(policy.getHashIterations()).thenReturn(12345);
    when(session.getProvider(PasswordHashProvider.class, "missing-provider")).thenReturn(null);
    when(session.getProvider(PasswordHashProvider.class)).thenReturn(provider);

    DeferredRegistrationUserCreation.performDummyHash(context);

    verify(session).getProvider(PasswordHashProvider.class, "missing-provider");
    verify(provider).encodedCredential("SlightlyLongerDummyPassword", -1);
  }

  @Test
  void dummyHashUsesPolicyIterationsWithTheDefaultProvider() {
    ValidationContext context = mock(ValidationContext.class);
    KeycloakSession session = mock(KeycloakSession.class);
    RealmModel realm = mock(RealmModel.class);
    PasswordPolicy policy = mock(PasswordPolicy.class);
    PasswordHashProvider provider = mock(PasswordHashProvider.class);
    when(context.getSession()).thenReturn(session);
    when(context.getRealm()).thenReturn(realm);
    when(realm.getPasswordPolicy()).thenReturn(policy);
    when(policy.getHashAlgorithm()).thenReturn(null);
    when(policy.getHashIterations()).thenReturn(31000);
    when(session.getProvider(PasswordHashProvider.class)).thenReturn(provider);

    DeferredRegistrationUserCreation.performDummyHash(context);

    verify(provider).encodedCredential("SlightlyLongerDummyPassword", 31000);
  }

  @Test
  void dummyHashFailsExplicitlyWhenNoProviderIsAvailable() {
    ValidationContext context = mock(ValidationContext.class);
    KeycloakSession session = mock(KeycloakSession.class);
    RealmModel realm = mock(RealmModel.class);
    PasswordPolicy policy = mock(PasswordPolicy.class);
    when(context.getSession()).thenReturn(session);
    when(context.getRealm()).thenReturn(realm);
    when(realm.getPasswordPolicy()).thenReturn(policy);
    when(policy.getHashAlgorithm()).thenReturn("missing-provider");
    when(session.getProvider(PasswordHashProvider.class, "missing-provider")).thenReturn(null);
    when(session.getProvider(PasswordHashProvider.class)).thenReturn(null);

    IllegalStateException error =
        assertThrows(
            IllegalStateException.class,
            () -> DeferredRegistrationUserCreation.performDummyHash(context));

    assertEquals("No password hash provider is available for dummy hashing", error.getMessage());
  }

  @Test
  void loginAttemptStateIsResetAndEarlyProfileErrorsUseDummyHash() {
    ValidationContext context = mock(ValidationContext.class);
    AuthenticatorConfigModel config = mock(AuthenticatorConfigModel.class);
    KeycloakSession session = mock(KeycloakSession.class);
    RealmModel realm = mock(RealmModel.class);
    HttpRequest request = mock(HttpRequest.class);
    EventBuilder event = mock(EventBuilder.class);
    AuthenticationSessionModel authenticationSession = mock(AuthenticationSessionModel.class);
    UserProfile profile = mock(UserProfile.class);
    Attributes attributes = mock(Attributes.class);
    UserModel user = mock(UserModel.class);
    PasswordHashProvider provider = mock(PasswordHashProvider.class);
    MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
    formData.add(UserModel.USERNAME, "voter");
    formData.add(RegistrationPage.FIELD_PASSWORD, "12345678");

    when(context.getAuthenticatorConfig()).thenReturn(config);
    when(config.getConfig())
        .thenReturn(
            Map.of(
                DeferredRegistrationUserCreation.SEARCH_ATTRIBUTES,
                UserModel.USERNAME,
                DeferredRegistrationUserCreation.FORM_MODE,
                DeferredRegistrationUserCreation.FormMode.LOGIN.getValue(),
                DeferredRegistrationUserCreation.PASSWORD_REQUIRED,
                "true"));
    when(context.getSession()).thenReturn(session);
    when(context.getRealm()).thenReturn(realm);
    when(realm.getAttributes()).thenReturn(Map.of());
    when(realm.isRegistrationEmailAsUsername()).thenReturn(false);
    when(realm.getPasswordPolicy()).thenReturn(null);
    when(session.getProvider(PasswordHashProvider.class)).thenReturn(provider);
    when(context.getHttpRequest()).thenReturn(request);
    when(request.getDecodedFormParameters()).thenReturn(formData);
    when(context.getEvent()).thenReturn(event);
    when(context.getAuthenticationSession()).thenReturn(authenticationSession);
    when(session.getAttribute("UP_REGISTER")).thenReturn(profile);
    when(profile.getAttributes()).thenReturn(attributes);
    when(attributes.getFirst(UserModel.EMAIL)).thenReturn("invalid@example.com");

    try (MockedStatic<Utils> utils = mockStatic(Utils.class)) {
      utils
          .when(() -> Utils.lookupUserByFormData(context, List.of(UserModel.USERNAME), formData))
          .thenReturn(user);

      DeferredRegistrationUserCreation action = new DeferredRegistrationUserCreation();
      action.validate(context);
      action.validate(context);
    }

    verify(authenticationSession, times(2))
        .removeAuthNote(AbstractUsernameFormAuthenticator.ATTEMPTED_USERNAME);
    verify(authenticationSession, never())
        .setAuthNote(eq(AbstractUsernameFormAuthenticator.ATTEMPTED_USERNAME), any());
    verify(provider, times(2)).encodedCredential("SlightlyLongerDummyPassword", -1);
  }

  @Test
  void failedLoginCredentialRetainsAttemptedUsernameForBruteForceAccounting() {
    ValidationContext context = mock(ValidationContext.class);
    AuthenticatorConfigModel config = mock(AuthenticatorConfigModel.class);
    KeycloakSession session = mock(KeycloakSession.class);
    RealmModel realm = mock(RealmModel.class);
    HttpRequest request = mock(HttpRequest.class);
    EventBuilder event = mock(EventBuilder.class);
    AuthenticationSessionModel authenticationSession = mock(AuthenticationSessionModel.class);
    UserProfile profile = mock(UserProfile.class);
    Attributes attributes = mock(Attributes.class);
    UserModel user = mock(UserModel.class);
    SubjectCredentialManager credentialManager = mock(SubjectCredentialManager.class);
    MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
    formData.add(UserModel.USERNAME, "voter");
    formData.add(RegistrationPage.FIELD_PASSWORD, "wrong-pin");

    when(context.getAuthenticatorConfig()).thenReturn(config);
    when(config.getConfig())
        .thenReturn(
            Map.of(
                DeferredRegistrationUserCreation.SEARCH_ATTRIBUTES,
                UserModel.USERNAME,
                DeferredRegistrationUserCreation.FORM_MODE,
                DeferredRegistrationUserCreation.FormMode.LOGIN.getValue(),
                DeferredRegistrationUserCreation.PASSWORD_REQUIRED,
                "true"));
    when(context.getSession()).thenReturn(session);
    when(context.getRealm()).thenReturn(realm);
    when(realm.getAttributes()).thenReturn(Map.of());
    when(realm.isRegistrationEmailAsUsername()).thenReturn(false);
    when(realm.isBruteForceProtected()).thenReturn(false);
    when(context.getHttpRequest()).thenReturn(request);
    when(request.getDecodedFormParameters()).thenReturn(formData);
    when(context.getEvent()).thenReturn(event);
    when(context.getAuthenticationSession()).thenReturn(authenticationSession);
    when(session.getAttribute("UP_REGISTER")).thenReturn(profile);
    when(profile.getAttributes()).thenReturn(attributes);
    when(attributes.getFirst(UserModel.EMAIL)).thenReturn(null);
    when(user.isEnabled()).thenReturn(true);
    when(user.getUsername()).thenReturn("voter");
    when(user.credentialManager()).thenReturn(credentialManager);
    when(credentialManager.isValid(any(CredentialInput.class))).thenReturn(false);

    try (MockedStatic<Utils> utils = mockStatic(Utils.class)) {
      utils
          .when(() -> Utils.lookupUserByFormData(context, List.of(UserModel.USERNAME), formData))
          .thenReturn(user);

      new DeferredRegistrationUserCreation().validate(context);
    }

    verify(authenticationSession, times(1))
        .removeAuthNote(AbstractUsernameFormAuthenticator.ATTEMPTED_USERNAME);
    verify(authenticationSession)
        .setAuthNote(AbstractUsernameFormAuthenticator.ATTEMPTED_USERNAME, "voter");
    verify(context, never()).success();
  }

  @Test
  void correctLoginCredentialClearsAttemptStateBeforeLaterValidationError() {
    ValidationContext context = mock(ValidationContext.class);
    AuthenticatorConfigModel config = mock(AuthenticatorConfigModel.class);
    KeycloakSession session = mock(KeycloakSession.class);
    RealmModel realm = mock(RealmModel.class);
    HttpRequest request = mock(HttpRequest.class);
    EventBuilder event = mock(EventBuilder.class);
    AuthenticationSessionModel authenticationSession = mock(AuthenticationSessionModel.class);
    RootAuthenticationSessionModel rootSession = mock(RootAuthenticationSessionModel.class);
    UserProfile profile = mock(UserProfile.class);
    Attributes attributes = mock(Attributes.class);
    UserProvider users = mock(UserProvider.class);
    UserModel user = mock(UserModel.class);
    UserModel duplicate = mock(UserModel.class);
    SubjectCredentialManager credentialManager = mock(SubjectCredentialManager.class);
    MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
    formData.add(UserModel.USERNAME, "voter");
    formData.add("phone", "123");
    formData.add(RegistrationPage.FIELD_PASSWORD, "12345678");

    when(context.getAuthenticatorConfig()).thenReturn(config);
    when(config.getConfig())
        .thenReturn(
            Map.of(
                DeferredRegistrationUserCreation.SEARCH_ATTRIBUTES,
                UserModel.USERNAME,
                DeferredRegistrationUserCreation.UNIQUE_ATTRIBUTES,
                "phone",
                DeferredRegistrationUserCreation.FORM_MODE,
                DeferredRegistrationUserCreation.FormMode.LOGIN.getValue(),
                DeferredRegistrationUserCreation.PASSWORD_REQUIRED,
                "true"));
    when(context.getSession()).thenReturn(session);
    when(context.getRealm()).thenReturn(realm);
    when(realm.getAttributes()).thenReturn(Map.of());
    when(realm.isRegistrationEmailAsUsername()).thenReturn(false);
    when(context.getHttpRequest()).thenReturn(request);
    when(request.getDecodedFormParameters()).thenReturn(formData);
    when(context.getEvent()).thenReturn(event);
    when(session.getAttribute("UP_REGISTER")).thenReturn(profile);
    when(profile.getAttributes()).thenReturn(attributes);
    when(attributes.getFirst(UserModel.EMAIL)).thenReturn(null);
    when(user.isEnabled()).thenReturn(true);
    when(user.getUsername()).thenReturn("voter");
    when(user.credentialManager()).thenReturn(credentialManager);
    when(credentialManager.isValid(any(CredentialInput.class))).thenReturn(true);
    when(user.getFirstAttribute("phone")).thenReturn(null);
    when(user.getAttributes()).thenReturn(Map.of());
    when(context.getAuthenticationSession()).thenReturn(authenticationSession);
    when(authenticationSession.getParentSession()).thenReturn(rootSession);
    when(rootSession.getId()).thenReturn("session-id");
    when(session.users()).thenReturn(users);
    when(users.searchForUserStream(realm, Map.of("phone", "123")))
        .thenReturn(Stream.of(user, duplicate));

    try (MockedStatic<Utils> utils = mockStatic(Utils.class)) {
      utils
          .when(() -> Utils.lookupUserByFormData(context, List.of(UserModel.USERNAME), formData))
          .thenReturn(user);

      new DeferredRegistrationUserCreation().validate(context);
    }

    verify(context)
        .error(
            Utils.ERROR_USER_ATTRIBUTES_NOT_UNIQUE
                + ": Unique attribute phone with value=123 present in more than one user");
    verify(context).validationError(eq(formData), anyList());
    verify(context, never()).success();

    InOrder notes = inOrder(authenticationSession);
    notes
        .verify(authenticationSession)
        .removeAuthNote(AbstractUsernameFormAuthenticator.ATTEMPTED_USERNAME);
    notes
        .verify(authenticationSession)
        .setAuthNote(AbstractUsernameFormAuthenticator.ATTEMPTED_USERNAME, "voter");
    notes
        .verify(authenticationSession)
        .removeAuthNote(AbstractUsernameFormAuthenticator.ATTEMPTED_USERNAME);
  }

  @Test
  void disabledUserUsesTheSameStandardFieldErrorAsABadPassword() throws Exception {
    ValidationContext context = mock(ValidationContext.class);
    KeycloakSession session = mock(KeycloakSession.class);
    RealmModel realm = mock(RealmModel.class);
    EventBuilder event = mock(EventBuilder.class);
    UserModel user = mock(UserModel.class);
    PasswordHashProvider provider = mock(PasswordHashProvider.class);
    MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
    formData.add(RegistrationPage.FIELD_PASSWORD, "12345678");

    when(context.getSession()).thenReturn(session);
    when(context.getRealm()).thenReturn(realm);
    when(context.getEvent()).thenReturn(event);
    when(realm.getPasswordPolicy()).thenReturn(null);
    when(session.getProvider(PasswordHashProvider.class)).thenReturn(provider);
    when(user.isEnabled()).thenReturn(false);

    Method method =
        DeferredRegistrationUserCreation.class.getDeclaredMethod(
            "validatePasswordForLogin",
            ValidationContext.class,
            UserModel.class,
            MultivaluedMap.class,
            boolean.class);
    method.setAccessible(true);

    assertFalse(
        (boolean)
            method.invoke(new DeferredRegistrationUserCreation(), context, user, formData, false));

    verify(event).error(org.keycloak.events.Errors.USER_DISABLED);
    verify(context)
        .validationError(
            eq(formData),
            argThat(
                errors ->
                    errors.size() == 1
                        && RegistrationPage.FIELD_PASSWORD.equals(errors.get(0).getField())
                        && Messages.INVALID_PASSWORD.equals(errors.get(0).getMessage())));
  }

  @Test
  void normalizeFormParametersRemovesHiddenAndSensitiveFields() throws Exception {
    MultivaluedMap<String, String> formParams = new MultivaluedHashMap<>();
    formParams.add(RegistrationPage.FIELD_PASSWORD, "password");
    formParams.add(RegistrationPage.FIELD_PASSWORD_CONFIRM, "password");
    formParams.add(UserModel.LOCALE, "en");
    formParams.add(CUSTOM_HIDDEN_ATTRIBUTE, "hidden");
    formParams.add(UserModel.EMAIL, "voter@example.com");

    Method method =
        DeferredRegistrationUserCreation.class.getDeclaredMethod(
            "normalizeFormParameters", MultivaluedMap.class, Set.class);
    method.setAccessible(true);

    @SuppressWarnings("unchecked")
    MultivaluedMap<String, String> normalized =
        (MultivaluedMap<String, String>)
            method.invoke(
                new DeferredRegistrationUserCreation(),
                formParams,
                Set.of(UserModel.LOCALE, CUSTOM_HIDDEN_ATTRIBUTE));

    assertFalse(normalized.containsKey(RegistrationPage.FIELD_PASSWORD));
    assertFalse(normalized.containsKey(RegistrationPage.FIELD_PASSWORD_CONFIRM));
    assertFalse(normalized.containsKey(UserModel.LOCALE));
    assertFalse(normalized.containsKey(CUSTOM_HIDDEN_ATTRIBUTE));
    assertTrue(normalized.containsKey(UserModel.EMAIL));
  }

  @Test
  void removeCredentialValuesSanitizesValidationFormData() {
    MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
    formData.add(RegistrationPage.FIELD_USERNAME, "voter");
    formData.add(RegistrationPage.FIELD_PASSWORD, "12345678");
    formData.add(RegistrationPage.FIELD_PASSWORD_CONFIRM, "12345678");

    DeferredRegistrationUserCreation.removeCredentialValues(formData);

    assertTrue(formData.containsKey(RegistrationPage.FIELD_USERNAME));
    assertFalse(formData.containsKey(RegistrationPage.FIELD_PASSWORD));
    assertFalse(formData.containsKey(RegistrationPage.FIELD_PASSWORD_CONFIRM));
  }

  @Test
  void hiddenProfileAttributesDefaultToLocale() {
    assertEquals(
        Set.of(UserModel.LOCALE),
        DeferredRegistrationUserCreation.getHiddenProfileAttributes(Map.of()));
  }

  @Test
  void hiddenProfileAttributesCanBeConfigured() {
    assertEquals(
        Set.of(UserModel.LOCALE, CUSTOM_HIDDEN_ATTRIBUTE),
        DeferredRegistrationUserCreation.getHiddenProfileAttributes(
            Map.of(
                DeferredRegistrationUserCreation.HIDDEN_PROFILE_ATTRIBUTES,
                " locale, " + CUSTOM_HIDDEN_ATTRIBUTE + " ")));
  }

  @Test
  void isRequiredErrorForHiddenAttributeOnlyMatchesConfiguredHiddenAttributes() {
    ValidationException.Error localeRequiredError =
        new ValidationException.Error(
            new ValidationError(
                "required",
                UserModel.LOCALE,
                DeferredRegistrationUserCreation.MISSING_FIELDS_ERROR));
    ValidationException.Error localeInvalidError =
        new ValidationException.Error(
            new ValidationError(
                "validator", UserModel.LOCALE, ValidationError.MESSAGE_INVALID_VALUE));
    ValidationException.Error emailRequiredError =
        new ValidationException.Error(
            new ValidationError(
                "required",
                UserModel.EMAIL,
                DeferredRegistrationUserCreation.MISSING_FIELDS_ERROR));

    Set<String> hiddenProfileAttributes = Set.of(UserModel.LOCALE);

    assertTrue(
        DeferredRegistrationUserCreation.isRequiredErrorForHiddenAttribute(
            localeRequiredError, hiddenProfileAttributes));
    assertFalse(
        DeferredRegistrationUserCreation.isRequiredErrorForHiddenAttribute(
            localeInvalidError, hiddenProfileAttributes));
    assertFalse(
        DeferredRegistrationUserCreation.isRequiredErrorForHiddenAttribute(
            emailRequiredError, hiddenProfileAttributes));
  }

  @Test
  void prefillPolicyDefaultsToIgnore() {
    ProviderConfigProperty policy =
        new DeferredRegistrationUserCreation()
            .getConfigProperties().stream()
                .filter(
                    property ->
                        DeferredRegistrationUserCreation.PREFILL_PARAMETERS_POLICY.equals(
                            property.getName()))
                .findFirst()
                .orElseThrow();

    assertEquals(
        DeferredRegistrationUserCreation.PrefillPolicy.IGNORE.name(), policy.getDefaultValue());
    assertEquals(
        List.of(
            DeferredRegistrationUserCreation.PrefillPolicy.IGNORE.name(),
            DeferredRegistrationUserCreation.PrefillPolicy.ACCEPT.name()),
        policy.getOptions());
  }

  @Test
  void prefillPolicyMustBeAcceptedAndOnlyAppliesVisibleAttributesOnGet() {
    DeferredRegistrationUserCreation action = new DeferredRegistrationUserCreation();
    FormContext context = mock(FormContext.class);
    LoginFormsProvider form = mock(LoginFormsProvider.class);
    AuthenticatorConfigModel config = mock(AuthenticatorConfigModel.class);
    HttpRequest request = mock(HttpRequest.class);
    AuthenticationSessionModel authenticationSession = mock(AuthenticationSessionModel.class);
    KeycloakSession session = mock(KeycloakSession.class);
    UserProfileProvider profileProvider = mock(UserProfileProvider.class);
    UserProfile profile = mock(UserProfile.class);
    Attributes attributes = mock(Attributes.class);
    AttributeMetadata metadata = mock(AttributeMetadata.class);

    Map<String, String> configValues = new java.util.HashMap<>();
    configValues.put(DeferredRegistrationUserCreation.FORM_MODE, "REGISTRATION");
    when(context.getAuthenticatorConfig()).thenReturn(config);
    when(config.getConfig()).thenReturn(configValues);
    when(context.getHttpRequest()).thenReturn(request);
    when(request.getHttpMethod()).thenReturn("GET");
    when(context.getAuthenticationSession()).thenReturn(authenticationSession);
    when(authenticationSession.getClientNotes())
        .thenReturn(
            Map.of(
                clientNote("username"), "voter@example.com",
                clientNote(UserModel.LOCALE), "es"));
    when(context.getSession()).thenReturn(session);
    when(session.getProvider(UserProfileProvider.class)).thenReturn(profileProvider);
    when(profileProvider.create(
            eq(UserProfileContext.REGISTRATION), ArgumentMatchers.<Map<String, ?>>any()))
        .thenReturn(profile);
    when(profile.getAttributes()).thenReturn(attributes);
    when(attributes.getWritable())
        .thenReturn(
            Map.of("username", List.of("voter@example.com"), UserModel.LOCALE, List.of("es")));
    when(attributes.getUnmanagedAttributes()).thenReturn(Map.of());
    when(attributes.getMetadata("username")).thenReturn(metadata);
    when(attributes.getMetadata(UserModel.LOCALE)).thenReturn(metadata);

    action.buildPage(context, form);
    verify(form, never()).setFormData(any());

    configValues.put(
        DeferredRegistrationUserCreation.PREFILL_PARAMETERS_POLICY,
        DeferredRegistrationUserCreation.PrefillPolicy.ACCEPT.name());
    action.buildPage(context, form);
    MultivaluedMap<String, String> expectedFormData = new MultivaluedHashMap<>();
    expectedFormData.putSingle("username", "voter@example.com");
    verify(form).setFormData(eq(expectedFormData));

    when(request.getHttpMethod()).thenReturn("POST");
    action.buildPage(context, form);
    verify(form).setFormData(any());
  }

  @Test
  void deferredAcceptRejectsInvalidStoredHintsWhileIgnoreLeavesThemUntouched() {
    DeferredRegistrationUserCreation action = new DeferredRegistrationUserCreation();
    FormContext context = mock(FormContext.class);
    LoginFormsProvider form = mock(LoginFormsProvider.class);
    AuthenticatorConfigModel config = mock(AuthenticatorConfigModel.class);
    AuthenticationSessionModel authenticationSession = mock(AuthenticationSessionModel.class);

    Map<String, String> configValues = new java.util.HashMap<>();
    configValues.put(DeferredRegistrationUserCreation.FORM_MODE, "REGISTRATION");
    when(context.getAuthenticatorConfig()).thenReturn(config);
    when(config.getConfig()).thenReturn(configValues);
    when(context.getAuthenticationSession()).thenReturn(authenticationSession);
    when(authenticationSession.getClientNotes())
        .thenReturn(
            Map.of(
                clientNote("username"), "voter@example.com",
                clientNote("invalid field"), "value"));

    action.buildPage(context, form);
    verify(form, never()).setError(LoginHintAuthorizationRequestFilter.INVALID_REQUEST_DESCRIPTION);

    configValues.put(
        DeferredRegistrationUserCreation.PREFILL_PARAMETERS_POLICY,
        DeferredRegistrationUserCreation.PrefillPolicy.ACCEPT.name());
    Response response = Response.status(Response.Status.BAD_REQUEST).build();
    when(form.setError(LoginHintAuthorizationRequestFilter.INVALID_REQUEST_DESCRIPTION))
        .thenReturn(form);
    when(form.createErrorPage(Response.Status.BAD_REQUEST)).thenReturn(response);

    AuthenticationFlowException failure =
        assertThrows(AuthenticationFlowException.class, () -> action.buildPage(context, form));

    assertEquals(response, failure.getResponse());
  }

  private static String clientNote(String attributeName) {
    return AuthorizationEndpoint.LOGIN_SESSION_NOTE_ADDITIONAL_REQ_PARAMS_PREFIX
        + LoginHintPrefill.HINT_PREFIX
        + attributeName;
  }
}
