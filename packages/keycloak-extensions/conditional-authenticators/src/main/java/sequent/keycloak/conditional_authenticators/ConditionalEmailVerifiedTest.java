package sequent.keycloak.conditional_authenticators;

/*  BUILD FAILED - CANNOT FIND JUNIT.JUPITER.API 
 * Unit Concitional AuthNote Authenticator  JC:Ayeng  6132024
 *
 * NOTE : Commented out so dependencies are installed properly for other Unit Testing Files.
 *         UNCOMMENT IF TESTING FOR THIS FILE ONLY, or dependencies will be needed in all extensions
* /


import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.mockito.InjectMocks;
import org.mockito.Mock;
import org.mockito.junit.jupiter.MockitoExtension;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.Mockito.when;

@ExtendWith(MockitoExtension.class)
public class ConditionalEmailVerifiedTest {

    @Mock
    private AuthenticationFlowContext context;

    @Mock
    private AuthenticatorConfigModel authConfig;

    @Mock
    private AuthenticationSessionModel authSession;

    @Mock
    private UserModel user;

    @InjectMocks
    private ConditionalEmailVerified authenticator;

    @BeforeEach
    public void setUp() {
        when(context.getAuthenticatorConfig()).thenReturn(authConfig);
        when(context.getUser()).thenReturn(user);
    }

    @Test
    public void testMatchCondition_whenEmailVerifiedAndNegateFalse() {
        // Prepare test data
        when(authConfig.getConfig()).thenReturn(
                ImmutableMap.of(
                        ConditionalEmailVerified.CONF_NEGATE, "false"
                )
        );
        when(user.isEmailVerified()).thenReturn(true);

        // Execute
        boolean result = authenticator.matchCondition(context);

        // Verify
        assertTrue(result);
    }

    @Test
    public void testMatchCondition_whenEmailNotVerifiedAndNegateTrue() {
        // Prepare test data
        when(authConfig.getConfig()).thenReturn(
                ImmutableMap.of(
                        ConditionalEmailVerified.CONF_NEGATE, "true"
                )
        );
        when(user.isEmailVerified()).thenReturn(false);

        // Execute
        boolean result = authenticator.matchCondition(context);

        // Verify
        assertTrue(result);
    }

    @Test
    public void testMatchCondition_whenEmailVerifiedAndNegateTrue() {
        // Prepare test data
        when(authConfig.getConfig()).thenReturn(
                ImmutableMap.of(
                        ConditionalEmailVerified.CONF_NEGATE, "true"
                )
        );
        when(user.isEmailVerified()).thenReturn(true);

        // Execute
        boolean result = authenticator.matchCondition(context);

        // Verify
        assertFalse(result);
    }

    // Add more test cases as needed
}
*/
 