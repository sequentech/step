// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.RequiredActionContext;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.sessions.AuthenticationSessionModel;

/**
 * The OTP entry template interpolates the resendTimer, codeLength and ttl attributes directly, so a
 * missing attribute renders the login page unusable with a 500. These tests cover realms whose
 * saved authenticator config predates one of those keys.
 */
class BaseResetMessageOTPRequiredActionTest {

  private final ResetEmailOTPRequiredAction action = new ResetEmailOTPRequiredAction();
  private final Map<String, Object> attributes = new HashMap<>();

  /** Runs the OTP form and returns the attributes it handed to the template. */
  private Map<String, Object> renderWith(AuthenticatorConfigModel config) {
    LoginFormsProvider form = mock(LoginFormsProvider.class);
    when(form.setAttribute(anyString(), any()))
        .thenAnswer(
            invocation -> {
              attributes.put(invocation.getArgument(0), invocation.getArgument(1));
              return form;
            });

    RequiredActionContext context = mock(RequiredActionContext.class);
    AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
    when(context.form()).thenReturn(form);
    when(context.getAuthenticationSession()).thenReturn(authSession);
    when(authSession.getAuthNote("email")).thenReturn("voter@example.com");

    action.createOTPForm(context, null, config);
    return attributes;
  }

  private AuthenticatorConfigModel configWith(Map<String, String> values) {
    AuthenticatorConfigModel config = new AuthenticatorConfigModel();
    config.setConfig(values);
    return config;
  }

  @Test
  void otpFormFallsBackToDefaultsWhenTheConfigIsMissingKeys() {
    renderWith(configWith(new HashMap<>()));

    assertEquals(Utils.CODE_LENGTH_DEFAULT, attributes.get("codeLength"));
    assertEquals(Utils.RESEND_ACTIVATION_TIMER_DEFAULT, attributes.get("resendTimer"));
    assertEquals(Utils.CODE_TTL_DEFAULT, attributes.get("ttl"));
  }

  @Test
  void otpFormFallsBackToDefaultsWhenThereIsNoConfig() {
    renderWith(null);

    assertEquals(Utils.CODE_LENGTH_DEFAULT, attributes.get("codeLength"));
    assertEquals(Utils.RESEND_ACTIVATION_TIMER_DEFAULT, attributes.get("resendTimer"));
    assertEquals(Utils.CODE_TTL_DEFAULT, attributes.get("ttl"));
  }

  @Test
  void otpFormUsesTheConfiguredValues() {
    Map<String, String> values = new HashMap<>();
    values.put(Utils.CODE_LENGTH, "4");
    values.put(Utils.RESEND_ACTIVATION_TIMER, "90");
    values.put(Utils.CODE_TTL, "120");

    renderWith(configWith(values));

    assertEquals("4", attributes.get("codeLength"));
    assertEquals("90", attributes.get("resendTimer"));
    assertEquals("120", attributes.get("ttl"));
  }
}
