// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.Mockito.*;

import java.util.Map;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.UserModel;
import org.mockito.junit.jupiter.MockitoExtension;

@ExtendWith(MockitoExtension.class)
public class RemoveUserAttributeTest {

  private RemoveUserAttribute removeUserAttribute;
  private AuthenticationFlowContext context;
  private AuthenticatorConfigModel authConfig;
  private UserModel user;

  @BeforeEach
  public void setup() {
    removeUserAttribute = new RemoveUserAttribute();
    context = mock(AuthenticationFlowContext.class);
    authConfig = mock(AuthenticatorConfigModel.class);
    user = mock(UserModel.class);
  }

  // critical edge cases, Null and Required
  @Test
  public void testAuthenticateUserIsNull() {
    // Mock a null user
    when(context.getUser()).thenReturn(null);

    // Call the method
    removeUserAttribute.authenticate(context);

    // Verify no attribute removal and no context success call
    verify(user, never()).removeAttribute(anyString());
    verify(context, never()).success();
  }

  @Test
  public void testAuthenticateExecutionNotRequired() {
    // Arrange
    when(context.getUser()).thenReturn(user);
    when(context.getAuthenticatorConfig()).thenReturn(authConfig);
    when(authConfig.getConfig())
        .thenReturn(Map.of(RemoveUserAttribute.CONF_USER_ATTRIBUTE, "test-attribute"));

    // Mocking AuthenticationExecutionModel or its relevant method to return a non-null value
    AuthenticationExecutionModel execution = mock(AuthenticationExecutionModel.class);
    when(context.getExecution()).thenReturn(execution);
    when(context.getExecution().isRequired()).thenReturn(false);
    when(context.getExecution().isConditional()).thenReturn(false);

    // Action
    removeUserAttribute.authenticate(context);

    // Assert
    // Verify that no attribute removal is attempted and context.success() is called
    verify(user, never()).removeAttribute("test-attribute");
    verify(context).success();
  }

  // Confirmation Test
  @Test
  public void testAuthenticateUserAttributeRemoved() {
    // Arrange
    String testAttribute = "test-attribute";
    when(context.getUser()).thenReturn(user);
    when(context.getAuthenticatorConfig()).thenReturn(authConfig);
    when(authConfig.getConfig())
        .thenReturn(Map.of(RemoveUserAttribute.CONF_USER_ATTRIBUTE, testAttribute));

    // Mock execution context
    AuthenticationExecutionModel execution = mock(AuthenticationExecutionModel.class);
    when(context.getExecution()).thenReturn(execution);
    when(context.getExecution().isRequired()).thenReturn(true);

    // Call the method
    removeUserAttribute.authenticate(context);

    // Verify the user attribute was removed
    verify(user, times(1)).removeAttribute("test-attribute");
    // Verify that context.success() was called
    verify(context, times(1)).success();
  }
}
