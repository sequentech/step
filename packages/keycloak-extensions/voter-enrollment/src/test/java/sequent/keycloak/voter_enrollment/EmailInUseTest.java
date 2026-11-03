// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import java.util.List;
import org.junit.jupiter.api.Test;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.ModelDuplicateException;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.UserProvider;

class EmailInUseTest {

  private static final String EMAIL = "voter@example.com";
  private static final List<String> UPDATES_EMAIL =
      List.of("email", "sequent.read-only.mobile-number");

  private final KeycloakSession session = mock(KeycloakSession.class);
  private final RealmModel realm = mock(RealmModel.class);
  private final UserProvider users = mock(UserProvider.class);

  EmailInUseTest() {
    when(session.users()).thenReturn(users);
  }

  private static UserModel user(String id) {
    UserModel user = mock(UserModel.class);
    when(user.getId()).thenReturn(id);
    return user;
  }

  /** Keycloak keeps emails unique, so setting another voter's email would fail the enrollment. */
  @Test
  void anotherVotersEmailIsInUse() {
    UserModel enrolling = user("enrolling");
    UserModel other = user("other");
    when(users.getUserByEmail(realm, EMAIL)).thenReturn(other);

    assertTrue(LookupAndUpdateUser.emailInUse(session, realm, enrolling, UPDATES_EMAIL, EMAIL));
  }

  @Test
  void theVotersOwnEmailIsNotInUse() {
    UserModel enrolling = user("enrolling");
    when(users.getUserByEmail(realm, EMAIL)).thenReturn(enrolling);

    assertFalse(LookupAndUpdateUser.emailInUse(session, realm, enrolling, UPDATES_EMAIL, EMAIL));
  }

  @Test
  void anUnusedEmailIsNotInUse() {
    when(users.getUserByEmail(realm, EMAIL)).thenReturn(null);

    assertFalse(
        LookupAndUpdateUser.emailInUse(session, realm, user("enrolling"), UPDATES_EMAIL, EMAIL));
  }

  /** An enrollment that doesn't update the email never sets it, whoever has it. */
  @Test
  void anEmailTheEnrollmentDoesNotSetIsNotChecked() {
    UserModel other = user("other");
    when(users.getUserByEmail(realm, EMAIL)).thenReturn(other);

    assertFalse(
        LookupAndUpdateUser.emailInUse(
            session, realm, user("enrolling"), List.of("sequent.read-only.mobile-number"), EMAIL));
  }

  @Test
  void aBlankEmailIsNotChecked() {
    assertFalse(
        LookupAndUpdateUser.emailInUse(session, realm, user("enrolling"), UPDATES_EMAIL, " "));
    assertFalse(
        LookupAndUpdateUser.emailInUse(session, realm, user("enrolling"), UPDATES_EMAIL, null));
  }

  /** A realm that allows duplicate emails accepts another voter's email, and never looks it up. */
  @Test
  void aRealmAllowingDuplicateEmailsAcceptsAnotherVotersEmail() {
    when(realm.isDuplicateEmailsAllowed()).thenReturn(true);
    // Keycloak's lookup fails when several users share the email
    when(users.getUserByEmail(any(), anyString()))
        .thenThrow(new ModelDuplicateException("Multiple users with email"));

    assertFalse(
        LookupAndUpdateUser.emailInUse(session, realm, user("enrolling"), UPDATES_EMAIL, EMAIL));
    verify(users, never()).getUserByEmail(any(), anyString());
  }

  /** Several holders always include another voter, even if one of them is the enrolling one. */
  @Test
  void anEmailSeveralUsersHaveIsInUse() {
    when(users.getUserByEmail(realm, EMAIL))
        .thenThrow(new ModelDuplicateException("Multiple users with email"));

    assertTrue(
        LookupAndUpdateUser.emailInUse(session, realm, user("enrolling"), UPDATES_EMAIL, EMAIL));
  }
}
