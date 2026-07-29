// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.verifyNoInteractions;
import static org.mockito.Mockito.when;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.FormActionFactory;
import org.keycloak.authentication.FormContext;
import org.keycloak.authentication.ValidationContext;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.http.HttpRequest;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.utils.FormMessage;
import org.keycloak.protocol.oidc.endpoints.AuthorizationEndpoint;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.userprofile.AttributeMetadata;
import org.keycloak.userprofile.Attributes;
import org.keycloak.userprofile.UserProfile;
import org.keycloak.userprofile.UserProfileContext;
import org.keycloak.userprofile.UserProfileProvider;
import org.mockito.ArgumentMatchers;

class LoginHintPrefillTest {

  @Test
  void extractsOnlyBoundedNamespacedClientNotes() {
    Map<String, String> clientNotes =
        Map.of(
            clientNote("username"),
            "voter@example.com",
            clientNote("dateOfBirth"),
            "2000-01-01",
            "state",
            "oidc-state");

    assertEquals(
        Map.of("username", "voter@example.com", "dateOfBirth", "2000-01-01"),
        LoginHintPrefill.extractHints(clientNotes));
  }

  @Test
  void rejectsAnInvalidHintSetWithoutReturningPartialData() {
    Map<String, String> tooManyHints = new HashMap<>();
    for (int index = 0; index <= LoginHintPrefill.MAX_HINT_COUNT; index++) {
      tooManyHints.put(clientNote("field" + index), "value" + index);
    }

    assertThrows(IllegalArgumentException.class, () -> LoginHintPrefill.extractHints(tooManyHints));
    assertThrows(
        IllegalArgumentException.class,
        () -> LoginHintPrefill.extractHints(Map.of(clientNote("first name"), "value")));
    assertThrows(
        IllegalArgumentException.class,
        () -> LoginHintPrefill.extractHints(Map.of(clientNote("username"), " ")));
  }

  @Test
  void keepsOnlyExplicitWritableManagedAttributes() {
    Attributes attributes = mock(Attributes.class);
    AttributeMetadata metadata = mock(AttributeMetadata.class);
    AttributeMetadata hiddenMetadata = mock(AttributeMetadata.class);
    when(metadata.getAnnotations()).thenReturn(Map.of());
    when(hiddenMetadata.getAnnotations()).thenReturn(Map.of("inputType", "hidden"));
    when(attributes.getWritable())
        .thenReturn(
            Map.of(
                "username", List.of("voter@example.com"),
                "dateOfBirth", List.of("2000-01-01"),
                "hiddenReference", List.of("internal-value"),
                "unmanaged", List.of("value")));
    when(attributes.getUnmanagedAttributes()).thenReturn(Map.of("unmanaged", List.of("value")));
    when(attributes.getMetadata("username")).thenReturn(metadata);
    when(attributes.getMetadata("dateOfBirth")).thenReturn(metadata);
    when(attributes.getMetadata("hiddenReference")).thenReturn(hiddenMetadata);

    MultivaluedMap<String, String> result =
        LoginHintPrefill.filterWritableHints(
            Map.of(
                "username", "voter@example.com",
                "dateOfBirth", "2000-01-01",
                "verificationStatus", "VERIFIED",
                "hiddenReference", "internal-value",
                "unmanaged", "value",
                "password", "secret"),
            attributes,
            Set.of("verificationStatus"));

    assertEquals(
        Map.of("username", List.of("voter@example.com"), "dateOfBirth", List.of("2000-01-01")),
        result);
  }

  @Test
  void skipsAttributesAnnotatedToIgnoreLoginHintPrefill() {
    Attributes attributes = mock(Attributes.class);
    AttributeMetadata editable = mock(AttributeMetadata.class);
    AttributeMetadata ignored = mock(AttributeMetadata.class);
    AttributeMetadata unknownPolicy = mock(AttributeMetadata.class);
    when(editable.getAnnotations()).thenReturn(Map.of());
    when(ignored.getAnnotations())
        .thenReturn(Map.of(LoginHintPrefill.PREFILL_POLICY_ANNOTATION, "IGNORE"));
    when(unknownPolicy.getAnnotations())
        .thenReturn(Map.of(LoginHintPrefill.PREFILL_POLICY_ANNOTATION, "sometimes"));
    when(attributes.getWritable())
        .thenReturn(
            Map.of(
                "username", List.of("voter@example.com"),
                "reference", List.of("ABC-123"),
                "dateOfBirth", List.of("2000-01-01")));
    when(attributes.getUnmanagedAttributes()).thenReturn(Map.of());
    when(attributes.getMetadata("username")).thenReturn(editable);
    when(attributes.getMetadata("reference")).thenReturn(ignored);
    when(attributes.getMetadata("dateOfBirth")).thenReturn(unknownPolicy);

    MultivaluedMap<String, String> result =
        LoginHintPrefill.filterWritableHints(
            Map.of(
                "username", "voter@example.com",
                "reference", "ABC-123",
                "dateOfBirth", "2000-01-01"),
            attributes,
            Set.of());

    assertEquals(Map.of("username", List.of("voter@example.com")), result);
  }

  @Test
  void locksOnlyAttributesAnnotatedAsReadOnly() {
    Attributes attributes = mock(Attributes.class);
    AttributeMetadata editable = mock(AttributeMetadata.class);
    AttributeMetadata readOnly = mock(AttributeMetadata.class);
    when(editable.getAnnotations()).thenReturn(Map.of());
    when(readOnly.getAnnotations())
        .thenReturn(Map.of(LoginHintPrefill.PREFILL_POLICY_ANNOTATION, "READ_ONLY"));
    when(attributes.getMetadata("username")).thenReturn(readOnly);
    when(attributes.getMetadata("dateOfBirth")).thenReturn(editable);

    MultivaluedMap<String, String> writableHints = new MultivaluedHashMap<>();
    writableHints.putSingle("username", "voter@example.com");
    writableHints.putSingle("dateOfBirth", "2000-01-01");

    assertEquals(Set.of("username"), LoginHintPrefill.filterLockedHints(writableHints, attributes));
  }

  @Test
  void reportsLockedHintsThatWereNotSubmittedUnchanged() {
    MultivaluedMap<String, String> writableHints = new MultivaluedHashMap<>();
    writableHints.putSingle("username", "voter@example.com");
    writableHints.putSingle("dateOfBirth", "2000-01-01");

    MultivaluedMap<String, String> tamperedFormData = new MultivaluedHashMap<>();
    tamperedFormData.putSingle("username", "someone.else@example.com");
    tamperedFormData.putSingle("dateOfBirth", "2000-01-01");

    assertEquals(
        Set.of("username"),
        LoginHintPrefill.findModifiedLockedHints(
            writableHints, Set.of("username", "dateOfBirth"), tamperedFormData));
    assertTrue(
        LoginHintPrefill.findModifiedLockedHints(
                writableHints, Set.of("dateOfBirth"), tamperedFormData)
            .isEmpty());
    assertEquals(
        Set.of("username"),
        LoginHintPrefill.findModifiedLockedHints(
            writableHints, Set.of("username"), new MultivaluedHashMap<>()));
  }

  @Test
  void stockActionMarksLockedAttributesOnEveryRender() {
    FormContext context = mock(FormContext.class);
    LoginFormsProvider form = mock(LoginFormsProvider.class);
    HttpRequest request = mock(HttpRequest.class);

    stubReadOnlyUsernameHint(context, request);
    when(request.getHttpMethod()).thenReturn("GET");

    new LoginHintRegistrationPrefill().buildPage(context, form);

    verify(form)
        .setAttribute(LoginHintRegistrationPrefill.READ_ONLY_ATTRIBUTES, List.of("username"));
    verify(form).setFormData(ArgumentMatchers.<MultivaluedMap<String, String>>any());

    when(request.getHttpMethod()).thenReturn("POST");
    new LoginHintRegistrationPrefill().buildPage(context, form);

    verify(form, times(2))
        .setAttribute(LoginHintRegistrationPrefill.READ_ONLY_ATTRIBUTES, List.of("username"));
    verify(form).setFormData(ArgumentMatchers.<MultivaluedMap<String, String>>any());
  }

  @Test
  void stockActionRejectsALockedFieldSubmittedWithADifferentValue() {
    ValidationContext context = mock(ValidationContext.class);
    HttpRequest request = mock(HttpRequest.class);
    MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
    formData.putSingle("username", "someone.else@example.com");

    stubReadOnlyUsernameHint(context, request);
    when(request.getDecodedFormParameters()).thenReturn(formData);

    new LoginHintRegistrationPrefill().validate(context);

    MultivaluedMap<String, String> restoredFormData = new MultivaluedHashMap<>();
    restoredFormData.putSingle("username", "voter@example.com");

    verify(context, never()).success();
    verify(context)
        .validationError(eq(restoredFormData), ArgumentMatchers.<java.util.List<FormMessage>>any());
  }

  @Test
  void stockActionAcceptsALockedFieldSubmittedUnchanged() {
    ValidationContext context = mock(ValidationContext.class);
    HttpRequest request = mock(HttpRequest.class);
    MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
    formData.putSingle("username", "voter@example.com");

    stubReadOnlyUsernameHint(context, request);
    when(request.getDecodedFormParameters()).thenReturn(formData);

    new LoginHintRegistrationPrefill().validate(context);

    verify(context).success();
    verify(context, never())
        .validationError(
            ArgumentMatchers.<MultivaluedMap<String, String>>any(),
            ArgumentMatchers.<java.util.List<FormMessage>>any());
  }

  private static void stubReadOnlyUsernameHint(FormContext context, HttpRequest request) {
    AuthenticationSessionModel authenticationSession = mock(AuthenticationSessionModel.class);
    KeycloakSession session = mock(KeycloakSession.class);
    UserProfileProvider profileProvider = mock(UserProfileProvider.class);
    UserProfile profile = mock(UserProfile.class);
    Attributes attributes = mock(Attributes.class);
    AttributeMetadata metadata = mock(AttributeMetadata.class);

    when(context.getHttpRequest()).thenReturn(request);
    when(context.getAuthenticationSession()).thenReturn(authenticationSession);
    when(authenticationSession.getClientNotes())
        .thenReturn(Map.of(clientNote("username"), "voter@example.com"));
    when(context.getSession()).thenReturn(session);
    when(session.getProvider(UserProfileProvider.class)).thenReturn(profileProvider);
    when(profileProvider.create(
            eq(UserProfileContext.REGISTRATION), ArgumentMatchers.<Map<String, ?>>any()))
        .thenReturn(profile);
    when(profile.getAttributes()).thenReturn(attributes);
    when(attributes.getWritable()).thenReturn(Map.of("username", List.of("voter@example.com")));
    when(attributes.getUnmanagedAttributes()).thenReturn(Map.of());
    when(attributes.getMetadata("username")).thenReturn(metadata);
    when(metadata.getAnnotations())
        .thenReturn(Map.of(LoginHintPrefill.PREFILL_POLICY_ANNOTATION, "READ_ONLY"));
  }

  @Test
  void stockActionPrefillsOnlyInitialGetRender() {
    FormContext context = mock(FormContext.class);
    LoginFormsProvider form = mock(LoginFormsProvider.class);
    HttpRequest request = mock(HttpRequest.class);
    AuthenticationSessionModel authenticationSession = mock(AuthenticationSessionModel.class);
    KeycloakSession session = mock(KeycloakSession.class);
    UserProfileProvider profileProvider = mock(UserProfileProvider.class);
    UserProfile profile = mock(UserProfile.class);
    Attributes attributes = mock(Attributes.class);
    AttributeMetadata metadata = mock(AttributeMetadata.class);

    when(context.getHttpRequest()).thenReturn(request);
    when(request.getHttpMethod()).thenReturn("GET");
    when(context.getAuthenticationSession()).thenReturn(authenticationSession);
    when(authenticationSession.getClientNotes())
        .thenReturn(Map.of(clientNote("username"), "voter@example.com"));
    when(context.getSession()).thenReturn(session);
    when(session.getProvider(UserProfileProvider.class)).thenReturn(profileProvider);
    when(profileProvider.create(
            eq(UserProfileContext.REGISTRATION), ArgumentMatchers.<Map<String, ?>>any()))
        .thenReturn(profile);
    when(profile.getAttributes()).thenReturn(attributes);
    when(attributes.getWritable()).thenReturn(Map.of("username", List.of("voter@example.com")));
    when(attributes.getUnmanagedAttributes()).thenReturn(Map.of());
    when(attributes.getMetadata("username")).thenReturn(metadata);

    new LoginHintRegistrationPrefill().buildPage(context, form);

    verify(form).setFormData(ArgumentMatchers.<MultivaluedMap<String, String>>any());

    when(request.getHttpMethod()).thenReturn("POST");
    new LoginHintRegistrationPrefill().buildPage(context, form);

    verify(form).setFormData(ArgumentMatchers.<MultivaluedMap<String, String>>any());
    verify(profileProvider, never())
        .create(eq(UserProfileContext.ACCOUNT), ArgumentMatchers.<Map<String, ?>>any());
  }

  @Test
  void stockActionRejectsAnInvalidHintSetAtomically() {
    FormContext context = mock(FormContext.class);
    LoginFormsProvider form = mock(LoginFormsProvider.class);
    HttpRequest request = mock(HttpRequest.class);
    AuthenticationSessionModel authenticationSession = mock(AuthenticationSessionModel.class);

    when(context.getHttpRequest()).thenReturn(request);
    when(request.getHttpMethod()).thenReturn("GET");
    when(context.getAuthenticationSession()).thenReturn(authenticationSession);
    when(authenticationSession.getClientNotes())
        .thenReturn(
            Map.of(
                clientNote("username"), "voter@example.com", clientNote("invalid field"), "value"));

    new LoginHintRegistrationPrefill().buildPage(context, form);

    verifyNoInteractions(form);
  }

  @Test
  void stockActionFactoryIsRegisteredWithExpectedRequirements() throws IOException {
    LoginHintRegistrationPrefill action = new LoginHintRegistrationPrefill();
    String serviceResource = "META-INF/services/" + FormActionFactory.class.getName();

    try (InputStream providers = getClass().getClassLoader().getResourceAsStream(serviceResource)) {
      assertNotNull(providers);
      String registeredProviders = new String(providers.readAllBytes(), StandardCharsets.UTF_8);
      assertTrue(registeredProviders.contains(LoginHintRegistrationPrefill.class.getName()));
    }

    assertEquals(LoginHintRegistrationPrefill.PROVIDER_ID, action.getId());
    assertArrayEquals(
        new AuthenticationExecutionModel.Requirement[] {
          AuthenticationExecutionModel.Requirement.REQUIRED,
          AuthenticationExecutionModel.Requirement.DISABLED
        },
        action.getRequirementChoices());
  }

  private static String clientNote(String attributeName) {
    return AuthorizationEndpoint.LOGIN_SESSION_NOTE_ADDITIONAL_REQ_PARAMS_PREFIX
        + LoginHintPrefill.HINT_PREFIX
        + attributeName;
  }
}
