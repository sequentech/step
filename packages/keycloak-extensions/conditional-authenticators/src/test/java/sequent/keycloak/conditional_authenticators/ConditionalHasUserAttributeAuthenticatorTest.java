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
import org.mockito.Mock;
import org.mockito.MockitoAnnotations;

public class ConditionalHasUserAttributeAuthenticatorTest {

  private ConditionalHasUserAttributeAuthenticator authenticator;
  @Mock private AuthenticationFlowContext context;
  @Mock private AuthenticatorConfigModel authConfig;
  @Mock private UserModel user;

  @BeforeEach
  void setUp() {
    MockitoAnnotations.openMocks(this);
    authenticator = ConditionalHasUserAttributeAuthenticator.SINGLETON;
  }

  @Test
  public void testMatchConditionUserAttributePresent() {
    setValidAuthConfig("testAttribute", false);
    when(context.getUser()).thenReturn(user);
    when(user.getFirstAttribute("testAttribute")).thenReturn("attributeValue");
    boolean result = authenticator.matchCondition(context);
    assertTrue(result, "Condition should match when user attribute is present");
  }

  @Test
  public void testMatchConditionUserNullAttributet() {
    setValidAuthConfig("testAttribute", false);
    when(context.getUser()).thenReturn(user);
    when(user.getFirstAttribute("")).thenReturn(null);
    boolean result = authenticator.matchCondition(context);
    assertFalse(result, "Condition should not match when user attribute is not present");
  }

  @Test
  public void testMatchConditionNullAuthConfig() {
    when(context.getAuthenticatorConfig()).thenReturn(null);
    boolean result = authenticator.matchCondition(context);
    assertFalse(result, "Condition should not match when AuthenticatorConfigModel is null");
  }

  @Test
  public void testMatchConditionNullUser() {
    setValidAuthConfig("testAttributes", false);
    when(context.getUser()).thenReturn(null);
    boolean result = authenticator.matchCondition(context);
    assertFalse(result, "Condition should not match when UserModel is null");
  }

  @Test
  public void testIsReuiredUser() {
    boolean results = authenticator.requiresUser();
    assertTrue(results, "requiresUser() should return true");
  }

  public void setValidAuthConfig(String userAttributeKey, boolean negate) {
    Map<String, String> config = new HashMap<>();
    config.put(
        ConditionalHasUserAttributeAuthenticatorFactory.CONF_USER_ATTRIBUTE_KEY, userAttributeKey);
    config.put(ConditionalHasUserAttributeAuthenticatorFactory.CONF_NEGATE, String.valueOf(negate));
    when(authConfig.getConfig()).thenReturn(config);
    when(context.getAuthenticatorConfig()).thenReturn(authConfig);
  }
}
