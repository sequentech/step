// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import java.lang.reflect.Method;
import java.util.List;
import java.util.Map;
import java.util.Set;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.FormContext;
import org.keycloak.authentication.forms.RegistrationPage;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.http.HttpRequest;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.UserModel;
import org.keycloak.protocol.oidc.endpoints.AuthorizationEndpoint;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.userprofile.AttributeMetadata;
import org.keycloak.userprofile.Attributes;
import org.keycloak.userprofile.UserProfile;
import org.keycloak.userprofile.UserProfileContext;
import org.keycloak.userprofile.UserProfileProvider;
import org.keycloak.userprofile.ValidationException;
import org.keycloak.validate.ValidationError;
import org.mockito.ArgumentMatchers;

class DeferredRegistrationUserCreationTest {

  private static final String CUSTOM_HIDDEN_ATTRIBUTE = "customHidden";

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

  private static String clientNote(String attributeName) {
    return AuthorizationEndpoint.LOGIN_SESSION_NOTE_ADDITIONAL_REQ_PARAMS_PREFIX
        + LoginHintPrefill.HINT_PREFIX
        + attributeName;
  }
}
