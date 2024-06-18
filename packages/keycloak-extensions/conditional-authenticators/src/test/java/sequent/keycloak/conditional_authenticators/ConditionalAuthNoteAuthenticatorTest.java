package sequent.keycloak.conditional_authenticators;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.authenticators.conditional.ConditionalAuthenticator;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.mockito.InjectMocks;
import org.mockito.Mock;
import org.mockito.MockitoAnnotations;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.Mockito.*;

import java.util.HashMap;
import java.util.Map;

public class ConditionalAuthNoteAuthenticatorTest {

    @Mock
    private AuthenticationFlowContext context;

    @Mock
    private AuthenticatorConfigModel authConfig;

    @Mock
    private AuthenticationSessionModel authSession;

    @InjectMocks
    private ConditionalAuthNoteAuthenticator authenticator;

    @BeforeEach
    void setUp() {
        MockitoAnnotations.openMocks(this);
    }

    @Test
    void testMatchCondition_WithMatchingAuthNote() {
        // Prepare mock behavior
        when(context.getAuthenticatorConfig()).thenReturn(authConfig);
        when(authConfig.getConfig()).thenReturn(createMockConfig("noteKey", "noteValue", "false"));
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
        when(authConfig.getConfig()).thenReturn(createMockConfig("noteKey", "noteValue", "false"));
        when(context.getAuthenticationSession()).thenReturn(authSession);
        when(authSession.getAuthNote("noteKey")).thenReturn("differentValue");

        // Call method under test
        boolean result = authenticator.matchCondition(context);

        // Verify result
        assertFalse(result, "Expected matchCondition() to return false");
    }


    @Test
    void testMatchCondition_WithNegatedOutput() {
        // Prepare mock behavior
        when(context.getAuthenticatorConfig()).thenReturn(authConfig);
        when(authConfig.getConfig()).thenReturn(createMockConfig("noteKey", "noteValue", "true"));
        when(context.getAuthenticationSession()).thenReturn(authSession);
        when(authSession.getAuthNote("noteKey")).thenReturn("noteValue");

        // Call method under test
        boolean result = authenticator.matchCondition(context);

        // Verify result
        assertFalse(result, "Expected matchCondition() to return false (negated)");
    }

    // Utility method to create mock config map
    private Map<String, String> createMockConfig(String key, String value, String negate) {
        Map<String, String> config = new HashMap<>();
        config.put(ConditionalAuthNoteAuthenticatorFactory.CONDITIONAL_AUTH_NOTE_KEY, key);
        config.put(ConditionalAuthNoteAuthenticatorFactory.CONDITIONAL_AUTH_NOTE_VALUE, value);
        config.put(ConditionalAuthNoteAuthenticatorFactory.CONF_NEGATE, negate);
        return config;
    }

    // Add more test cases as needed for different scenarios

}
