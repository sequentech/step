// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.authenticator.forgot_password;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

import jakarta.ws.rs.core.MultivaluedHashMap;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.stream.Stream;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.credential.hash.PasswordHashProvider;
import org.keycloak.events.EventBuilder;
import org.keycloak.http.HttpRequest;
import org.keycloak.models.*;
import org.keycloak.representations.userprofile.config.UPAttribute;
import org.keycloak.representations.userprofile.config.UPConfig;
import org.keycloak.services.managers.BruteForceProtector;
import org.keycloak.userprofile.UserProfileProvider;
import org.mockito.MockedStatic;

class EncryptedAttributeLoginTest {
  private final KeycloakSession session = mock(KeycloakSession.class);
  private final RealmModel realm = mock(RealmModel.class);
  private final UserProvider users = mock(UserProvider.class);
  private final AuthenticatorConfigModel config = new AuthenticatorConfigModel();
  private Map<String, String> vector;
  private UserModel voter;
  private List<String> identifiers = List.of("username");
  private final MultiAttributeCredentialResolver.MatchPolicy policy =
      MultiAttributeCredentialResolver.MatchPolicy.REJECT_AMBIGUOUS;

  @BeforeEach
  void setup() throws Exception {
    vector = EncryptedAttributeCredentialTest.fixture();
    when(session.users()).thenReturn(users);
    when(session.singleUseObjects()).thenReturn(new FakeSingleUseObjectProvider());
    when(session.getProvider(PasswordHashProvider.class))
        .thenReturn(mock(PasswordHashProvider.class));
    when(realm.getId()).thenReturn("realm");
    when(realm.getName()).thenReturn("tenant-tenant-event-event");
    config.setConfig(
        new HashMap<>(
            Map.of(
                "matchAttributes",
                "username",
                "credentialPolicy",
                "SECRET_ATTRIBUTE",
                "credentialSecretAttribute",
                "login-code")));
    var attribute = new UPAttribute();
    attribute.setName("login-code");
    attribute.setAnnotations(Map.of("sequent.secret", "true"));
    var profileConfig = new UPConfig();
    profileConfig.setAttributes(List.of(attribute));
    var profiles = mock(UserProfileProvider.class);
    when(profiles.getConfiguration()).thenReturn(profileConfig);
    when(session.getProvider(UserProfileProvider.class)).thenReturn(profiles);
    voter = mock(UserModel.class);
    when(voter.getId()).thenReturn("voter");
    when(voter.isEnabled()).thenReturn(true);
    when(voter.getAttributeStream("login-code")).thenAnswer(i -> Stream.of(vector.get("envelope")));
    when(users.getUserByUsername(realm, "voter")).thenReturn(voter);
  }

  // Substitute only the environment dependency, exercising the real policy and codec.
  private MockedStatic<EncryptedAttributeCredential> master(String key) {
    var mocked = mockStatic(EncryptedAttributeCredential.class, CALLS_REAL_METHODS);
    mocked
        .when(
            () ->
                EncryptedAttributeCredential.verifier(session, realm, config, identifiers, policy))
        .thenAnswer(
            i ->
                EncryptedAttributeCredential.verifier(
                    session, realm, config, identifiers, policy, key));
    return mocked;
  }

  private MultiAttributeCredentialResolver.Resolution resolve(String value) {
    return MultiAttributeCredentialResolver.resolveAuthenticatedUser(
        session,
        realm,
        identifiers,
        Map.of(identifiers.get(0), "voter"),
        value,
        Utils.getThrottleConfig(config),
        policy,
        config);
  }

  @Test
  void secretLoginDoesNotRequireOrConsultAKeycloakPassword() {
    try (var ignored = master(vector.get("master"))) {
      assertEquals(voter, resolve(vector.get("plaintext")).authenticatedUser().orElseThrow());
      verify(voter, never()).credentialManager();
      var failure = resolve("incorrect");
      assertTrue(failure.authenticatedUser().isEmpty());
      assertEquals(voter, failure.attributableUser().orElseThrow());
    }
  }

  @Test
  void missingMalformedAndWrongMasterCannotAuthenticate() {
    for (String key : new String[] {null, "", "invalid", "00".repeat(32)}) {
      try (var ignored = master(key)) {
        assertTrue(resolve(vector.get("plaintext")).authenticatedUser().isEmpty());
        verify(voter, never()).credentialManager();
      }
    }
  }

  @Test
  void missingAnnotationOrSecretLookupConfigurationCannotAuthenticate() {
    try (var ignored = master(vector.get("master"))) {
      config.getConfig().put("credentialSecretAttribute", "other");
      assertTrue(resolve(vector.get("plaintext")).authenticatedUser().isEmpty());
      config.getConfig().put("credentialSecretAttribute", "login-code");
      assertTrue(
          EncryptedAttributeCredential.verifier(
                  session, realm, config, List.of("login-code"), policy, vector.get("master"))
              .isEmpty());
    }
  }

  @Test
  void nonexistentAndIncorrectSecretBothPerformDummyHashWork() {
    var hasher = session.getProvider(PasswordHashProvider.class);
    try (var ignored = master(vector.get("master"))) {
      assertTrue(resolve("wrong").authenticatedUser().isEmpty());
      verify(hasher, times(1)).encodedCredential(anyString(), anyInt());
      when(users.getUserByUsername(realm, "voter")).thenReturn(null);
      assertTrue(resolve("wrong").authenticatedUser().isEmpty());
      verify(hasher, times(2)).encodedCredential(anyString(), anyInt());
    }
  }

  @Test
  void disabledAndLockedAccountsCannotAuthenticate() {
    try (var ignored = master(vector.get("master"))) {
      when(voter.isEnabled()).thenReturn(false);
      assertTrue(resolve(vector.get("plaintext")).authenticatedUser().isEmpty());
      when(voter.isEnabled()).thenReturn(true);
      when(realm.isBruteForceProtected()).thenReturn(true);
      var protector = mock(BruteForceProtector.class);
      when(session.getProvider(BruteForceProtector.class)).thenReturn(protector);
      when(protector.isTemporarilyDisabled(session, realm, voter)).thenReturn(true);
      var locked = resolve(vector.get("plaintext"));
      assertTrue(locked.authenticatedUser().isEmpty());
      assertEquals(MultiAttributeCredentialResolver.LockoutState.TEMPORARY, locked.lockoutState());
    }
  }

  @Test
  void secretGuessesShareTheSameTupleThrottle() {
    config.getConfig().put("tupleMaxFailures", "2");
    try (var ignored = master(vector.get("master"))) {
      assertTrue(resolve("wrong-one").authenticatedUser().isEmpty());
      assertTrue(resolve("wrong-two").authenticatedUser().isEmpty());
      assertTrue(resolve(vector.get("plaintext")).authenticatedUser().isEmpty());
    }
  }

  @Test
  void ambiguityIncludingLockedAccountsAndCandidateCapsFailClosed() {
    identifiers = List.of("group");
    var other = mock(UserModel.class);
    when(other.getId()).thenReturn("other");
    when(other.isEnabled()).thenReturn(true);
    when(other.getAttributeStream("login-code"))
        .thenAnswer(i -> Stream.of(vector.get("other_envelope")));
    when(users.searchForUserStream(eq(realm), anyMap(), eq(0), anyInt()))
        .thenAnswer(i -> Stream.of(voter, other));
    try (var ignored = master(vector.get("master"))) {
      assertTrue(resolve(vector.get("plaintext")).authenticatedUser().isEmpty());
      when(realm.isBruteForceProtected()).thenReturn(true);
      var protector = mock(BruteForceProtector.class);
      when(session.getProvider(BruteForceProtector.class)).thenReturn(protector);
      when(protector.isTemporarilyDisabled(session, realm, other)).thenReturn(true);
      assertTrue(resolve(vector.get("plaintext")).authenticatedUser().isEmpty());
      config.getConfig().put("maxCandidates", "1");
      clearInvocations(voter, other);
      assertTrue(resolve(vector.get("plaintext")).authenticatedUser().isEmpty());
      verify(voter, never()).getAttributeStream(anyString());
      verify(other, never()).getAttributeStream(anyString());
    }
  }

  @Test
  void browserAndIvrUseTheExistingPasswordParameter() {
    for (boolean direct : List.of(false, true)) {
      var context = mock(AuthenticationFlowContext.class);
      var request = mock(HttpRequest.class);
      var form = new MultivaluedHashMap<String, String>();
      form.add("username", "voter");
      form.add("password", vector.get("plaintext"));
      when(request.getDecodedFormParameters()).thenReturn(form);
      when(context.getHttpRequest()).thenReturn(request);
      when(context.getSession()).thenReturn(session);
      when(context.getRealm()).thenReturn(realm);
      when(context.getAuthenticatorConfig()).thenReturn(config);
      when(context.getEvent()).thenReturn(mock(EventBuilder.class));
      if (direct) {
        config
            .getConfig()
            .putAll(
                Map.of(
                    "field",
                    "identifier##pin",
                    "max_digits",
                    "20##20",
                    "kind",
                    "identifier##secret",
                    "maps_to",
                    "username##password"));
      }
      try (var ignored = master(vector.get("master"))) {
        if (direct) new MultiAttributePasswordDirectGrantAuthenticator().authenticate(context);
        else new MultiAttributePasswordAuthenticator().action(context);
        verify(context).setUser(voter);
        verify(context).success();
        verify(voter, never()).credentialManager();
      }
    }
  }
}
