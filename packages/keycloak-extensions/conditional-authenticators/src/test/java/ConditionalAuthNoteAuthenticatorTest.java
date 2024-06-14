// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.authenticators.conditional.ConditionalAuthenticator;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.mockito.Mock;
import org.mockito.MockitoAnnotations;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

public class ConditionalAuthNoteAuthenticatorTest {

    @Mock
    private AuthenticationFlowContext context;

    @Mock
    private AuthenticatorConfigModel authConfig;

    @Mock
    private AuthenticationSessionModel authSession;

    private ConditionalAuthNoteAuthenticator authenticator;

    @BeforeEach
    void setUp() {
        MockitoAnnotations.openMocks(this);
        authenticator = ConditionalAuthNoteAuthenticator.SINGLETON;
    }

    @Test
    void testMatchCondition_WithMatchingAuthNote() {
        // Prepare mock behavior
        when(context.getAuthenticatorConfig()).thenReturn(authConfig);
        when(authConfig.getConfig()).thenReturn(
                ConditionalAuthNoteAuthenticatorFactory.createConfig("noteKey", "noteValue", false)
        );
        when(context.getAuthenticationSession()).thenReturn(authSession);
        when(authSession.getAuthNote("noteKey")).thenReturn("noteValue");

        // Call method under test
        boolean result = authenticator.matchCondition(context);

        // Verify result
        assertTrue(result, "Expected matchCondition() to return true");
    }

    @Test
    void testMatchCondition_WithNonMatchingAuthNote() {
        // Prepare mock behavior
        when(context.getAuthenticatorConfig()).thenReturn(authConfig);
        when(authConfig.getConfig()).thenReturn(
                ConditionalAuthNoteAuthenticatorFactory.createConfig("noteKey", "noteValue", false)
        );
        when(context.getAuthenticationSession()).thenReturn(authSession);
        when(authSession.getAuthNote("noteKey")).thenReturn("differentValue");

        // Call method under test
        boolean result = authenticator.matchCondition(context);

        // Verify result
        assertFalse(result, "Expected matchCondition() to return false");
    }

    // Add more test cases as needed for different scenarios

}

 