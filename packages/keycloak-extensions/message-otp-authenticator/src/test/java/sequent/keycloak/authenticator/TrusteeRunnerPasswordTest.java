// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequent.vote>
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import java.util.stream.Stream;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.models.ClientModel;
import org.keycloak.models.GroupModel;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;

class TrusteeRunnerPasswordTest {
  private final RealmModel realm = mock(RealmModel.class);
  private final ClientModel client = mock(ClientModel.class);
  private final UserModel user = mock(UserModel.class);
  private final GroupModel group = mock(GroupModel.class);
  private final AuthenticationSessionModel session = mock(AuthenticationSessionModel.class);

  @BeforeEach
  void setup() {
    when(realm.getName()).thenReturn(TrusteeRunnerPassword.REALM);
    when(client.getClientId()).thenReturn(TrusteeRunnerPassword.CLIENT_ID);
    when(client.isEnabled()).thenReturn(true);
    when(client.getProtocol()).thenReturn("openid-connect");
    when(client.getClientAuthenticatorType()).thenReturn("client-secret");
    when(client.isDirectAccessGrantsEnabled()).thenReturn(true);
    when(user.getUsername()).thenReturn("trustee1");
    when(user.getId()).thenReturn("synthetic-subject");
    when(user.isEnabled()).thenReturn(true);
    when(user.getAttributeStream("trustee")).thenAnswer(invocation -> Stream.of("trustee1"));
    when(user.getAttributeStream("tenant-id"))
        .thenAnswer(invocation -> Stream.of("90505c8a-23a9-4cdf-a26b-4e19f6a097d5"));
    when(group.getName()).thenReturn("trustee");
    when(user.getGroupsStream()).thenAnswer(invocation -> Stream.of(group));
    when(session.getClient()).thenReturn(client);
  }

  @Test
  void requiresPasswordSuccessInThisSessionForThisUser() {
    assertTrue(TrusteeRunnerPassword.eligible(realm, client, user));
    assertFalse(TrusteeRunnerPassword.verifiedMachineSession(realm, session, user));
    when(session.getAuthNote("sequent.trustee-runner.password-verified"))
        .thenReturn("a-different-subject");
    assertFalse(TrusteeRunnerPassword.verifiedMachineSession(realm, session, user));
    when(session.getAuthNote("sequent.trustee-runner.password-verified"))
        .thenReturn("synthetic-subject");
    assertTrue(TrusteeRunnerPassword.verifiedMachineSession(realm, session, user));
  }

  @Test
  void rejectsBrowserPublicAndSharedClientEvenWithNote() {
    when(session.getAuthNote("sequent.trustee-runner.password-verified"))
        .thenReturn("synthetic-subject");
    when(client.isStandardFlowEnabled()).thenReturn(true);
    assertFalse(TrusteeRunnerPassword.verifiedMachineSession(realm, session, user));
    when(client.isStandardFlowEnabled()).thenReturn(false);
    when(client.isPublicClient()).thenReturn(true);
    assertFalse(TrusteeRunnerPassword.verifiedMachineSession(realm, session, user));
    when(client.isPublicClient()).thenReturn(false);
    when(client.getClientId()).thenReturn("api-key-client");
    assertFalse(TrusteeRunnerPassword.verifiedMachineSession(realm, session, user));
  }

  @Test
  void staysDisabledUntilProvisionedAndRejectsASeparateHumanTrustee() {
    when(client.isEnabled()).thenReturn(false);
    assertFalse(TrusteeRunnerPassword.eligible(realm, client, user));
    when(client.isEnabled()).thenReturn(true);
    when(user.getUsername()).thenReturn("human-trustee");
    when(user.getAttributeStream("trustee"))
        .thenAnswer(invocation -> Stream.of("human-trustee"));
    assertFalse(TrusteeRunnerPassword.eligible(realm, client, user));
  }

  @Test
  void acceptsExactlySixNamedTrusteesWithMatchingIdentityClaims() {
    for (int i = 0; i <= 7; i++) {
      String name = "trustee" + i;
      when(user.getUsername()).thenReturn(name);
      when(user.getAttributeStream("trustee")).thenAnswer(invocation -> Stream.of(name));
      if (i >= 1 && i <= 6) {
        assertTrue(TrusteeRunnerPassword.eligible(realm, client, user), name);
      } else {
        assertFalse(TrusteeRunnerPassword.eligible(realm, client, user), name);
      }
    }
  }

  @Test
  void rejectsWrongRealmGroupUserAndIdentityMapping() {
    when(realm.getName()).thenReturn("tenant-somewhere-else");
    assertFalse(TrusteeRunnerPassword.eligible(realm, client, user));
    when(realm.getName()).thenReturn(TrusteeRunnerPassword.REALM);
    when(group.getName()).thenReturn("admin");
    assertFalse(TrusteeRunnerPassword.eligible(realm, client, user));
    when(group.getName()).thenReturn("trustee");
    when(user.getUsername()).thenReturn("other-operator");
    assertFalse(TrusteeRunnerPassword.eligible(realm, client, user));
    when(user.getUsername()).thenReturn("trustee1");
    when(user.getAttributeStream("trustee")).thenAnswer(invocation -> Stream.of("trustee2"));
    assertFalse(TrusteeRunnerPassword.eligible(realm, client, user));
  }
}
