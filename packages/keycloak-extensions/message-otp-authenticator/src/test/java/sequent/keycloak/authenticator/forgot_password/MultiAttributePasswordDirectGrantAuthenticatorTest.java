// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.lenient;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.stream.Stream;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.credential.CredentialInput;
import org.keycloak.events.EventBuilder;
import org.keycloak.http.HttpRequest;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.SubjectCredentialManager;
import org.keycloak.models.UserCredentialModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.UserProvider;
import org.keycloak.provider.ProviderConfigProperty;
import org.mockito.Mock;
import org.mockito.junit.jupiter.MockitoExtension;

@ExtendWith(MockitoExtension.class)
class MultiAttributePasswordDirectGrantAuthenticatorTest {

  private MultiAttributePasswordDirectGrantAuthenticator authenticator;

  @Mock private AuthenticationFlowContext context;
  @Mock private KeycloakSession session;
  @Mock private RealmModel realm;
  @Mock private UserProvider userProvider;
  @Mock private AuthenticatorConfigModel authConfig;
  @Mock private HttpRequest httpRequest;
  @Mock private EventBuilder event;

  @BeforeEach
  void setUp() {
    authenticator = new MultiAttributePasswordDirectGrantAuthenticator();
    lenient().when(session.users()).thenReturn(userProvider);
    lenient().when(context.getSession()).thenReturn(session);
    lenient().when(context.getRealm()).thenReturn(realm);
    lenient().when(context.getHttpRequest()).thenReturn(httpRequest);
    lenient().when(context.getAuthenticatorConfig()).thenReturn(authConfig);
    lenient().when(context.getEvent()).thenReturn(event);
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

  private void formParams(String... keyValuePairs) {
    MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
    for (int i = 0; i < keyValuePairs.length; i += 2) {
      formData.add(keyValuePairs[i], keyValuePairs[i + 1]);
    }
    lenient().when(httpRequest.getDecodedFormParameters()).thenReturn(formData);
  }

  private void ivrConfig(String field, String maxDigits, String kind, String mapsTo) {
    Map<String, String> config = new HashMap<>();
    config.put("field", field);
    config.put("max_digits", maxDigits);
    config.put("kind", kind);
    config.put("maps_to", mapsTo);
    lenient().when(authConfig.getConfig()).thenReturn(config);
  }

  // ── Happy path ───────────────────────────────────────────────────────────

  @Test
  void dobAndPin_uniqueMatch_correctPin_succeeds() {
    ivrConfig("dob##pin", "8##8", "identifier##secret", "dob##password");
    formParams("dob", "19900101", "password", "1234");
    UserModel user = mockUser("user-1", "1234", true);
    when(userProvider.searchForUserByUserAttributeStream(realm, "dob", "19900101"))
        .thenReturn(Stream.of(user));

    authenticator.authenticate(context);

    verify(context).setUser(user);
    verify(context).success();
    verify(context, never()).failure(any());
  }

  @Test
  void dobAndPin_multipleCandidates_pinDisambiguates_succeeds() {
    ivrConfig("dob##pin", "8##8", "identifier##secret", "dob##password");
    formParams("dob", "19900101", "password", "bob-pin");
    UserModel alice = mockUser("alice", "alice-pin", true);
    UserModel bob = mockUser("bob", "bob-pin", true);
    when(userProvider.searchForUserByUserAttributeStream(realm, "dob", "19900101"))
        .thenReturn(Stream.of(alice, bob));

    authenticator.authenticate(context);

    verify(context).setUser(bob);
    verify(context).success();
  }

  @Test
  void dobAndPin_wrongPin_fails() {
    ivrConfig("dob##pin", "8##8", "identifier##secret", "dob##password");
    formParams("dob", "19900101", "password", "wrong");
    UserModel user = mockUser("user-1", "1234", true);
    when(userProvider.searchForUserByUserAttributeStream(realm, "dob", "19900101"))
        .thenReturn(Stream.of(user));

    authenticator.authenticate(context);

    verify(context, never()).setUser(any());
    verify(context, never()).success();
    verify(context).failure(any(), any());
  }

  @Test
  void dobAndPin_noCandidates_fails() {
    ivrConfig("dob##pin", "8##8", "identifier##secret", "dob##password");
    formParams("dob", "19900101", "password", "1234");
    when(userProvider.searchForUserByUserAttributeStream(realm, "dob", "19900101"))
        .thenReturn(Stream.empty());

    authenticator.authenticate(context);

    verify(context, never()).setUser(any());
    verify(context).failure(any(), any());
  }

  // ── Misconfiguration ─────────────────────────────────────────────────────

  @Test
  void missingSecretKind_fails() {
    // Only "identifier" entries, no "secret" - misconfigured, must not silently succeed.
    ivrConfig("dob", "8", "identifier", "dob");
    formParams("dob", "19900101");

    authenticator.authenticate(context);

    verify(context, never()).setUser(any());
    verify(context).failure(any(), any());
  }

  @Test
  void mismatchedMapsToAndKindCounts_fails() {
    Map<String, String> config = new HashMap<>();
    config.put("field", "dob##pin");
    config.put("max_digits", "8##8");
    config.put("kind", "identifier"); // only one value for two fields
    config.put("maps_to", "dob##password");
    lenient().when(authConfig.getConfig()).thenReturn(config);
    formParams("dob", "19900101", "password", "1234");

    authenticator.authenticate(context);

    verify(context, never()).setUser(any());
    verify(context).failure(any(), any());
  }

  // ── Factory metadata ─────────────────────────────────────────────────────

  @Test
  void factory_providerId() {
    assertEquals("multi-attribute-password-direct-grant", authenticator.getId());
  }

  @Test
  void factory_requiresUserFalse() {
    assertTrue(!authenticator.requiresUser());
  }

  @Test
  void factory_configPropertiesDeclareIvrKeys() {
    List<ProviderConfigProperty> props = authenticator.getConfigProperties();
    List<String> names = props.stream().map(ProviderConfigProperty::getName).toList();
    assertTrue(names.containsAll(List.of("field", "max_digits", "kind", "maps_to", "prompt_key")));
  }
}
