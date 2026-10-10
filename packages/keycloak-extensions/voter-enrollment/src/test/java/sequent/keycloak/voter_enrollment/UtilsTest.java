// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyMap;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import java.util.List;
import java.util.Map;
import java.util.stream.Stream;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.FormContext;
import org.keycloak.http.HttpRequest;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.UserProvider;
import org.keycloak.sessions.AuthenticationSessionModel;

class UtilsTest {

  private static final String DATE_OF_BIRTH = "dateOfBirth";
  private static final String USERNAME_VALUE = "123456";
  private static final String DATE_OF_BIRTH_VALUE = "1990-01-01";
  private static final String USER_ID_AUTH_NOTE = "userId";
  private static final List<String> SEARCH_ATTRIBUTES = List.of(UserModel.USERNAME, DATE_OF_BIRTH);

  private final UserProvider users = mock(UserProvider.class);
  private final RealmModel realm = mock(RealmModel.class);
  private final FormContext context = mock(FormContext.class);
  private final UserModel voter = mock(UserModel.class);

  UtilsTest() {
    KeycloakSession session = mock(KeycloakSession.class);
    when(context.getSession()).thenReturn(session);
    when(context.getRealm()).thenReturn(realm);
    when(session.users()).thenReturn(users);
  }

  @Test
  void lookupUserByFormDataRequiresEveryConfiguredSearchAttribute() {
    when(users.searchForUserStream(any(RealmModel.class), anyMap())).thenReturn(Stream.of(voter));
    MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
    formData.add(UserModel.USERNAME, USERNAME_VALUE);

    assertNull(Utils.lookupUserByFormData(context, SEARCH_ATTRIBUTES, formData));
    verify(users, never()).searchForUserStream(any(RealmModel.class), anyMap());
  }

  @Test
  void lookupUserByFormDataRejectsBlankSearchAttribute() {
    when(users.searchForUserStream(any(RealmModel.class), anyMap())).thenReturn(Stream.of(voter));
    MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
    formData.add(UserModel.USERNAME, USERNAME_VALUE);
    formData.add(DATE_OF_BIRTH, "  ");

    assertNull(Utils.lookupUserByFormData(context, SEARCH_ATTRIBUTES, formData));
    verify(users, never()).searchForUserStream(any(RealmModel.class), anyMap());
  }

  @Test
  void lookupUserByFormDataWithoutSearchAttributesFindsNoUser() {
    when(users.searchForUserStream(any(RealmModel.class), anyMap())).thenReturn(Stream.of(voter));
    MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
    formData.add(UserModel.USERNAME, USERNAME_VALUE);

    assertNull(Utils.lookupUserByFormData(context, List.of(), formData));
    verify(users, never()).searchForUserStream(any(RealmModel.class), anyMap());
  }

  @Test
  void lookupUserByFormDataSearchesForExactTrimmedValues() {
    when(users.searchForUserStream(any(RealmModel.class), anyMap())).thenReturn(Stream.of(voter));
    MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
    formData.add(UserModel.USERNAME, " " + USERNAME_VALUE + " ");
    formData.add(DATE_OF_BIRTH, DATE_OF_BIRTH_VALUE);
    formData.add("phone", "555");

    assertEquals(voter, Utils.lookupUserByFormData(context, SEARCH_ATTRIBUTES, formData));
    verify(users)
        .searchForUserStream(
            realm,
            Map.of(
                UserModel.USERNAME,
                USERNAME_VALUE,
                DATE_OF_BIRTH,
                DATE_OF_BIRTH_VALUE,
                UserModel.EXACT,
                Boolean.TRUE.toString()));
  }

  @Test
  void lookupUserByFormDataRejectsAmbiguousMatches() {
    UserModel otherVoter = mock(UserModel.class);
    when(users.searchForUserStream(any(RealmModel.class), anyMap()))
        .thenReturn(Stream.of(voter, otherVoter));
    MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
    formData.add(UserModel.USERNAME, USERNAME_VALUE);
    formData.add(DATE_OF_BIRTH, DATE_OF_BIRTH_VALUE);

    assertNull(Utils.lookupUserByFormData(context, SEARCH_ATTRIBUTES, formData));
  }

  @Test
  void storedRegistrationDataIsNotBoundToAUserWithoutSearchAttributes() {
    HttpRequest request = mock(HttpRequest.class);
    AuthenticationSessionModel authenticationSession = mock(AuthenticationSessionModel.class);
    MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
    formData.add(UserModel.USERNAME, USERNAME_VALUE);
    when(context.getHttpRequest()).thenReturn(request);
    when(request.getDecodedFormParameters()).thenReturn(formData);
    when(context.getAuthenticationSession()).thenReturn(authenticationSession);
    when(voter.getId()).thenReturn("voter-id");
    when(users.searchForUserStream(any(RealmModel.class), anyMap())).thenReturn(Stream.of(voter));

    Utils.storeUserDataInAuthSessionNotes(context, List.of());

    verify(authenticationSession).setAuthNote(UserModel.USERNAME, USERNAME_VALUE);
    verify(authenticationSession, never()).setAuthNote(eq(USER_ID_AUTH_NOTE), any());
    verify(authenticationSession).removeAuthNote(USER_ID_AUTH_NOTE);
  }
}
