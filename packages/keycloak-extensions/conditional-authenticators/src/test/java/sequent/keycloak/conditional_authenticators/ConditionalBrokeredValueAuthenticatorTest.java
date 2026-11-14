// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import java.io.IOException;
import java.io.UncheckedIOException;
import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.authenticators.broker.AbstractIdpAuthenticator;
import org.keycloak.authentication.authenticators.broker.util.PostBrokerLoginConstants;
import org.keycloak.authentication.authenticators.broker.util.SerializedBrokeredIdentityContext;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.util.JsonSerialization;

class ConditionalBrokeredValueAuthenticatorTest {

  private static final String UUID_V8 =
      "^[0-9a-f]{8}-[0-9a-f]{4}-8[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$";
  private static final String VOTER_ID = "3f2b8c1a-9d4e-8a7b-b1c2-0d9e8f7a6b5c";

  private final ConditionalBrokeredValueAuthenticator condition =
      ConditionalBrokeredValueAuthenticator.SINGLETON;
  private AuthenticationFlowContext context;
  private AuthenticationSessionModel authSession;

  @BeforeEach
  void setUp() {
    context = mock(AuthenticationFlowContext.class);
    authSession = mock(AuthenticationSessionModel.class);
    when(context.getAuthenticationSession()).thenReturn(authSession);
    when(authSession.getRealm()).thenReturn(mock(RealmModel.class));
  }

  private void configure(String... pairs) {
    Map<String, String> config = new HashMap<>();
    for (int i = 0; i < pairs.length; i += 2) {
      config.put(pairs[i], pairs[i + 1]);
    }
    AuthenticatorConfigModel model = new AuthenticatorConfigModel();
    model.setAlias("check");
    model.setConfig(config);
    when(context.getAuthenticatorConfig()).thenReturn(model);
  }

  /** Stores a brokered identity in the login session as JSON, the way Keycloak does. */
  private void brokered(String note, String username, String... attributes) {
    SerializedBrokeredIdentityContext identity = new SerializedBrokeredIdentityContext();
    identity.setId("broker-id");
    identity.setIdentityProviderId("idp");
    identity.setModelUsername(username);
    for (int i = 0; i < attributes.length; i += 2) {
      identity.setSingleAttribute(attributes[i], attributes[i + 1]);
    }
    String json;
    try {
      json = JsonSerialization.writeValueAsString(identity);
    } catch (IOException e) {
      throw new UncheckedIOException(e);
    }
    when(authSession.getAuthNote(note)).thenReturn(json);
  }

  private void usernameCheck(boolean negate) {
    configure(
        ConditionalBrokeredValueAuthenticatorFactory.CONF_TARGET,
        "USERNAME",
        ConditionalBrokeredValueAuthenticatorFactory.CONF_REGEX,
        UUID_V8,
        ConditionalBrokeredValueAuthenticatorFactory.CONF_NEGATE,
        String.valueOf(negate));
  }

  private void areaCheck(boolean negate) {
    configure(
        ConditionalBrokeredValueAuthenticatorFactory.CONF_ATTRIBUTE,
        "area-id",
        ConditionalBrokeredValueAuthenticatorFactory.CONF_NEGATE,
        String.valueOf(negate));
  }

  @Test
  void usernameMatchingRegexMatchesBeforeUserExists() {
    brokered(AbstractIdpAuthenticator.BROKERED_CONTEXT_NOTE, VOTER_ID);
    usernameCheck(false);
    assertTrue(condition.matchCondition(context));
    usernameCheck(true);
    assertFalse(condition.matchCondition(context));
  }

  @Test
  void negatedCheckMatchesInvalidUsername() {
    brokered(
        AbstractIdpAuthenticator.BROKERED_CONTEXT_NOTE, "3f2b8c1a-9d4e-4a7b-b1c2-0d9e8f7a6b5c");
    usernameCheck(true);
    assertTrue(condition.matchCondition(context));
  }

  @Test
  void unresolvedUsernameCountsAsEmptyEvenIfTheFallbackMatches() throws IOException {
    brokered(AbstractIdpAuthenticator.BROKERED_CONTEXT_NOTE, VOTER_ID);
    String json = authSession.getAuthNote(AbstractIdpAuthenticator.BROKERED_CONTEXT_NOTE);
    SerializedBrokeredIdentityContext identity =
        JsonSerialization.readValue(json, SerializedBrokeredIdentityContext.class);
    identity
        .getContextData()
        .put(
            ConditionalBrokeredValueAuthenticator.USERNAME_UNRESOLVED_KEY,
            SerializedBrokeredIdentityContext.ContextDataEntry.create(
                String.class.getName(), "true"));
    when(authSession.getAuthNote(AbstractIdpAuthenticator.BROKERED_CONTEXT_NOTE))
        .thenReturn(JsonSerialization.writeValueAsString(identity));
    usernameCheck(false);
    assertFalse(condition.matchCondition(context));
    usernameCheck(true);
    assertTrue(condition.matchCondition(context));
  }

  @Test
  void regexMustMatchEntirely() {
    brokered(AbstractIdpAuthenticator.BROKERED_CONTEXT_NOTE, VOTER_ID + "@idp.example.org");
    usernameCheck(false);
    assertFalse(condition.matchCondition(context));
  }

  @Test
  void attributeWithoutRegexOnlyNeedsAValue() {
    brokered(AbstractIdpAuthenticator.BROKERED_CONTEXT_NOTE, VOTER_ID, "area-id", "area-1");
    areaCheck(false);
    assertTrue(condition.matchCondition(context));
  }

  @Test
  void missingAttributeMatchesNegatedCheck() {
    brokered(AbstractIdpAuthenticator.BROKERED_CONTEXT_NOTE, VOTER_ID);
    areaCheck(false);
    assertFalse(condition.matchCondition(context));
    areaCheck(true);
    assertTrue(condition.matchCondition(context));
  }

  @Test
  void readsPostBrokerLoginIdentity() {
    brokered(PostBrokerLoginConstants.PBL_BROKERED_IDENTITY_CONTEXT, VOTER_ID);
    areaCheck(true);
    assertTrue(condition.matchCondition(context));
  }

  @Test
  void fallsBackToUserOutsideBrokerFlows() {
    UserModel user = mock(UserModel.class);
    when(user.getFirstAttribute("area-id")).thenReturn("area-1");
    when(context.getUser()).thenReturn(user);
    areaCheck(false);
    assertTrue(condition.matchCondition(context));
  }

  @Test
  void noBrokeredIdentityAndNoUserDoesNotMatch() {
    areaCheck(false);
    assertFalse(condition.matchCondition(context));
  }

  @Test
  void invalidConfigurationFailsTheLogin() {
    brokered(AbstractIdpAuthenticator.BROKERED_CONTEXT_NOTE, VOTER_ID);
    configure(ConditionalBrokeredValueAuthenticatorFactory.CONF_REGEX, "(");
    assertThrows(IllegalStateException.class, () -> condition.matchCondition(context));
    configure(
        ConditionalBrokeredValueAuthenticatorFactory.CONF_TARGET, "USERNAME",
        ConditionalBrokeredValueAuthenticatorFactory.CONF_REGEX, "(");
    assertThrows(IllegalStateException.class, () -> condition.matchCondition(context));
    configure(ConditionalBrokeredValueAuthenticatorFactory.CONF_TARGET, "NICKNAME");
    assertThrows(IllegalStateException.class, () -> condition.matchCondition(context));
    when(context.getAuthenticatorConfig()).thenReturn(null);
    assertThrows(IllegalStateException.class, () -> condition.matchCondition(context));
  }
}
