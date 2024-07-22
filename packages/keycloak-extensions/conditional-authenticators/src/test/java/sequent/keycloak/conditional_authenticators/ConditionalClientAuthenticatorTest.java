// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.Mockito.*;

import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.ClientModel;
import org.keycloak.sessions.AuthenticationSessionModel;

public class ConditionalClientAuthenticatorTest {

  private ConditionalClientAuthenticator conditionalClientAuthenticator;
  private AuthenticationFlowContext context;
  private AuthenticatorConfigModel authConfig;
  private AuthenticationSessionModel authSession;
  private ClientModel client;

  @BeforeEach
  void setUp() {
    conditionalClientAuthenticator = ConditionalClientAuthenticator.SINGLETON;
    context = mock(AuthenticationFlowContext.class);
    authConfig = mock(AuthenticatorConfigModel.class);
    authSession = mock(AuthenticationSessionModel.class);
    client = mock(ClientModel.class);
  }

  @Test
  public void testMatchConditionClientIdMatched() {
    // Mock configuration
    Map<String, String> config = new HashMap<>();
    config.put(ConditionalClientAuthenticatorFactory.CONDITIONAL_CLIENT_ID, "test-client");
    config.put(ConditionalClientAuthenticatorFactory.CONF_NEGATE, "false");
    when(authConfig.getConfig()).thenReturn(config);

    // Mock context behavior
    when(context.getAuthenticatorConfig()).thenReturn(authConfig);
    when(context.getAuthenticationSession()).thenReturn(authSession);
    when(authSession.getClient()).thenReturn(client);
    when(client.getClientId()).thenReturn("test-client");

    // Test matchCondition method
    boolean result = conditionalClientAuthenticator.matchCondition(context);

    assertTrue(result, "Condition should match when client ID matches");
  }

  @Test
  public void testMatchConditionClientIdNotMatched() {
    // Mock configuration
    Map<String, String> config = new HashMap<>();
    config.put(ConditionalClientAuthenticatorFactory.CONDITIONAL_CLIENT_ID, "test-client");
    config.put(ConditionalClientAuthenticatorFactory.CONF_NEGATE, "false");
    when(authConfig.getConfig()).thenReturn(config);

    // Mock context behavior
    when(context.getAuthenticatorConfig()).thenReturn(authConfig);
    when(context.getAuthenticationSession()).thenReturn(authSession);
    when(authSession.getClient()).thenReturn(client);
    when(client.getClientId()).thenReturn("another-client");

    // Test matchCondition method
    boolean result = conditionalClientAuthenticator.matchCondition(context);

    assertFalse(result, "Condition should not match when client ID does not match");
  }
}
