// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.conditional_authenticators;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.ClientModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.mockito.InjectMocks;
import org.mockito.Mock;
import org.mockito.junit.jupiter.MockitoExtension;

import sequent.keycloak.conditional_authenticators.ConditionalClientAuthenticator;
import sequent.keycloak.conditional_authenticators.ConditionalClientAuthenticatorFactory;
import software.amazon.awssdk.utils.ImmutableMap;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.Mockito.*;

@ExtendWith(MockitoExtension.class)
public class ConditionalClientAuthenticatorTest {

    @Mock
    private AuthenticationFlowContext context;

    @Mock
    private AuthenticatorConfigModel authConfig;

    @Mock
    private AuthenticationSessionModel authSession;

    @Mock
    private ClientModel client;

    @InjectMocks
    private ConditionalClientAuthenticator authenticator;

    @BeforeEach
    public void setUp() {
        when(context.getAuthenticatorConfig()).thenReturn(authConfig);
        when(context.getAuthenticationSession()).thenReturn(authSession);
    }

    @Test
    public void testMatchCondition_withMatchingClientId() {
        // Prepare test data
        String clientId = "test-client";
        when(authConfig.getConfig()).thenReturn(
                ImmutableMap.of(
                        ConditionalClientAuthenticatorFactory.CONDITIONAL_CLIENT_ID, clientId,
                        ConditionalClientAuthenticatorFactory.CONF_NEGATE, "false"
                )
        );
        when(authSession.getClient()).thenReturn(client);
        when(client.getClientId()).thenReturn(clientId);

        // Execute
        boolean result = authenticator.matchCondition(context);

        // Verify
        assertTrue(result);
    }

    @Test
    public void testMatchCondition_withNonMatchingClientId() {
        // Prepare test data
        String requiredClientId = "required-client";
        String actualClientId = "actual-client";
        when(authConfig.getConfig()).thenReturn(
                ImmutableMap.of(
                        ConditionalClientAuthenticatorFactory.CONDITIONAL_CLIENT_ID, requiredClientId,
                        ConditionalClientAuthenticatorFactory.CONF_NEGATE, "false"
                )
        );
        when(authSession.getClient()).thenReturn(client);
        when(client.getClientId()).thenReturn(actualClientId);

        // Execute
        boolean result = authenticator.matchCondition(context);

        // Verify
        assertFalse(result);
    }

    // Add more test cases as needed
}
