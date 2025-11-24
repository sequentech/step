// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;
import static sequent.keycloak.conditional_authenticators.ConditionalEmailVerified.CONF_NEGATE;

import java.util.Map;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.UserModel;
import org.mockito.Mock;
import org.mockito.MockitoAnnotations;

public class ConditionalEmailVerifiedTest {

  private ConditionalEmailVerified conditionalEmailVerified;
  @Mock private AuthenticationFlowContext context;
  @Mock private UserModel user;
  @Mock private AuthenticatorConfigModel authConfig;

  @BeforeEach
  public void setup() {
    MockitoAnnotations.openMocks(this);
    conditionalEmailVerified = new ConditionalEmailVerified();
  }

  @Test
  public void testMatchConditionEmailVerified() {
    when(context.getAuthenticatorConfig()).thenReturn(authConfig);
    when(authConfig.getConfig()).thenReturn(Map.of(CONF_NEGATE, "false"));
    when(context.getUser()).thenReturn(user);
    when(user.isEmailVerified()).thenReturn(true);
    boolean result = conditionalEmailVerified.matchCondition(context);
    assertTrue(result, "Condition should match when email is verified");
  }

  @Test
  public void testMatchConditionEmailNotVerified() {
    when(context.getAuthenticatorConfig()).thenReturn(authConfig);
    when(authConfig.getConfig()).thenReturn(Map.of(CONF_NEGATE, "false"));
    when(context.getUser()).thenReturn(user);
    when(user.isEmailVerified()).thenReturn(false);
    ConditionalEmailVerified conditionalEmailVerified = new ConditionalEmailVerified();
    boolean result = conditionalEmailVerified.matchCondition(context);
    System.out.println("Before assertion: result = " + result);
    assertFalse(result, "Condition should match when email is verified");
  }

  @Test
  public void testMatchConditionNullAuthConfig() {
    when(context.getAuthenticatorConfig()).thenReturn(null);
    boolean result = conditionalEmailVerified.matchCondition(context);
    assertFalse(result, "Condition should not match when authenticator config is null");
  }

  @Test
  public void testMatchConditionNullConfig() {
    AuthenticatorConfigModel mockAuthConfig = mock(AuthenticatorConfigModel.class);
    when(context.getAuthenticatorConfig()).thenReturn(mockAuthConfig);
    when(mockAuthConfig.getConfig()).thenReturn(null);
    boolean result = conditionalEmailVerified.matchCondition(context);
    assertFalse(result, "Condition should not match when config in authenticator config is null");
  }
}
