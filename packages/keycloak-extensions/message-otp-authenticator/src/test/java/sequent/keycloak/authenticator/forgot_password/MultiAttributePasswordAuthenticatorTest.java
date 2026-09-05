// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyInt;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.ArgumentMatchers.isNull;
import static org.mockito.Mockito.inOrder;
import static org.mockito.Mockito.lenient;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.Response;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.stream.Stream;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.credential.CredentialInput;
import org.keycloak.credential.hash.PasswordHashProvider;
import org.keycloak.events.EventBuilder;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.http.HttpRequest;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.SubjectCredentialManager;
import org.keycloak.models.UserCredentialModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.UserProvider;
import org.keycloak.models.UserSessionModel;
import org.keycloak.models.UserSessionProvider;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.representations.userprofile.config.UPAttribute;
import org.keycloak.representations.userprofile.config.UPConfig;
import org.keycloak.services.managers.BruteForceProtector;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.sessions.RootAuthenticationSessionModel;
import org.keycloak.userprofile.AttributeMetadata;
import org.keycloak.userprofile.Attributes;
import org.keycloak.userprofile.UserProfile;
import org.keycloak.userprofile.UserProfileContext;
import org.keycloak.userprofile.UserProfileProvider;
import org.mockito.InOrder;
import org.mockito.Mock;
import org.mockito.junit.jupiter.MockitoExtension;
import sequent.keycloak.authenticator.forgot_password.MultiAttributeCredentialResolver.LockoutState;
import sequent.keycloak.authenticator.forgot_password.MultiAttributeCredentialResolver.Resolution;

@ExtendWith(MockitoExtension.class)
class MultiAttributePasswordAuthenticatorTest {

  private static final int DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS =
      Integer.parseInt(Utils.MAX_ATTRIBUTE_LOOKUP_RESULTS_DEFAULT);

  private MultiAttributePasswordAuthenticator authenticator;

  @Mock private KeycloakSession session;
  @Mock private RealmModel realm;
  @Mock private UserProvider userProvider;
  @Mock private PasswordHashProvider passwordHashProvider;
  private final FakeSingleUseObjectProvider singleUseObjects = new FakeSingleUseObjectProvider();

  @BeforeEach
  void setUp() {
    authenticator = new MultiAttributePasswordAuthenticator();
    lenient().when(session.users()).thenReturn(userProvider);
    lenient().when(realm.getId()).thenReturn("test-realm");
    // Every "no viable candidate" path performs a dummy password hash for timing equalization
    // (see MultiAttributeCredentialResolver#performDummyHash) - stub it as a safe no-op default
    // so tests that don't care about this don't need to.
    lenient()
        .when(session.getProvider(PasswordHashProvider.class))
        .thenReturn(passwordHashProvider);
    // Per-tuple failure throttle (see MultiAttributeCredentialResolver.ThrottleConfig) needs a
    // SingleUseObjectProvider for every request that reaches a valid attribute-value combination.
    lenient().when(session.singleUseObjects()).thenReturn(singleUseObjects);
  }

  private UserModel mockUser(String id, String password, boolean enabled) {
    UserModel user = mock(UserModel.class);
    lenient().when(user.getId()).thenReturn(id);
    lenient().when(user.isEnabled()).thenReturn(enabled);
    SubjectCredentialManager credentialManager = mock(SubjectCredentialManager.class);
    lenient().when(user.credentialManager()).thenReturn(credentialManager);
    lenient()
        .when(credentialManager.isValid(any(CredentialInput.class)))
        .thenAnswer(
            invocation -> {
              CredentialInput input = invocation.getArgument(0);
              return input instanceof UserCredentialModel
                  && password.equals(((UserCredentialModel) input).getValue());
            });
    return user;
  }

  private Map<String, String> valuesOf(String... keyValuePairs) {
    Map<String, String> values = new HashMap<>();
    for (int i = 0; i < keyValuePairs.length; i += 2) {
      values.put(keyValuePairs[i], keyValuePairs[i + 1]);
    }
    return values;
  }

  // ── Single attribute, unique candidate ──────────────────────────────────

  @Test
  void singleAttribute_uniqueMatch_correctPassword_succeeds() {
    UserModel user = mockUser("user-1", "correct-horse", true);
    when(userProvider.searchForUserStream(
            realm,
            Map.of("nationalId", "X123", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenReturn(Stream.of(user));

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("nationalId"), valuesOf("nationalId", "X123"), "correct-horse");

    assertTrue(result.authenticatedUser().isPresent());
    assertEquals(user, result.authenticatedUser().get());
  }

  @Test
  void singleAttribute_uniqueMatch_wrongPassword_fails() {
    UserModel user = mockUser("user-1", "correct-horse", true);
    when(userProvider.searchForUserStream(
            realm,
            Map.of("nationalId", "X123", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenReturn(Stream.of(user));

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("nationalId"), valuesOf("nationalId", "X123"), "wrong");

    assertTrue(result.authenticatedUser().isEmpty());
  }

  @Test
  void singleAttribute_disabledUser_fails() {
    UserModel user = mockUser("user-1", "correct-horse", false);
    when(userProvider.searchForUserStream(
            realm,
            Map.of("nationalId", "X123", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenReturn(Stream.of(user));

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("nationalId"), valuesOf("nationalId", "X123"), "correct-horse");

    assertTrue(result.authenticatedUser().isEmpty());
  }

  // ── Multiple attributes intersect to one candidate ──────────────────────

  @Test
  void twoAttributes_intersectionNarrowsToOne_succeeds() {
    UserModel alice = mockUser("alice", "alice-pw", true);
    UserModel bob = mockUser("bob", "bob-pw", true);

    // Both attributes are ANDed together into a single store query (see
    // MultiAttributeCredentialResolver.ThrottleConfig#maxAttributeLookupResults) - the store
    // computes the true intersection itself, so only alice (who matches both) comes back.
    when(userProvider.searchForUserStream(
            realm,
            Map.of("dateOfBirth", "19900101", "nationalId", "ALICE-ID", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenReturn(Stream.of(alice));

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session,
            realm,
            List.of("dateOfBirth", "nationalId"),
            valuesOf("dateOfBirth", "19900101", "nationalId", "ALICE-ID"),
            "alice-pw");

    assertTrue(result.authenticatedUser().isPresent());
    assertEquals(alice, result.authenticatedUser().get());
  }

  // ── effectiveMatchAttributes: form-level narrowing before the resolver ever sees the list ──

  @Test
  void effectiveMatchAttributes_dropsBlankOptionalAttribute() {
    List<String> result =
        authenticator.effectiveMatchAttributes(
            List.of("dateOfBirth", "nationalId"),
            valuesOf("dateOfBirth", "19900101", "nationalId", ""),
            Set.of("nationalId"));

    assertEquals(List.of("dateOfBirth"), result);
  }

  @Test
  void effectiveMatchAttributes_keepsMandatoryAttributeEvenWhenBlank() {
    // nationalId is blank but NOT in optionalAttributes - kept as-is, so the resolver's own
    // blank-attribute check rejects it exactly as it does today.
    List<String> result =
        authenticator.effectiveMatchAttributes(
            List.of("dateOfBirth", "nationalId"),
            valuesOf("dateOfBirth", "19900101", "nationalId", ""),
            Set.of());

    assertEquals(List.of("dateOfBirth", "nationalId"), result);
  }

  @Test
  void effectiveMatchAttributes_keepsOptionalAttributeWhenFilledIn() {
    List<String> result =
        authenticator.effectiveMatchAttributes(
            List.of("dateOfBirth", "nationalId"),
            valuesOf("dateOfBirth", "19900101", "nationalId", "X123"),
            Set.of("nationalId"));

    assertEquals(List.of("dateOfBirth", "nationalId"), result);
  }

  @Test
  void effectiveMatchAttributes_fallsBackToOriginalListWhenEveryAttributeWouldBeDropped() {
    // Both optional and blank - dropping both would hand the resolver an empty list, which it
    // treats as a static misconfiguration (see MultiAttributeCredentialResolver's empty-list
    // check) rather than a normal all-blank submission, and would run an unconstrained query if
    // it didn't. Falling back to the original list instead lets the resolver's own
    // blank-attribute check reject it the same way as any other invalid submission.
    List<String> result =
        authenticator.effectiveMatchAttributes(
            List.of("dateOfBirth", "nationalId"),
            valuesOf("dateOfBirth", "", "nationalId", ""),
            Set.of("dateOfBirth", "nationalId"));

    assertEquals(List.of("dateOfBirth", "nationalId"), result);
  }

  @Test
  void effectiveMatchAttributes_dropsOptionalUsernameWhenBlank() {
    List<String> result =
        authenticator.effectiveMatchAttributes(
            List.of("username", "dateOfBirth"),
            valuesOf("username", "", "dateOfBirth", "19900101"),
            Set.of("username"));

    assertEquals(List.of("dateOfBirth"), result);
  }

  // ── DOB-not-unique-alone case: multiple candidates, password disambiguates ──

  @Test
  void singleAttribute_multipleCandidates_passwordUniquelyMatchesOne_succeeds() {
    UserModel alice = mockUser("alice", "alice-pw", true);
    UserModel bob = mockUser("bob", "bob-pw", true);
    when(userProvider.searchForUserStream(
            realm,
            Map.of("dateOfBirth", "19900101", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenReturn(Stream.of(alice, bob));

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session,
            realm,
            List.of("dateOfBirth"),
            valuesOf("dateOfBirth", "19900101"),
            "alice-pw");

    assertTrue(result.authenticatedUser().isPresent());
    assertEquals(alice, result.authenticatedUser().get());
  }

  @Test
  void singleAttribute_multipleCandidates_passwordMatchesNone_fails() {
    UserModel alice = mockUser("alice", "alice-pw", true);
    UserModel bob = mockUser("bob", "bob-pw", true);
    when(userProvider.searchForUserStream(
            realm,
            Map.of("dateOfBirth", "19900101", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenReturn(Stream.of(alice, bob));

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("dateOfBirth"), valuesOf("dateOfBirth", "19900101"), "wrong");

    assertTrue(result.authenticatedUser().isEmpty());
  }

  @Test
  void singleAttribute_multipleCandidates_passwordMatchesMoreThanOne_fails() {
    UserModel alice = mockUser("alice", "shared-pw", true);
    UserModel bob = mockUser("bob", "shared-pw", true);
    when(userProvider.searchForUserStream(
            realm,
            Map.of("dateOfBirth", "19900101", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenReturn(Stream.of(alice, bob));

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session,
            realm,
            List.of("dateOfBirth"),
            valuesOf("dateOfBirth", "19900101"),
            "shared-pw");

    assertTrue(result.authenticatedUser().isEmpty());
  }

  // ── Zero candidates ──────────────────────────────────────────────────────

  @Test
  void noCandidates_fails() {
    when(userProvider.searchForUserStream(
            realm,
            Map.of("nationalId", "unknown", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenReturn(Stream.empty());

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("nationalId"), valuesOf("nationalId", "unknown"), "pw");

    assertTrue(result.authenticatedUser().isEmpty());
  }

  @Test
  void twoAttributes_disjointCandidateSets_fails() {
    // No single user has both dateOfBirth=19900101 AND nationalId=BOB-ID, so the store's combined
    // ANDed query returns nothing.
    when(userProvider.searchForUserStream(
            realm,
            Map.of("dateOfBirth", "19900101", "nationalId", "BOB-ID", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenReturn(Stream.empty());

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session,
            realm,
            List.of("dateOfBirth", "nationalId"),
            valuesOf("dateOfBirth", "19900101", "nationalId", "BOB-ID"),
            "alice-pw");

    assertTrue(result.authenticatedUser().isEmpty());
  }

  // ── Missing / blank submitted values ────────────────────────────────────

  @Test
  void blankSubmittedValue_failsWithoutLookup() {
    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("nationalId"), valuesOf("nationalId", "  "), "pw");

    assertTrue(result.authenticatedUser().isEmpty());
  }

  @Test
  void missingSubmittedValue_failsWithoutLookup() {
    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("nationalId"), Map.of(), "pw");

    assertTrue(result.authenticatedUser().isEmpty());
  }

  @Test
  void blankPassword_failsWithoutLookup() {
    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("nationalId"), valuesOf("nationalId", "X123"), " ");

    assertTrue(result.authenticatedUser().isEmpty());
  }

  @Test
  void noMatchAttributesConfigured_fails() {
    Resolution result =
        authenticator.resolveAuthenticatedUser(session, realm, List.of(), Map.of(), "pw");

    assertTrue(result.authenticatedUser().isEmpty());
  }

  // ── username / email special-casing ─────────────────────────────────────

  @Test
  void usernameAttribute_usesGetUserByUsername() {
    UserModel user = mockUser("user-1", "pw", true);
    when(userProvider.getUserByUsername(realm, "voter1")).thenReturn(user);

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("username"), valuesOf("username", "voter1"), "pw");

    assertTrue(result.authenticatedUser().isPresent());
    assertEquals(user, result.authenticatedUser().get());
  }

  @Test
  void emailAttribute_usesExactSearchForUserStream() {
    UserModel user = mockUser("user-1", "pw", true);
    when(userProvider.searchForUserStream(
            realm,
            Map.of(UserModel.EMAIL, "voter1@example.com", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenReturn(Stream.of(user));

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("email"), valuesOf("email", "voter1@example.com"), "pw");

    assertTrue(result.authenticatedUser().isPresent());
    assertEquals(user, result.authenticatedUser().get());
  }

  @Test
  void emailAttribute_duplicateEmailsAllowed_multipleCandidates_passwordDisambiguates() {
    // When a realm allows duplicate emails, more than one user can share the same email.
    // getUserByEmail() would silently pick just one of them; searchForUserStream() must return
    // every candidate so the password check can still disambiguate.
    UserModel alice = mockUser("alice", "alice-pw", true);
    UserModel bob = mockUser("bob", "bob-pw", true);
    when(userProvider.searchForUserStream(
            realm,
            Map.of(UserModel.EMAIL, "shared@example.com", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenReturn(Stream.of(alice, bob));

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("email"), valuesOf("email", "shared@example.com"), "bob-pw");

    assertTrue(result.authenticatedUser().isPresent());
    assertEquals(bob, result.authenticatedUser().get());
  }

  // ── Brute-force attribution ──────────────────────────────────────────────

  @Test
  void singleCandidate_wrongPassword_attributesFailureToCandidate() {
    UserModel user = mockUser("user-1", "correct-horse", true);
    when(userProvider.searchForUserStream(
            realm,
            Map.of("nationalId", "X123", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenReturn(Stream.of(user));

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("nationalId"), valuesOf("nationalId", "X123"), "wrong");

    // Even though authentication failed, the single candidate is reported so the caller can
    // context.setUser() it before signaling failure - otherwise Keycloak's brute-force counters
    // never engage (DefaultAuthenticationFlow.processResult() ->
    // AuthenticationProcessor.logFailure()
    // can only find a user via authenticationSession.getAuthenticatedUser()).
    assertTrue(result.attributableUser().isPresent());
    assertEquals(user, result.attributableUser().get());
    assertEquals(LockoutState.NONE, result.lockoutState());
  }

  @Test
  void ambiguousCandidates_wrongPassword_doesNotAttributeFailure() {
    UserModel alice = mockUser("alice", "alice-pw", true);
    UserModel bob = mockUser("bob", "bob-pw", true);
    when(userProvider.searchForUserStream(
            realm,
            Map.of("dateOfBirth", "19900101", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenReturn(Stream.of(alice, bob));

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("dateOfBirth"), valuesOf("dateOfBirth", "19900101"), "wrong");

    // With more than one viable candidate there is no single account a failure can honestly be
    // attributed to.
    assertTrue(result.attributableUser().isEmpty());
  }

  @Test
  void singleCandidate_correctPassword_attributableUserIsTheAuthenticatedUser() {
    UserModel user = mockUser("user-1", "correct-horse", true);
    when(userProvider.searchForUserStream(
            realm,
            Map.of("nationalId", "X123", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenReturn(Stream.of(user));

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("nationalId"), valuesOf("nationalId", "X123"), "correct-horse");

    assertEquals(user, result.attributableUser().orElse(null));
  }

  // ── Brute-force lockout ──────────────────────────────────────────────────

  @Mock private BruteForceProtector bruteForceProtector;

  @Test
  void singleCandidate_temporarilyLockedOut_failsWithoutPasswordCheck() {
    UserModel user = mockUser("user-1", "correct-horse", true);
    when(userProvider.searchForUserStream(
            realm,
            Map.of("nationalId", "X123", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenReturn(Stream.of(user));
    when(realm.isBruteForceProtected()).thenReturn(true);
    when(session.getProvider(BruteForceProtector.class)).thenReturn(bruteForceProtector);
    when(bruteForceProtector.isPermanentlyLockedOut(session, realm, user)).thenReturn(false);
    when(bruteForceProtector.isTemporarilyDisabled(session, realm, user)).thenReturn(true);

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("nationalId"), valuesOf("nationalId", "X123"), "correct-horse");

    assertEquals(LockoutState.TEMPORARY, result.lockoutState());
    assertEquals(user, result.attributableUser().orElse(null));
    assertTrue(result.authenticatedUser().isEmpty());
    // No credential check should even be attempted against a locked-out account.
    verify(user.credentialManager(), never()).isValid(any(CredentialInput.class));
  }

  @Test
  void singleCandidate_permanentlyLockedOut_reportsPermanent() {
    UserModel user = mockUser("user-1", "correct-horse", true);
    when(userProvider.searchForUserStream(
            realm,
            Map.of("nationalId", "X123", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenReturn(Stream.of(user));
    when(realm.isBruteForceProtected()).thenReturn(true);
    when(session.getProvider(BruteForceProtector.class)).thenReturn(bruteForceProtector);
    when(bruteForceProtector.isPermanentlyLockedOut(session, realm, user)).thenReturn(true);

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("nationalId"), valuesOf("nationalId", "X123"), "correct-horse");

    assertEquals(LockoutState.PERMANENT, result.lockoutState());
  }

  @Test
  void multipleCandidatesAllLockedOut_ambiguous_staysGeneric() {
    UserModel alice = mockUser("alice", "alice-pw", true);
    UserModel bob = mockUser("bob", "bob-pw", true);
    when(userProvider.searchForUserStream(
            realm,
            Map.of("dateOfBirth", "19900101", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenReturn(Stream.of(alice, bob));
    when(realm.isBruteForceProtected()).thenReturn(true);
    when(session.getProvider(BruteForceProtector.class)).thenReturn(bruteForceProtector);
    when(bruteForceProtector.isTemporarilyDisabled(session, realm, alice)).thenReturn(true);
    when(bruteForceProtector.isTemporarilyDisabled(session, realm, bob)).thenReturn(true);
    // isPermanentlyLockedOut() is left unstubbed - Mockito defaults unstubbed boolean methods to
    // false, which is exactly the "not permanently locked" case this test needs.

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("dateOfBirth"), valuesOf("dateOfBirth", "19900101"), "pw");

    // Two locked-out candidates: no single account to attribute the lockout to, so this stays a
    // generic failure rather than confirming "an account with this DOB is locked."
    assertEquals(LockoutState.NONE, result.lockoutState());
    assertTrue(result.attributableUser().isEmpty());
  }

  @Test
  void realmNotBruteForceProtected_neverConsultsProtector() {
    UserModel user = mockUser("user-1", "correct-horse", true);
    when(userProvider.searchForUserStream(
            realm,
            Map.of("nationalId", "X123", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenReturn(Stream.of(user));
    // realm.isBruteForceProtected() defaults to false (unstubbed) - the protector must never be
    // consulted in that case.

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("nationalId"), valuesOf("nationalId", "X123"), "correct-horse");

    assertTrue(result.authenticatedUser().isPresent());
    verify(session, never()).getProvider(BruteForceProtector.class);
  }

  // ── Timing side-channel: dummy hash on every early-return path ──────────

  @Test
  void noCandidates_performsDummyHash() {
    when(userProvider.searchForUserStream(
            realm,
            Map.of("nationalId", "unknown", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenReturn(Stream.empty());

    authenticator.resolveAuthenticatedUser(
        session, realm, List.of("nationalId"), valuesOf("nationalId", "unknown"), "pw");

    verify(passwordHashProvider).encodedCredential(anyString(), anyInt());
  }

  @Test
  void blankSubmittedValue_performsDummyHash() {
    authenticator.resolveAuthenticatedUser(
        session, realm, List.of("nationalId"), valuesOf("nationalId", "  "), "pw");

    verify(passwordHashProvider).encodedCredential(anyString(), anyInt());
  }

  @Test
  void noMatchAttributesConfigured_performsDummyHash() {
    authenticator.resolveAuthenticatedUser(session, realm, List.of(), Map.of(), "pw");

    verify(passwordHashProvider).encodedCredential(anyString(), anyInt());
  }

  @Test
  void singleCandidate_wrongPassword_doesNotAlsoPerformDummyHash() {
    // A real candidate got a real credential check - a redundant dummy hash on top would be
    // pointless extra cost, not a correctness issue, but confirms the two paths are disjoint.
    UserModel user = mockUser("user-1", "correct-horse", true);
    when(userProvider.searchForUserStream(
            realm,
            Map.of("nationalId", "X123", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenReturn(Stream.of(user));

    authenticator.resolveAuthenticatedUser(
        session, realm, List.of("nationalId"), valuesOf("nationalId", "X123"), "wrong");

    verify(passwordHashProvider, never()).encodedCredential(anyString(), anyInt());
  }

  // ── DoS mitigation: maxCandidates cap ────────────────────────────────────

  @Test
  void candidateCap_exceededCount_genericFailureWithNoPasswordChecks() {
    UserModel alice = mockUser("alice", "alice-pw", true);
    UserModel bob = mockUser("bob", "bob-pw", true);
    UserModel carol = mockUser("carol", "carol-pw", true);
    when(userProvider.searchForUserStream(
            realm,
            Map.of("dateOfBirth", "19900101", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenReturn(Stream.of(alice, bob, carol));
    MultiAttributeCredentialResolver.ThrottleConfig throttleConfig =
        new MultiAttributeCredentialResolver.ThrottleConfig(2, 10, 60);

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session,
            realm,
            List.of("dateOfBirth"),
            valuesOf("dateOfBirth", "19900101"),
            "alice-pw",
            throttleConfig);

    assertTrue(result.authenticatedUser().isEmpty());
    assertTrue(result.attributableUser().isEmpty());
    verify(alice.credentialManager(), never()).isValid(any(CredentialInput.class));
    verify(bob.credentialManager(), never()).isValid(any(CredentialInput.class));
    verify(carol.credentialManager(), never()).isValid(any(CredentialInput.class));
    verify(passwordHashProvider).encodedCredential(anyString(), anyInt());
  }

  @Test
  void candidateCap_exceededCount_neverConsultsBruteForceProtector() {
    // The cap is checked against enabled candidates before any lockoutStateOf() call, so it also
    // bounds the K BruteForceProtector lookups that would otherwise run for every candidate - not
    // just the password hashes.
    UserModel alice = mockUser("alice", "alice-pw", true);
    UserModel bob = mockUser("bob", "bob-pw", true);
    UserModel carol = mockUser("carol", "carol-pw", true);
    when(userProvider.searchForUserStream(
            realm,
            Map.of("dateOfBirth", "19900101", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenReturn(Stream.of(alice, bob, carol));
    lenient().when(realm.isBruteForceProtected()).thenReturn(true);
    MultiAttributeCredentialResolver.ThrottleConfig throttleConfig =
        new MultiAttributeCredentialResolver.ThrottleConfig(2, 10, 60);

    authenticator.resolveAuthenticatedUser(
        session,
        realm,
        List.of("dateOfBirth"),
        valuesOf("dateOfBirth", "19900101"),
        "alice-pw",
        throttleConfig);

    verify(session, never()).getProvider(BruteForceProtector.class);
  }

  @Test
  void candidateCap_atBoundary_normalDisambiguationProceeds() {
    UserModel alice = mockUser("alice", "alice-pw", true);
    UserModel bob = mockUser("bob", "bob-pw", true);
    when(userProvider.searchForUserStream(
            realm,
            Map.of("dateOfBirth", "19900101", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenReturn(Stream.of(alice, bob));
    MultiAttributeCredentialResolver.ThrottleConfig throttleConfig =
        new MultiAttributeCredentialResolver.ThrottleConfig(2, 10, 60);

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session,
            realm,
            List.of("dateOfBirth"),
            valuesOf("dateOfBirth", "19900101"),
            "alice-pw",
            throttleConfig);

    assertTrue(result.authenticatedUser().isPresent());
    assertEquals(alice, result.authenticatedUser().get());
  }

  // ── DoS mitigation: maxAttributeLookupResults ────────────────────────────

  @Test
  void attributeLookup_passesConfiguredMaxAttributeLookupResultsAsQueryLimit() {
    // Bounding retrieval is the user store's job, via maxResults - a real SQL LIMIT under the
    // default JPA provider - see ThrottleConfig#maxAttributeLookupResults. This verifies the
    // configured ceiling is actually passed through as the combined query's maxResults: this stub
    // only matches maxResults=5, so if the resolver passed a different value, the mock would
    // return null and the resolver would NPE instead of authenticating.
    UserModel user = mockUser("user-1", "pw", true);
    when(userProvider.searchForUserStream(
            realm, Map.of("dateOfBirth", "19900101", UserModel.EXACT, "true"), 0, 5))
        .thenReturn(Stream.of(user));
    MultiAttributeCredentialResolver.ThrottleConfig throttleConfig =
        new MultiAttributeCredentialResolver.ThrottleConfig(10, 10, 60, 5);

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session,
            realm,
            List.of("dateOfBirth"),
            valuesOf("dateOfBirth", "19900101"),
            "pw",
            throttleConfig);

    assertTrue(result.authenticatedUser().isPresent());
    assertEquals(user, result.authenticatedUser().get());
  }

  @Test
  void throttleConfig_defaults_includesMaxAttributeLookupResultsDefault() {
    assertEquals(
        Integer.parseInt(Utils.MAX_ATTRIBUTE_LOOKUP_RESULTS_DEFAULT),
        MultiAttributeCredentialResolver.ThrottleConfig.defaults().maxAttributeLookupResults());
  }

  @Test
  void throttleConfig_threeArgConstructor_defaultsMaxAttributeLookupResults() {
    MultiAttributeCredentialResolver.ThrottleConfig throttleConfig =
        new MultiAttributeCredentialResolver.ThrottleConfig(10, 10, 60);

    assertEquals(
        Integer.parseInt(Utils.MAX_ATTRIBUTE_LOOKUP_RESULTS_DEFAULT),
        throttleConfig.maxAttributeLookupResults());
  }

  // ── DoS mitigation: per-tuple failure throttle ───────────────────────────

  @Test
  void tupleThrottle_maxFailuresReached_shortCircuitsWithoutUserSearch() {
    // A Stream can only be consumed once - thenAnswer (rather than thenReturn) gives each of the
    // multiple resolveAuthenticatedUser() calls below its own fresh stream.
    UserModel user = mockUser("user-1", "correct-horse", true);
    when(userProvider.searchForUserStream(
            realm,
            Map.of("nationalId", "X123", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenAnswer(invocation -> Stream.of(user));
    MultiAttributeCredentialResolver.ThrottleConfig throttleConfig =
        new MultiAttributeCredentialResolver.ThrottleConfig(10, 2, 60);

    // Two failed attempts against the same attribute-value tuple.
    authenticator.resolveAuthenticatedUser(
        session,
        realm,
        List.of("nationalId"),
        valuesOf("nationalId", "X123"),
        "wrong",
        throttleConfig);
    authenticator.resolveAuthenticatedUser(
        session,
        realm,
        List.of("nationalId"),
        valuesOf("nationalId", "X123"),
        "wrong",
        throttleConfig);

    // Third attempt: the tuple is now throttled, so it must short-circuit before any user search,
    // even with the correct password.
    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session,
            realm,
            List.of("nationalId"),
            valuesOf("nationalId", "X123"),
            "correct-horse",
            throttleConfig);

    assertTrue(result.authenticatedUser().isEmpty());
    assertTrue(result.attributableUser().isEmpty());
    verify(userProvider, times(2))
        .searchForUserStream(
            realm,
            Map.of("nationalId", "X123", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS);
  }

  @Test
  void tupleThrottle_caseVariedSubmittedValue_stillCountsTowardSameTuple() {
    // The user store matches custom attributes case-insensitively (see
    // MultiAttributeCredentialResolver#resolveAuthenticatedUser), so "X123" and "x123" resolve to
    // the same candidate. The failure throttle must treat them as the same tuple too - otherwise
    // an attacker could reset their allowance of attempts simply by varying the submitted value's
    // casing, multiplying the effective attempt budget by every case permutation of the value.
    UserModel user = mockUser("user-1", "correct-horse", true);
    when(userProvider.searchForUserStream(
            realm,
            Map.of("nationalId", "X123", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenAnswer(invocation -> Stream.of(user));
    when(userProvider.searchForUserStream(
            realm,
            Map.of("nationalId", "x123", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenAnswer(invocation -> Stream.of(user));
    MultiAttributeCredentialResolver.ThrottleConfig throttleConfig =
        new MultiAttributeCredentialResolver.ThrottleConfig(10, 2, 60);

    // Two failed attempts against the same logical value, submitted with different casing.
    authenticator.resolveAuthenticatedUser(
        session,
        realm,
        List.of("nationalId"),
        valuesOf("nationalId", "X123"),
        "wrong",
        throttleConfig);
    authenticator.resolveAuthenticatedUser(
        session,
        realm,
        List.of("nationalId"),
        valuesOf("nationalId", "x123"),
        "wrong",
        throttleConfig);

    // Third attempt: the tuple must already be throttled, so it short-circuits before any user
    // search, even though this exact casing was never submitted before.
    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session,
            realm,
            List.of("nationalId"),
            valuesOf("nationalId", "X123"),
            "correct-horse",
            throttleConfig);

    assertTrue(result.authenticatedUser().isEmpty());
    verify(userProvider, times(1))
        .searchForUserStream(
            realm,
            Map.of("nationalId", "X123", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS);
    verify(userProvider, times(1))
        .searchForUserStream(
            realm,
            Map.of("nationalId", "x123", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS);
  }

  @Test
  void tupleThrottle_windowExpires_countResets() {
    UserModel user = mockUser("user-1", "correct-horse", true);
    when(userProvider.searchForUserStream(
            realm,
            Map.of("nationalId", "X123", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenAnswer(invocation -> Stream.of(user));
    MultiAttributeCredentialResolver.ThrottleConfig throttleConfig =
        new MultiAttributeCredentialResolver.ThrottleConfig(10, 1, 60);

    authenticator.resolveAuthenticatedUser(
        session,
        realm,
        List.of("nationalId"),
        valuesOf("nationalId", "X123"),
        "wrong",
        throttleConfig);
    singleUseObjects.advanceTimeSeconds(61);

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session,
            realm,
            List.of("nationalId"),
            valuesOf("nationalId", "X123"),
            "correct-horse",
            throttleConfig);

    assertTrue(result.authenticatedUser().isPresent());
  }

  @Test
  void tupleThrottle_successClearsCounter() {
    UserModel user = mockUser("user-1", "correct-horse", true);
    when(userProvider.searchForUserStream(
            realm,
            Map.of("nationalId", "X123", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenAnswer(invocation -> Stream.of(user));
    // tupleMaxFailures=2 so the single prior failure below (count=1) doesn't itself throttle the
    // success attempt that follows - otherwise that attempt could never reach the credential
    // check needed to prove success clears the counter.
    MultiAttributeCredentialResolver.ThrottleConfig throttleConfig =
        new MultiAttributeCredentialResolver.ThrottleConfig(10, 2, 60);

    // One failure, then a success - the success must clear the counter so it doesn't carry over
    // into the next failure and prematurely throttle it.
    authenticator.resolveAuthenticatedUser(
        session,
        realm,
        List.of("nationalId"),
        valuesOf("nationalId", "X123"),
        "wrong",
        throttleConfig);
    authenticator.resolveAuthenticatedUser(
        session,
        realm,
        List.of("nationalId"),
        valuesOf("nationalId", "X123"),
        "correct-horse",
        throttleConfig);

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session,
            realm,
            List.of("nationalId"),
            valuesOf("nationalId", "X123"),
            "wrong",
            throttleConfig);

    // Not throttled (the prior failure was cleared by the success in between), so this attempt
    // reaches the real single-candidate path and gets attributed normally.
    assertTrue(result.attributableUser().isPresent());
    assertEquals(user, result.attributableUser().get());
  }

  // ── MatchPolicy: FIRST_MATCH ─────────────────────────────────────────────

  @Test
  void firstMatch_multipleCandidates_uniquePasswords_succeedsWithTheMatchingOne() {
    UserModel alice = mockUser("alice", "alice-pw", true);
    UserModel bob = mockUser("bob", "bob-pw", true);
    when(userProvider.searchForUserStream(
            realm,
            Map.of("dateOfBirth", "19900101", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenReturn(Stream.of(alice, bob));
    MultiAttributeCredentialResolver.ThrottleConfig throttleConfig =
        new MultiAttributeCredentialResolver.ThrottleConfig(10, 10, 60);

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session,
            realm,
            List.of("dateOfBirth"),
            valuesOf("dateOfBirth", "19900101"),
            "alice-pw",
            throttleConfig,
            MultiAttributeCredentialResolver.MatchPolicy.FIRST_MATCH);

    assertTrue(result.authenticatedUser().isPresent());
    assertEquals(alice, result.authenticatedUser().get());
  }

  @Test
  void firstMatch_multipleCandidates_sharedPassword_succeedsDespiteAmbiguity() {
    // The security-relevant property FIRST_MATCH trades away: alice and bob share a password, so
    // this is exactly the ambiguous case REJECT_AMBIGUOUS (the default) would fail generically -
    // see ambiguousCandidates_wrongPassword_doesNotAttributeFailure and
    // singleAttribute_multipleCandidates_passwordMatchesMoreThanOne_fails above. FIRST_MATCH
    // authenticates as whichever candidate it happens to check first instead, which is why its
    // config description warns that password uniqueness across candidates is mandatory.
    UserModel alice = mockUser("alice", "shared-pw", true);
    UserModel bob = mockUser("bob", "shared-pw", true);
    when(userProvider.searchForUserStream(
            realm,
            Map.of("dateOfBirth", "19900101", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenReturn(Stream.of(alice, bob));
    MultiAttributeCredentialResolver.ThrottleConfig throttleConfig =
        new MultiAttributeCredentialResolver.ThrottleConfig(10, 10, 60);

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session,
            realm,
            List.of("dateOfBirth"),
            valuesOf("dateOfBirth", "19900101"),
            "shared-pw",
            throttleConfig,
            MultiAttributeCredentialResolver.MatchPolicy.FIRST_MATCH);

    assertTrue(result.authenticatedUser().isPresent());
    assertTrue(List.of(alice, bob).contains(result.authenticatedUser().get()));
  }

  @Test
  void firstMatch_multipleCandidates_noneMatch_failsGenerically() {
    UserModel alice = mockUser("alice", "alice-pw", true);
    UserModel bob = mockUser("bob", "bob-pw", true);
    when(userProvider.searchForUserStream(
            realm,
            Map.of("dateOfBirth", "19900101", UserModel.EXACT, "true"),
            0,
            DEFAULT_MAX_ATTRIBUTE_LOOKUP_RESULTS))
        .thenReturn(Stream.of(alice, bob));
    MultiAttributeCredentialResolver.ThrottleConfig throttleConfig =
        new MultiAttributeCredentialResolver.ThrottleConfig(10, 10, 60);

    Resolution result =
        authenticator.resolveAuthenticatedUser(
            session,
            realm,
            List.of("dateOfBirth"),
            valuesOf("dateOfBirth", "19900101"),
            "wrong",
            throttleConfig,
            MultiAttributeCredentialResolver.MatchPolicy.FIRST_MATCH);

    assertTrue(result.authenticatedUser().isEmpty());
    assertTrue(result.attributableUser().isEmpty());
  }

  @Test
  void matchPolicy_fromString_nullOrBlank_defaultsToRejectAmbiguous() {
    assertEquals(
        MultiAttributeCredentialResolver.MatchPolicy.REJECT_AMBIGUOUS,
        MultiAttributeCredentialResolver.MatchPolicy.fromString(null));
    assertEquals(
        MultiAttributeCredentialResolver.MatchPolicy.REJECT_AMBIGUOUS,
        MultiAttributeCredentialResolver.MatchPolicy.fromString("  "));
  }

  @Test
  void matchPolicy_fromString_caseInsensitive() {
    assertEquals(
        MultiAttributeCredentialResolver.MatchPolicy.FIRST_MATCH,
        MultiAttributeCredentialResolver.MatchPolicy.fromString("first_match"));
  }

  @Test
  void matchPolicy_fromString_unknownValue_throws() {
    assertThrows(
        IllegalArgumentException.class,
        () -> MultiAttributeCredentialResolver.MatchPolicy.fromString("bogus"));
  }

  @Test
  void factory_configPropertiesIncludeMatchPolicyWithSecurityWarning() {
    MultiAttributePasswordAuthenticator factory = new MultiAttributePasswordAuthenticator();
    ProviderConfigProperty matchPolicyProp =
        factory.getConfigProperties().stream()
            .filter(prop -> Utils.MATCH_POLICY.equals(prop.getName()))
            .findFirst()
            .orElse(null);

    assertTrue(matchPolicyProp != null);
    assertEquals(
        MultiAttributeCredentialResolver.MatchPolicy.REJECT_AMBIGUOUS.name(),
        matchPolicyProp.getDefaultValue());
    assertTrue(matchPolicyProp.getOptions().contains("FIRST_MATCH"));
    // The user-facing description must actually carry the safety warning, not just exist as a
    // config option - this is the requirement this whole feature was added under.
    assertTrue(matchPolicyProp.getHelpText().toLowerCase().contains("unique"));
  }

  // ── Rendering: HTML5 input type resolved from the realm's User Profile ──

  private void mockUserProfileAttributes(UPAttribute... attributes) {
    UserProfileProvider userProfileProvider = mock(UserProfileProvider.class);
    UPConfig upConfig = new UPConfig();
    for (UPAttribute attribute : attributes) {
      upConfig.addOrReplaceAttribute(attribute);
    }
    lenient().when(session.getProvider(UserProfileProvider.class)).thenReturn(userProfileProvider);
    lenient().when(userProfileProvider.getConfiguration()).thenReturn(upConfig);
  }

  @Test
  void getRealmUserProfileAttributes_noUserProfileProvider_returnsEmptyList() {
    when(session.getProvider(UserProfileProvider.class)).thenReturn(null);

    List<UPAttribute> attributes = Utils.getRealmUserProfileAttributes(session);

    assertTrue(attributes.isEmpty());
  }

  @Test
  void getRealmUserProfileAttributes_noConfiguration_returnsEmptyList() {
    UserProfileProvider userProfileProvider = mock(UserProfileProvider.class);
    when(session.getProvider(UserProfileProvider.class)).thenReturn(userProfileProvider);
    when(userProfileProvider.getConfiguration()).thenReturn(null);

    List<UPAttribute> attributes = Utils.getRealmUserProfileAttributes(session);

    assertTrue(attributes.isEmpty());
  }

  @Test
  void getRealmUserProfileAttributes_noAttributesInConfiguration_returnsEmptyList() {
    mockUserProfileAttributes();

    List<UPAttribute> attributes = Utils.getRealmUserProfileAttributes(session);

    assertTrue(attributes.isEmpty());
  }

  @Test
  void resolveHtml5InputType_html5Prefix_stripsPrefix() {
    assertEquals(
        "date",
        Utils.resolveHtml5InputType(
            List.of(new UPAttribute("dateOfBirth", Map.of("inputType", "html5-date"))),
            "dateOfBirth"));
  }

  @Test
  void resolveHtml5InputType_unknownAttribute_fallsBackToText() {
    assertEquals("text", Utils.resolveHtml5InputType(List.of(), "nationalId"));
  }

  private void mockEmptyUserProfile() {
    UserProfileProvider userProfileProvider = mock(UserProfileProvider.class);
    UserProfile userProfile = mock(UserProfile.class);
    Attributes attributes = mock(Attributes.class);
    UPConfig configuration = new UPConfig();
    configuration.setAttributes(List.of());
    lenient().when(userProfile.getAttributes()).thenReturn(attributes);
    lenient().when(userProfileProvider.getConfiguration()).thenReturn(configuration);
    lenient()
        .when(userProfileProvider.create(eq(UserProfileContext.REGISTRATION), isNull(), isNull()))
        .thenReturn(userProfile);
    lenient().when(session.getProvider(UserProfileProvider.class)).thenReturn(userProfileProvider);
  }

  private AuthenticationFlowContext mockChallengeContext(Map<String, String> config) {
    AuthenticationFlowContext context = mock(AuthenticationFlowContext.class);
    AuthenticatorConfigModel authConfig = mock(AuthenticatorConfigModel.class);
    when(authConfig.getConfig()).thenReturn(config);
    when(context.getAuthenticatorConfig()).thenReturn(authConfig);
    lenient().when(context.getSession()).thenReturn(session);
    LoginFormsProvider form = mock(LoginFormsProvider.class);
    lenient().when(context.form()).thenReturn(form);
    lenient()
        .when(form.createForm(MultiAttributePasswordAuthenticator.FORM_FTL))
        .thenReturn(mock(Response.class));
    mockEmptyUserProfile();
    return context;
  }

  @Test
  void challenge_setsMatchAttributesAndProfileFormAttributes() {
    AuthenticationFlowContext context =
        mockChallengeContext(Map.of(Utils.MATCH_ATTRIBUTES, "dateOfBirth"));

    authenticator.challenge(context, new MultivaluedHashMap<>(), null);

    verify(context.form()).setAttribute("matchAttributes", List.of("dateOfBirth"));
    verify(context.form()).setAttribute(eq("profile"), any(LoginBean.class));
  }

  @Test
  void challenge_honorUserProfileRequiredEnabled_setsFormAttribute() {
    AuthenticationFlowContext context =
        mockChallengeContext(
            Map.of(
                Utils.MATCH_ATTRIBUTES, "dateOfBirth",
                Utils.HONOR_USER_PROFILE_REQUIRED, "true"));

    authenticator.challenge(context, new MultivaluedHashMap<>(), null);

    verify(context.form()).setAttribute("honorUserProfileRequired", true);
  }

  @Test
  void challenge_honorUserProfileRequiredNotConfigured_neverSetsFormAttribute() {
    AuthenticationFlowContext context =
        mockChallengeContext(Map.of(Utils.MATCH_ATTRIBUTES, "dateOfBirth"));

    authenticator.challenge(context, new MultivaluedHashMap<>(), null);

    verify(context.form(), never()).setAttribute(eq("honorUserProfileRequired"), any());
  }

  @Test
  void factory_configPropertiesIncludeHonorUserProfileRequiredDisabledByDefault() {
    MultiAttributePasswordAuthenticator factory = new MultiAttributePasswordAuthenticator();
    ProviderConfigProperty honorRequiredProp =
        factory.getConfigProperties().stream()
            .filter(prop -> Utils.HONOR_USER_PROFILE_REQUIRED.equals(prop.getName()))
            .findFirst()
            .orElse(null);

    assertTrue(honorRequiredProp != null);
    assertEquals(ProviderConfigProperty.BOOLEAN_TYPE, honorRequiredProp.getType());
    assertEquals("false", honorRequiredProp.getDefaultValue());
  }

  // ── optionalAttributes: which matchAttributes the realm's User Profile does NOT require ──

  @Test
  void optionalAttributes_declaredRequiredAttribute_staysMandatory() {
    AuthenticationFlowContext context =
        mockChallengeContext(
            Map.of(
                Utils.MATCH_ATTRIBUTES, "dateOfBirth",
                Utils.HONOR_USER_PROFILE_REQUIRED, "true"));
    mockUserProfileForOptionalAttributes(Map.of("dateOfBirth", true));

    Set<String> optional = authenticator.optionalAttributes(context, List.of("dateOfBirth"));

    assertTrue(optional.isEmpty());
  }

  @Test
  void optionalAttributes_declaredNonRequiredAttribute_becomesOptional() {
    AuthenticationFlowContext context =
        mockChallengeContext(
            Map.of(
                Utils.MATCH_ATTRIBUTES, "nationalId",
                Utils.HONOR_USER_PROFILE_REQUIRED, "true"));
    mockUserProfileForOptionalAttributes(Map.of("nationalId", false));

    Set<String> optional = authenticator.optionalAttributes(context, List.of("nationalId"));

    assertEquals(Set.of("nationalId"), optional);
  }

  @Test
  void optionalAttributes_undeclaredAttribute_staysMandatory() {
    // "voterId" has no User Profile entry at all (e.g. a typo, or a non-User-Profile field) -
    // the conservative default is to leave it mandatory rather than silently widen the match.
    AuthenticationFlowContext context =
        mockChallengeContext(
            Map.of(
                Utils.MATCH_ATTRIBUTES, "voterId",
                Utils.HONOR_USER_PROFILE_REQUIRED, "true"));
    mockUserProfileForOptionalAttributes(Map.of());

    Set<String> optional = authenticator.optionalAttributes(context, List.of("voterId"));

    assertTrue(optional.isEmpty());
  }

  @Test
  void optionalAttributes_notEnabled_returnsEmptySetRegardlessOfUserProfile() {
    AuthenticationFlowContext context =
        mockChallengeContext(Map.of(Utils.MATCH_ATTRIBUTES, "nationalId"));
    mockUserProfileForOptionalAttributes(Map.of("nationalId", false));

    Set<String> optional = authenticator.optionalAttributes(context, List.of("nationalId"));

    assertTrue(optional.isEmpty());
  }

  private void mockUserProfileForOptionalAttributes(Map<String, Boolean> requiredByName) {
    UserProfileProvider userProfileProvider = mock(UserProfileProvider.class);
    UserProfile userProfile = mock(UserProfile.class);
    Attributes attributes = mock(Attributes.class);

    for (Map.Entry<String, Boolean> entry : requiredByName.entrySet()) {
      String name = entry.getKey();
      AttributeMetadata metadata = mock(AttributeMetadata.class);
      lenient().when(metadata.getName()).thenReturn(name);
      lenient().when(metadata.getAttributeDisplayName()).thenReturn(name);
      lenient().when(metadata.getAnnotations()).thenReturn(Map.of());
      lenient().when(metadata.getValidators()).thenReturn(List.of());
      lenient().when(attributes.getMetadata(name)).thenReturn(metadata);
      lenient().when(attributes.isRequired(name)).thenReturn(entry.getValue());
    }
    lenient().when(userProfile.getAttributes()).thenReturn(attributes);
    lenient()
        .when(userProfileProvider.create(eq(UserProfileContext.REGISTRATION), isNull(), isNull()))
        .thenReturn(userProfile);
    lenient().when(session.getProvider(UserProfileProvider.class)).thenReturn(userProfileProvider);
  }

  // ── Date normalization (collectSubmittedValues) ─────────────────────────

  @Test
  void collectSubmittedValues_dateTypedAttribute_normalizesFromHtml5Format() {
    mockUserProfileAttributes(new UPAttribute("dateOfBirth", Map.of("inputType", "html5-date")));
    MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
    formData.add("dateOfBirth", "1990-01-05");

    Map<String, String> submitted =
        authenticator.collectSubmittedValues(session, List.of("dateOfBirth"), formData);

    assertEquals("1990-01-05", submitted.get("dateOfBirth"));
  }

  @Test
  void collectSubmittedValues_nonDateAttribute_passesThroughUnchanged() {
    mockUserProfileAttributes(new UPAttribute("nationalId", Map.of("inputType", "text")));
    MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
    formData.add("nationalId", "X123");

    Map<String, String> submitted =
        authenticator.collectSubmittedValues(session, List.of("nationalId"), formData);

    assertEquals("X123", submitted.get("nationalId"));
  }

  @Test
  void collectSubmittedValues_dateTypedAttribute_missingValue_staysNull() {
    mockUserProfileAttributes(new UPAttribute("dateOfBirth", Map.of("inputType", "html5-date")));
    MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();

    Map<String, String> submitted =
        authenticator.collectSubmittedValues(session, List.of("dateOfBirth"), formData);

    assertNull(submitted.get("dateOfBirth"));
  }

  // ── Browser authentication-session user lifecycle ───────────────────────

  @Test
  void action_clearsStaleUserBeforeSettingResolvedUser() {
    AuthenticationFlowContext context = mockActionContext();
    UserModel resolvedUser = mock(UserModel.class);
    MultiAttributePasswordAuthenticator actionAuthenticator =
        actionAuthenticator(Resolution.success(resolvedUser), mock(Response.class));

    actionAuthenticator.action(context);

    InOrder inOrder = inOrder(context);
    inOrder.verify(context).clearUser();
    inOrder.verify(context).setUser(resolvedUser);
    inOrder.verify(context).success();
  }

  @Test
  void authenticate_terminatePolicyWorksWithBothCredentialPolicies() {
    for (String credentialPolicy : List.of("PASSWORD", "SECRET_ATTRIBUTE")) {
      AuthenticationFlowContext context = mock(AuthenticationFlowContext.class);
      AuthenticatorConfigModel authConfig = new AuthenticatorConfigModel();
      authConfig.setConfig(
          Map.of(
              "existingUserSessionPolicy",
              "TERMINATE_BEFORE_LOGIN",
              EncryptedAttributeCredential.POLICY,
              credentialPolicy));
      lenient().when(context.getAuthenticatorConfig()).thenReturn(authConfig);
      UserSessionModel existingSession = mock(UserSessionModel.class);
      UserSessionProvider userSessions = mock(UserSessionProvider.class);
      AuthenticationSessionModel authenticationSession = mock(AuthenticationSessionModel.class);
      RootAuthenticationSessionModel rootSession = mock(RootAuthenticationSessionModel.class);
      lenient().when(context.getAuthenticationSession()).thenReturn(authenticationSession);
      lenient().when(context.getSession()).thenReturn(session);
      lenient().when(context.getRealm()).thenReturn(realm);
      lenient().when(authenticationSession.getParentSession()).thenReturn(rootSession);
      lenient().when(rootSession.getId()).thenReturn("browser-session");
      lenient().when(session.sessions()).thenReturn(userSessions);
      lenient()
          .when(userSessions.getUserSession(realm, "browser-session"))
          .thenReturn(existingSession);
      Response challengeResponse = mock(Response.class);

      challengeAuthenticator(challengeResponse).authenticate(context);

      InOrder inOrder = inOrder(context, userSessions);
      inOrder.verify(userSessions).removeUserSession(realm, existingSession);
      inOrder.verify(context).challenge(challengeResponse);
    }
  }

  @Test
  void authenticate_defaultPolicyKeepsExistingSession() {
    AuthenticationFlowContext context = mock(AuthenticationFlowContext.class);
    AuthenticatorConfigModel authConfig = new AuthenticatorConfigModel();
    authConfig.setConfig(Map.of());
    lenient().when(context.getAuthenticatorConfig()).thenReturn(authConfig);
    Response challengeResponse = mock(Response.class);

    challengeAuthenticator(challengeResponse).authenticate(context);

    verify(context, never()).getAuthenticationSession();
    verify(context).challenge(challengeResponse);
  }

  @Test
  void factory_configPropertiesIncludeSessionAndCredentialPolicies() {
    var properties = authenticator.getConfigProperties();
    ProviderConfigProperty policyProperty =
        properties.stream()
            .filter(property -> "existingUserSessionPolicy".equals(property.getName()))
            .findFirst()
            .orElse(null);
    assertTrue(policyProperty != null);
    assertEquals(ProviderConfigProperty.LIST_TYPE, policyProperty.getType());
    assertEquals("KEEP", policyProperty.getDefaultValue());
    assertEquals(List.of("KEEP", "TERMINATE_BEFORE_LOGIN"), policyProperty.getOptions());
    assertTrue(
        properties.stream().anyMatch(p -> EncryptedAttributeCredential.POLICY.equals(p.getName())));
    assertTrue(
        properties.stream()
            .anyMatch(p -> EncryptedAttributeCredential.ATTRIBUTE.equals(p.getName())));
  }

  @Test
  void action_failureClearsAttributedUserAfterSignalingFailure() {
    AuthenticationFlowContext context = mockActionContext();
    UserModel attributableUser = mock(UserModel.class);
    Response challengeResponse = mock(Response.class);
    MultiAttributePasswordAuthenticator actionAuthenticator =
        actionAuthenticator(Resolution.failureAttributedTo(attributableUser), challengeResponse);

    actionAuthenticator.action(context);

    InOrder inOrder = inOrder(context);
    inOrder.verify(context).clearUser();
    inOrder.verify(context).setUser(attributableUser);
    inOrder
        .verify(context)
        .failureChallenge(
            org.keycloak.authentication.AuthenticationFlowError.INVALID_CREDENTIALS,
            challengeResponse);
    inOrder.verify(context).clearUser();
  }

  @Test
  void action_lockedOutStatesRenderGenericErrorWithoutHidingInternalEventReason() {
    for (LockoutState state : List.of(LockoutState.TEMPORARY, LockoutState.PERMANENT)) {
      AuthenticationFlowContext context = mockActionContext();
      EventBuilder event = context.getEvent();
      UserModel attributableUser = mock(UserModel.class);
      Response challengeResponse = mock(Response.class);
      java.util.concurrent.atomic.AtomicReference<String> renderedError =
          new java.util.concurrent.atomic.AtomicReference<>();
      MultiAttributePasswordAuthenticator actionAuthenticator =
          actionAuthenticator(
              Resolution.lockedOut(attributableUser, state), challengeResponse, renderedError);

      actionAuthenticator.action(context);

      assertEquals(
          MultiAttributePasswordAuthenticator.INVALID_CREDENTIALS_MESSAGE, renderedError.get());
      verify(event)
          .error(
              state == LockoutState.PERMANENT
                  ? org.keycloak.events.Errors.USER_DISABLED
                  : org.keycloak.events.Errors.USER_TEMPORARILY_DISABLED);
      verify(context).forceChallenge(challengeResponse);
      verify(context, never()).failureChallenge(any(), any());
    }
  }

  @Test
  void action_passesNarrowedMatchAttributesFromOptionalAttributesToResolver() {
    AuthenticationFlowContext context = mockActionContext();
    AuthenticatorConfigModel authConfig = context.getAuthenticatorConfig();
    when(authConfig.getConfig())
        .thenReturn(Map.of(Utils.MATCH_ATTRIBUTES, "dateOfBirth##nationalId"));
    java.util.concurrent.atomic.AtomicReference<List<String>> captured =
        new java.util.concurrent.atomic.AtomicReference<>();
    MultiAttributePasswordAuthenticator actionAuthenticator =
        new MultiAttributePasswordAuthenticator() {
          @Override
          protected Map<String, String> collectSubmittedValues(
              KeycloakSession session,
              List<String> matchAttributes,
              MultivaluedMap<String, String> formData) {
            // nationalId left blank - only dateOfBirth has a submitted value.
            return valuesOf("dateOfBirth", "19900101");
          }

          @Override
          protected Set<String> optionalAttributes(
              AuthenticationFlowContext context, List<String> matchAttributes) {
            return Set.of("nationalId");
          }

          @Override
          protected Resolution resolveAuthenticatedUser(
              KeycloakSession session,
              RealmModel realm,
              List<String> matchAttributes,
              Map<String, String> submittedValues,
              String password,
              MultiAttributeCredentialResolver.ThrottleConfig throttleConfig,
              MultiAttributeCredentialResolver.MatchPolicy matchPolicy) {
            captured.set(matchAttributes);
            return Resolution.failure();
          }

          @Override
          protected Response challenge(
              AuthenticationFlowContext context,
              MultivaluedMap<String, String> formData,
              String error) {
            return mock(Response.class);
          }
        };

    actionAuthenticator.action(context);

    assertEquals(List.of("dateOfBirth"), captured.get());
  }

  private AuthenticationFlowContext mockActionContext() {
    AuthenticationFlowContext context = mock(AuthenticationFlowContext.class);
    HttpRequest request = mock(HttpRequest.class);
    AuthenticatorConfigModel authConfig = mock(AuthenticatorConfigModel.class);
    MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
    formData.add(MultiAttributePasswordAuthenticator.FIELD_PASSWORD, "pin");
    when(context.getHttpRequest()).thenReturn(request);
    when(request.getDecodedFormParameters()).thenReturn(formData);
    when(context.getAuthenticatorConfig()).thenReturn(authConfig);
    when(authConfig.getConfig()).thenReturn(Map.of());
    when(context.getSession()).thenReturn(session);
    when(context.getRealm()).thenReturn(realm);
    lenient().when(context.getEvent()).thenReturn(mock(EventBuilder.class));
    return context;
  }

  private MultiAttributePasswordAuthenticator challengeAuthenticator(Response challengeResponse) {
    return new MultiAttributePasswordAuthenticator() {
      @Override
      protected Response challenge(
          AuthenticationFlowContext context,
          MultivaluedMap<String, String> formData,
          String error) {
        return challengeResponse;
      }
    };
  }

  private MultiAttributePasswordAuthenticator actionAuthenticator(
      Resolution resolution, Response challengeResponse) {
    return actionAuthenticator(resolution, challengeResponse, null);
  }

  private MultiAttributePasswordAuthenticator actionAuthenticator(
      Resolution resolution,
      Response challengeResponse,
      java.util.concurrent.atomic.AtomicReference<String> renderedError) {
    return new MultiAttributePasswordAuthenticator() {
      @Override
      protected Map<String, String> collectSubmittedValues(
          KeycloakSession session,
          List<String> matchAttributes,
          MultivaluedMap<String, String> formData) {
        return Map.of();
      }

      @Override
      protected Resolution resolveAuthenticatedUser(
          KeycloakSession session,
          RealmModel realm,
          List<String> matchAttributes,
          Map<String, String> submittedValues,
          String password,
          MultiAttributeCredentialResolver.ThrottleConfig throttleConfig,
          MultiAttributeCredentialResolver.MatchPolicy matchPolicy) {
        return resolution;
      }

      @Override
      protected Response challenge(
          AuthenticationFlowContext context,
          MultivaluedMap<String, String> formData,
          String error) {
        if (renderedError != null) {
          renderedError.set(error);
        }
        return challengeResponse;
      }
    };
  }

  // ── Utils.normalizeDate ──────────────────────────────────────────────────

  @Test
  void normalizeDate_isoFormat_isUnchanged() {
    assertEquals("1990-01-05", Utils.normalizeDate("1990-01-05", "YYYY-MM-DD"));
  }

  @Test
  void normalizeDate_mmddyyyy_reordersToCanonical() {
    assertEquals("1990-01-05", Utils.normalizeDate("01051990", "MMDDYYYY"));
  }

  @Test
  void normalizeDate_yyyymmddDigitsOnly_reordersToCanonical() {
    assertEquals("1990-01-05", Utils.normalizeDate("19900105", "YYYYMMDD"));
  }

  @Test
  void normalizeDate_digitCountMismatch_returnsRawValueUnchanged() {
    // Can't confidently reorder if the digit count doesn't match the declared format - fall back
    // to returning the original value unchanged rather than guessing.
    assertEquals("1-2-3", Utils.normalizeDate("1-2-3", "YYYY-MM-DD"));
  }

  @Test
  void normalizeDate_null_returnsNull() {
    assertNull(Utils.normalizeDate(null, "YYYY-MM-DD"));
  }

  // ── Factory metadata ────────────────────────────────────────────────────

  @Test
  void factory_providerId() {
    MultiAttributePasswordAuthenticator factory = new MultiAttributePasswordAuthenticator();
    assertEquals("multi-attribute-password-form", factory.getId());
  }

  @Test
  void factory_configPropertiesIncludeMatchAttributes() {
    MultiAttributePasswordAuthenticator factory = new MultiAttributePasswordAuthenticator();
    boolean hasMatchAttributes =
        factory.getConfigProperties().stream()
            .anyMatch(prop -> Utils.MATCH_ATTRIBUTES.equals(prop.getName()));
    assertTrue(hasMatchAttributes);
  }

  @Test
  void factory_configPropertiesIncludeDosMitigationOptions() {
    MultiAttributePasswordAuthenticator factory = new MultiAttributePasswordAuthenticator();
    List<String> names =
        factory.getConfigProperties().stream().map(prop -> prop.getName()).toList();
    assertTrue(names.contains(Utils.MAX_CANDIDATES));
    assertTrue(names.contains(Utils.TUPLE_MAX_FAILURES));
    assertTrue(names.contains(Utils.TUPLE_FAILURE_WINDOW_SECONDS));
    assertTrue(names.contains(Utils.MAX_ATTRIBUTE_LOOKUP_RESULTS));
  }

  @Test
  void factory_isConfigurable() {
    MultiAttributePasswordAuthenticator factory = new MultiAttributePasswordAuthenticator();
    assertTrue(factory.isConfigurable());
    assertFalse(factory.isUserSetupAllowed());
  }
}
