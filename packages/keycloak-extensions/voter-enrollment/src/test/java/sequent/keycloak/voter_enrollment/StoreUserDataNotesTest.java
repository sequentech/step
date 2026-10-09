// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.anyMap;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import java.util.List;
import java.util.Set;
import java.util.stream.Stream;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.FormContext;
import org.keycloak.authentication.forms.RegistrationPage;
import org.keycloak.http.HttpRequest;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.UserProvider;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.mockito.ArgumentCaptor;

class StoreUserDataNotesTest {

  private static final List<String> AUTHENTICATOR_NOTES =
      List.of(
          "code",
          "ttl",
          "one-time-link.visited",
          "Email verified",
          "verificationCompleted",
          "verificationStatus",
          "verificationRejectionReason",
          "verificationMismatchedFields",
          "fields_match");

  /**
   * Profile attributes and other form fields such as the password and terms acceptance must still
   * reach the notes the later authenticators read, while the notes those authenticators own keep
   * only the values they set themselves. Neither those names nor names containing the list
   * separator may appear in the list of stored form fields.
   */
  @Test
  void formFieldsBecomeNotesExceptAuthenticatorNotes() {
    MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
    formData.add(UserModel.FIRST_NAME, "Ana");
    formData.add(UserModel.EMAIL, "voter@example.com");
    formData.add(RegistrationPage.FIELD_PASSWORD, "secret");
    formData.add("termsAccepted", "on");
    AUTHENTICATOR_NOTES.forEach(note -> formData.add(note, "99999999999999"));
    formData.add("keyUserdata", "code");
    formData.add("lastName;code", "Ana");
    formData.add("userId", "other-voter-id");

    AuthenticationSessionModel authSession = storeNotes(formData);

    verify(authSession).setAuthNote(UserModel.FIRST_NAME, "Ana");
    verify(authSession).setAuthNote(UserModel.EMAIL, "voter@example.com");
    verify(authSession).setAuthNote(RegistrationPage.FIELD_PASSWORD, "secret");
    verify(authSession).setAuthNote("termsAccepted", "on");
    verify(authSession).setAuthNote("userId", "voter-id");
    AUTHENTICATOR_NOTES.forEach(
        note -> verify(authSession, never()).setAuthNote(eq(note), anyString()));
    verify(authSession, never()).setAuthNote(eq("lastName;code"), anyString());
    verify(authSession, never()).setAuthNote("userId", "other-voter-id");
    ArgumentCaptor<String> storedFields = ArgumentCaptor.forClass(String.class);
    verify(authSession).setAuthNote(eq("keyUserdata"), storedFields.capture());
    assertEquals(
        Set.of(
            UserModel.FIRST_NAME,
            UserModel.EMAIL,
            RegistrationPage.FIELD_PASSWORD,
            "termsAccepted"),
        Set.of(storedFields.getValue().split(";")));
  }

  /** A submission made only of reserved names stores an empty field list instead of failing. */
  @Test
  void onlyAuthenticatorNotesStoreAnEmptyFieldList() {
    MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
    AUTHENTICATOR_NOTES.forEach(note -> formData.add(note, "99999999999999"));

    AuthenticationSessionModel authSession = storeNotes(formData);

    verify(authSession).setAuthNote("keyUserdata", "");
    verify(authSession).setAuthNote("userId", "voter-id");
    AUTHENTICATOR_NOTES.forEach(
        note -> verify(authSession, never()).setAuthNote(eq(note), anyString()));
  }

  /** An empty stored field list reads back as no fields, not as one field with an empty name. */
  @Test
  void emptyFieldListReadsBackAsNoFields() {
    AuthenticationFlowContext context = mock(AuthenticationFlowContext.class);
    AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
    when(context.getAuthenticationSession()).thenReturn(authSession);
    when(authSession.getAuthNote("keyUserdata")).thenReturn("");

    assertTrue(Utils.getAttributeValuesFromAuthNote(context, "keyUserdata").isEmpty());
  }

  private static AuthenticationSessionModel storeNotes(MultivaluedMap<String, String> formData) {
    FormContext context = mock(FormContext.class);
    HttpRequest request = mock(HttpRequest.class);
    AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
    KeycloakSession session = mock(KeycloakSession.class);
    RealmModel realm = mock(RealmModel.class);
    UserProvider users = mock(UserProvider.class);
    UserModel user = mock(UserModel.class);

    when(context.getHttpRequest()).thenReturn(request);
    when(request.getDecodedFormParameters()).thenReturn(formData);
    when(context.getAuthenticationSession()).thenReturn(authSession);
    when(context.getSession()).thenReturn(session);
    when(context.getRealm()).thenReturn(realm);
    when(session.users()).thenReturn(users);
    when(users.searchForUserStream(eq(realm), anyMap())).thenReturn(Stream.of(user));
    when(user.getId()).thenReturn("voter-id");

    Utils.storeUserDataInAuthSessionNotes(context, List.of(UserModel.FIRST_NAME));
    return authSession;
  }
}
