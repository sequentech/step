// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.lenient;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.stream.Stream;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.keycloak.credential.CredentialInput;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.SubjectCredentialManager;
import org.keycloak.models.UserCredentialModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.UserProvider;
import org.keycloak.representations.userprofile.config.UPAttribute;
import org.keycloak.representations.userprofile.config.UPConfig;
import org.keycloak.userprofile.UserProfileProvider;
import org.mockito.Mock;
import org.mockito.junit.jupiter.MockitoExtension;

@ExtendWith(MockitoExtension.class)
class MultiAttributePasswordAuthenticatorTest {

  private MultiAttributePasswordAuthenticator authenticator;

  @Mock private KeycloakSession session;
  @Mock private RealmModel realm;
  @Mock private UserProvider userProvider;

  @BeforeEach
  void setUp() {
    authenticator = new MultiAttributePasswordAuthenticator();
    lenient().when(session.users()).thenReturn(userProvider);
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
    when(userProvider.searchForUserByUserAttributeStream(realm, "nationalId", "X123"))
        .thenReturn(Stream.of(user));

    Optional<UserModel> result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("nationalId"), valuesOf("nationalId", "X123"), "correct-horse");

    assertTrue(result.isPresent());
    assertEquals(user, result.get());
  }

  @Test
  void singleAttribute_uniqueMatch_wrongPassword_fails() {
    UserModel user = mockUser("user-1", "correct-horse", true);
    when(userProvider.searchForUserByUserAttributeStream(realm, "nationalId", "X123"))
        .thenReturn(Stream.of(user));

    Optional<UserModel> result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("nationalId"), valuesOf("nationalId", "X123"), "wrong");

    assertTrue(result.isEmpty());
  }

  @Test
  void singleAttribute_disabledUser_fails() {
    UserModel user = mockUser("user-1", "correct-horse", false);
    when(userProvider.searchForUserByUserAttributeStream(realm, "nationalId", "X123"))
        .thenReturn(Stream.of(user));

    Optional<UserModel> result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("nationalId"), valuesOf("nationalId", "X123"), "correct-horse");

    assertTrue(result.isEmpty());
  }

  // ── Multiple attributes intersect to one candidate ──────────────────────

  @Test
  void twoAttributes_intersectionNarrowsToOne_succeeds() {
    UserModel alice = mockUser("alice", "alice-pw", true);
    UserModel bob = mockUser("bob", "bob-pw", true);

    when(userProvider.searchForUserByUserAttributeStream(realm, "dateOfBirth", "19900101"))
        .thenReturn(Stream.of(alice, bob));
    when(userProvider.searchForUserByUserAttributeStream(realm, "nationalId", "ALICE-ID"))
        .thenReturn(Stream.of(alice));

    Optional<UserModel> result =
        authenticator.resolveAuthenticatedUser(
            session,
            realm,
            List.of("dateOfBirth", "nationalId"),
            valuesOf("dateOfBirth", "19900101", "nationalId", "ALICE-ID"),
            "alice-pw");

    assertTrue(result.isPresent());
    assertEquals(alice, result.get());
  }

  // ── DOB-not-unique-alone case: multiple candidates, password disambiguates ──

  @Test
  void singleAttribute_multipleCandidates_passwordUniquelyMatchesOne_succeeds() {
    UserModel alice = mockUser("alice", "alice-pw", true);
    UserModel bob = mockUser("bob", "bob-pw", true);
    when(userProvider.searchForUserByUserAttributeStream(realm, "dateOfBirth", "19900101"))
        .thenReturn(Stream.of(alice, bob));

    Optional<UserModel> result =
        authenticator.resolveAuthenticatedUser(
            session,
            realm,
            List.of("dateOfBirth"),
            valuesOf("dateOfBirth", "19900101"),
            "alice-pw");

    assertTrue(result.isPresent());
    assertEquals(alice, result.get());
  }

  @Test
  void singleAttribute_multipleCandidates_passwordMatchesNone_fails() {
    UserModel alice = mockUser("alice", "alice-pw", true);
    UserModel bob = mockUser("bob", "bob-pw", true);
    when(userProvider.searchForUserByUserAttributeStream(realm, "dateOfBirth", "19900101"))
        .thenReturn(Stream.of(alice, bob));

    Optional<UserModel> result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("dateOfBirth"), valuesOf("dateOfBirth", "19900101"), "wrong");

    assertTrue(result.isEmpty());
  }

  @Test
  void singleAttribute_multipleCandidates_passwordMatchesMoreThanOne_fails() {
    UserModel alice = mockUser("alice", "shared-pw", true);
    UserModel bob = mockUser("bob", "shared-pw", true);
    when(userProvider.searchForUserByUserAttributeStream(realm, "dateOfBirth", "19900101"))
        .thenReturn(Stream.of(alice, bob));

    Optional<UserModel> result =
        authenticator.resolveAuthenticatedUser(
            session,
            realm,
            List.of("dateOfBirth"),
            valuesOf("dateOfBirth", "19900101"),
            "shared-pw");

    assertTrue(result.isEmpty());
  }

  // ── Zero candidates ──────────────────────────────────────────────────────

  @Test
  void noCandidates_fails() {
    when(userProvider.searchForUserByUserAttributeStream(realm, "nationalId", "unknown"))
        .thenReturn(Stream.empty());

    Optional<UserModel> result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("nationalId"), valuesOf("nationalId", "unknown"), "pw");

    assertTrue(result.isEmpty());
  }

  @Test
  void twoAttributes_disjointCandidateSets_fails() {
    UserModel alice = mockUser("alice", "alice-pw", true);
    UserModel bob = mockUser("bob", "bob-pw", true);
    when(userProvider.searchForUserByUserAttributeStream(realm, "dateOfBirth", "19900101"))
        .thenReturn(Stream.of(alice));
    when(userProvider.searchForUserByUserAttributeStream(realm, "nationalId", "BOB-ID"))
        .thenReturn(Stream.of(bob));

    Optional<UserModel> result =
        authenticator.resolveAuthenticatedUser(
            session,
            realm,
            List.of("dateOfBirth", "nationalId"),
            valuesOf("dateOfBirth", "19900101", "nationalId", "BOB-ID"),
            "alice-pw");

    assertTrue(result.isEmpty());
  }

  // ── Missing / blank submitted values ────────────────────────────────────

  @Test
  void blankSubmittedValue_failsWithoutLookup() {
    Optional<UserModel> result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("nationalId"), valuesOf("nationalId", "  "), "pw");

    assertTrue(result.isEmpty());
  }

  @Test
  void missingSubmittedValue_failsWithoutLookup() {
    Optional<UserModel> result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("nationalId"), Map.of(), "pw");

    assertTrue(result.isEmpty());
  }

  @Test
  void blankPassword_failsWithoutLookup() {
    Optional<UserModel> result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("nationalId"), valuesOf("nationalId", "X123"), " ");

    assertTrue(result.isEmpty());
  }

  @Test
  void noMatchAttributesConfigured_fails() {
    Optional<UserModel> result =
        authenticator.resolveAuthenticatedUser(session, realm, List.of(), Map.of(), "pw");

    assertTrue(result.isEmpty());
  }

  // ── username / email special-casing ─────────────────────────────────────

  @Test
  void usernameAttribute_usesGetUserByUsername() {
    UserModel user = mockUser("user-1", "pw", true);
    when(userProvider.getUserByUsername(realm, "voter1")).thenReturn(user);

    Optional<UserModel> result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("username"), valuesOf("username", "voter1"), "pw");

    assertTrue(result.isPresent());
    assertEquals(user, result.get());
  }

  @Test
  void emailAttribute_usesGetUserByEmail() {
    UserModel user = mockUser("user-1", "pw", true);
    when(userProvider.getUserByEmail(realm, "voter1@example.com")).thenReturn(user);

    Optional<UserModel> result =
        authenticator.resolveAuthenticatedUser(
            session, realm, List.of("email"), valuesOf("email", "voter1@example.com"), "pw");

    assertTrue(result.isPresent());
    assertEquals(user, result.get());
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
  void buildAttributeFields_html5DateAnnotation_resolvesToDateInputType() {
    mockUserProfileAttributes(new UPAttribute("dateOfBirth", Map.of("inputType", "html5-date")));

    List<Map<String, String>> fields =
        authenticator.buildAttributeFields(session, List.of("dateOfBirth"));

    assertEquals(List.of(Map.of("name", "dateOfBirth", "type", "date")), fields);
  }

  @Test
  void buildAttributeFields_nonHtml5InputType_fallsBackToText() {
    mockUserProfileAttributes(new UPAttribute("country", Map.of("inputType", "select")));

    List<Map<String, String>> fields =
        authenticator.buildAttributeFields(session, List.of("country"));

    assertEquals(List.of(Map.of("name", "country", "type", "text")), fields);
  }

  @Test
  void buildAttributeFields_noUserProfileEntry_fallsBackToText() {
    mockUserProfileAttributes();

    List<Map<String, String>> fields =
        authenticator.buildAttributeFields(session, List.of("nationalId"));

    assertEquals(List.of(Map.of("name", "nationalId", "type", "text")), fields);
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
  void factory_isConfigurable() {
    MultiAttributePasswordAuthenticator factory = new MultiAttributePasswordAuthenticator();
    assertTrue(factory.isConfigurable());
    assertFalse(factory.isUserSetupAllowed());
  }
}
