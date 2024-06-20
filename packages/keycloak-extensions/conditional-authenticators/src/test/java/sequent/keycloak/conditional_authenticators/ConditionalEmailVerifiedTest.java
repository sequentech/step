// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

import java.util.Map;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
//import org.keycloak.authentication.AuthenticatorConfigModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.AuthenticatorConfigModel;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;
import static sequent.keycloak.conditional_authenticators.ConditionalEmailVerified.CONF_NEGATE;


public class ConditionalEmailVerifiedTest {

    private ConditionalEmailVerified conditionalEmailVerified;
    private AuthenticationFlowContext context;
    private UserModel mockUser;

    @BeforeEach
    public void setup() {
        conditionalEmailVerified = new ConditionalEmailVerified();
        context = mock(AuthenticationFlowContext.class);
        mockUser = mock(UserModel.class);

        // Mocking authenticator config
        AuthenticatorConfigModel mockAuthConfig = mock(AuthenticatorConfigModel.class);
        when(context.getAuthenticatorConfig()).thenReturn(mockAuthConfig);
        when(mockAuthConfig.getConfig()).thenReturn(null); 
        when(mockAuthConfig.getAlias()).thenReturn("testAlias");

        // Mocking user and its email verification status
        when(context.getUser()).thenReturn(mockUser);
    }

    @Test
    public void testMatchConditionEmailVerified() {
        // Mocking email verified status
        AuthenticationFlowContext context = mock(AuthenticationFlowContext.class);
        AuthenticatorConfigModel authConfig = mock(AuthenticatorConfigModel.class);
        UserModel mockUser = mock(UserModel.class);

        // Mock AuthenticatorConfigModel and config
        when(context.getAuthenticatorConfig()).thenReturn(authConfig);
        when(authConfig.getConfig()).thenReturn(Map.of(CONF_NEGATE, "false")); // Assuming a valid config setup

        // Mock UserModel
        when(context.getUser()).thenReturn(mockUser);
        when(mockUser.isEmailVerified()).thenReturn(true);

        // Instantiate the conditional authenticator
        ConditionalEmailVerified conditionalEmailVerified = new ConditionalEmailVerified();

        // Test the method
        boolean result = conditionalEmailVerified.matchCondition(context);

        System.out.println("Before assertion: result = " + result);
        assertTrue(result, "Condition should match when email is verified");        
    }
    

    @Test
    public void testMatchConditionEmailNotVerified() {
        // Mocking email verified status
        AuthenticationFlowContext context = mock(AuthenticationFlowContext.class);
        AuthenticatorConfigModel authConfig = mock(AuthenticatorConfigModel.class);
        UserModel mockUser = mock(UserModel.class);

        // Mock AuthenticatorConfigModel and config
        when(context.getAuthenticatorConfig()).thenReturn(authConfig);
        when(authConfig.getConfig()).thenReturn(Map.of(CONF_NEGATE, "false")); // Assuming a valid config setup

        // Mock UserModel
        when(context.getUser()).thenReturn(mockUser);
        when(mockUser.isEmailVerified()).thenReturn(false);

        // Instantiate the conditional authenticator
        ConditionalEmailVerified conditionalEmailVerified = new ConditionalEmailVerified();

        // Test the method
        boolean result = conditionalEmailVerified.matchCondition(context);

        System.out.println("Before assertion: result = " + result);
        assertFalse(result, "Condition should match when email is verified");        
    }

    @Test
    public void testMatchConditionNullAuthConfig() {
        // Mocking null authenticator config
        when(context.getAuthenticatorConfig()).thenReturn(null);

        boolean result = conditionalEmailVerified.matchCondition(context);

        assertFalse(result, "Condition should not match when authenticator config is null");
    }

    @Test
    public void testMatchConditionNullConfig() {
        // Mocking null config in authenticator config
        AuthenticatorConfigModel mockAuthConfig = mock(AuthenticatorConfigModel.class);
        when(context.getAuthenticatorConfig()).thenReturn(mockAuthConfig);
        when(mockAuthConfig.getConfig()).thenReturn(null);

        boolean result = conditionalEmailVerified.matchCondition(context);

        assertFalse(result, "Condition should not match when config in authenticator config is null");
    }

   

}
