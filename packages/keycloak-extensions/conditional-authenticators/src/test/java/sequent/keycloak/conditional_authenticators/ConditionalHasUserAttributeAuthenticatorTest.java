// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.UserModel;

public class ConditionalHasUserAttributeAuthenticatorTest {

  private ConditionalHasUserAttributeAuthenticator authenticator;
  private AuthenticationFlowContext context;
  private AuthenticatorConfigModel authConfig;
  private UserModel user;

  @BeforeEach
  void setUp() {
    authenticator = ConditionalHasUserAttributeAuthenticator.SINGLETON;
    context = mock(AuthenticationFlowContext.class);
    authConfig = mock(AuthenticatorConfigModel.class);
    user = mock(UserModel.class);
  }

  @Test
  public void testMatchConditionUserAttributePresent() {
    // Mock configuration
    Map<String, String> config = new HashMap<>();
    config.put(
        ConditionalHasUserAttributeAuthenticatorFactory.CONF_USER_ATTRIBUTE_KEY, "testAttribute");
    config.put(ConditionalHasUserAttributeAuthenticatorFactory.CONF_NEGATE, "false");
    when(authConfig.getConfig()).thenReturn(config);

    // Mock context behavior
    when(context.getAuthenticatorConfig()).thenReturn(authConfig);
    when(context.getUser()).thenReturn(user);
    when(user.getFirstAttribute("testAttribute")).thenReturn("attributeValue");

    // Test the method
    boolean result = authenticator.matchCondition(context);

    assertTrue(result, "Condition should match when user attribute is present");
  }

  @Test
  public void testMatchConditionUserAttributeNotPresent() {
    // Mock configuration
    Map<String, String> config = new HashMap<>();
    config.put(
        ConditionalHasUserAttributeAuthenticatorFactory.CONF_USER_ATTRIBUTE_KEY, "testAttribute");
    config.put(ConditionalHasUserAttributeAuthenticatorFactory.CONF_NEGATE, "false");
    when(authConfig.getConfig()).thenReturn(config);

    // Mock context behavior
    when(context.getAuthenticatorConfig()).thenReturn(authConfig);
    when(context.getUser()).thenReturn(user);
    when(user.getFirstAttribute("testAttribute")).thenReturn(null); // Attribute is not present

    // Test the method
    boolean result = authenticator.matchCondition(context);

    assertFalse(result, "Condition should not match when user attribute is not present");
  }

  @Test
  public void testMatchConditionNullAuthConfig() {
    // Mock context behavior with null AuthenticatorConfigModel
    when(context.getAuthenticatorConfig()).thenReturn(null);

    // Test the method
    boolean result = authenticator.matchCondition(context);

    assertFalse(result, "Condition should not match when AuthenticatorConfigModel is null");
  }

  @Test
  public void testMatchConditionNullUser() {
    // Mock configuration
    Map<String, String> config = new HashMap<>();
    config.put(
        ConditionalHasUserAttributeAuthenticatorFactory.CONF_USER_ATTRIBUTE_KEY, "testAttribute");
    config.put(ConditionalHasUserAttributeAuthenticatorFactory.CONF_NEGATE, "false");
    when(authConfig.getConfig()).thenReturn(config);

    // Mock context behavior with null UserModel
    when(context.getAuthenticatorConfig()).thenReturn(authConfig);
    when(context.getUser()).thenReturn(null);

    // Test the method
    boolean result = authenticator.matchCondition(context);

    assertFalse(result, "Condition should not match when UserModel is null");
  }
}
