// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import jakarta.ws.rs.core.MultivaluedMap;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.FormContext;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.http.HttpRequest;
import org.keycloak.models.KeycloakSession;
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
    when(attributes.getWritable())
        .thenReturn(
            Map.of(
                "username", List.of("voter@example.com"),
                "dateOfBirth", List.of("2000-01-01"),
                "unmanaged", List.of("value")));
    when(attributes.getUnmanagedAttributes()).thenReturn(Map.of("unmanaged", List.of("value")));
    when(attributes.getMetadata("username")).thenReturn(metadata);
    when(attributes.getMetadata("dateOfBirth")).thenReturn(metadata);

    MultivaluedMap<String, String> result =
        LoginHintPrefill.filterWritableHints(
            Map.of(
                "username", "voter@example.com",
                "dateOfBirth", "2000-01-01",
                "verificationStatus", "VERIFIED",
                "unmanaged", "value",
                "password", "secret"),
            attributes,
            Set.of("verificationStatus"));

    assertEquals(
        Map.of("username", List.of("voter@example.com"), "dateOfBirth", List.of("2000-01-01")),
        result);
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

  private static String clientNote(String attributeName) {
    return AuthorizationEndpoint.LOGIN_SESSION_NOTE_ADDITIONAL_REQ_PARAMS_PREFIX
        + LoginHintPrefill.HINT_PREFIX
        + attributeName;
  }
}
