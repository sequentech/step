package sequent.keycloak.conditional_authenticators;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.UserModel;
import org.mockito.Mock;
import org.mockito.MockitoAnnotations;
import sequent.keycloak.conditional_authenticators.ConditionalHasUserAttributeAuthenticator;
import sequent.keycloak.conditional_authenticators.ConditionalHasUserAttributeAuthenticatorFactory;

import java.util.HashMap;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.Mockito.*;

public class ConditionalHasUserAttributeAuthenticatorTest {

    @Mock
    private AuthenticationFlowContext context;

    @Mock
    private AuthenticatorConfigModel authConfig;

    @Mock
    private UserModel user;

    private ConditionalHasUserAttributeAuthenticator authenticator;

    @BeforeEach
    void setUp() {
        MockitoAnnotations.openMocks(this);
        authenticator = ConditionalHasUserAttributeAuthenticator.SINGLETON;
    }

    @Test
    void testMatchCondition_WhenAttributePresentAndNegateFalse() {
        // Prepare mock behavior
        Map<String, String> config = new HashMap<>();
        config.put(ConditionalHasUserAttributeAuthenticatorFactory.CONF_USER_ATTRIBUTE_KEY, "attributeKey");
        config.put(ConditionalHasUserAttributeAuthenticatorFactory.CONF_NEGATE, "false");

        when(context.getAuthenticatorConfig()).thenReturn(authConfig);
        when(authConfig.getConfig()).thenReturn(config);
        when(context.getUser()).thenReturn(user);
        when(user.getFirstAttribute("attributeKey")).thenReturn("attributeValue");

        // Call method under test
        boolean result = authenticator.matchCondition(context);

        // Verify result
        assertTrue(result, "Expected matchCondition() to return true");
    }

    @Test
    void testMatchCondition_WhenAttributeNotPresentAndNegateFalse() {
        // Prepare mock behavior
        Map<String, String> config = new HashMap<>();
        config.put(ConditionalHasUserAttributeAuthenticatorFactory.CONF_USER_ATTRIBUTE_KEY, "attributeKey");
        config.put(ConditionalHasUserAttributeAuthenticatorFactory.CONF_NEGATE, "false");

        when(context.getAuthenticatorConfig()).thenReturn(authConfig);
        when(authConfig.getConfig()).thenReturn(config);
        when(context.getUser()).thenReturn(user);
        when(user.getFirstAttribute("attributeKey")).thenReturn(null);

        // Call method under test
        boolean result = authenticator.matchCondition(context);

        // Verify result
        assertFalse(result, "Expected matchCondition() to return false");
    }

    @Test
    void testMatchCondition_WhenAttributePresentAndNegateTrue() {
        // Prepare mock behavior
        Map<String, String> config = new HashMap<>();
        config.put(ConditionalHasUserAttributeAuthenticatorFactory.CONF_USER_ATTRIBUTE_KEY, "attributeKey");
        config.put(ConditionalHasUserAttributeAuthenticatorFactory.CONF_NEGATE, "true");

        when(context.getAuthenticatorConfig()).thenReturn(authConfig);
        when(authConfig.getConfig()).thenReturn(config);
        when(context.getUser()).thenReturn(user);
        when(user.getFirstAttribute("attributeKey")).thenReturn("attributeValue");

        // Call method under test
        boolean result = authenticator.matchCondition(context);

        // Verify result
        assertFalse(result, "Expected matchCondition() to return false");
    }

    @Test
    void testMatchCondition_WhenAttributeNotPresentAndNegateTrue() {
        // Prepare mock behavior
        Map<String, String> config = new HashMap<>();
        config.put(ConditionalHasUserAttributeAuthenticatorFactory.CONF_USER_ATTRIBUTE_KEY, "attributeKey");
        config.put(ConditionalHasUserAttributeAuthenticatorFactory.CONF_NEGATE, "true");

        when(context.getAuthenticatorConfig()).thenReturn(authConfig);
        when(authConfig.getConfig()).thenReturn(config);
        when(context.getUser()).thenReturn(user);
        when(user.getFirstAttribute("attributeKey")).thenReturn(null);

        // Call method under test
        boolean result = authenticator.matchCondition(context);

        // Verify result
        assertTrue(result, "Expected matchCondition() to return true");
    }
}
