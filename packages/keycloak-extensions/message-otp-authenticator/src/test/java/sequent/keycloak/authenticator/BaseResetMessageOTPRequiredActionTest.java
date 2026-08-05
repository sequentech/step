// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.authenticator;

import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.verify;
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
  private final LoginFormsProvider form = mock(LoginFormsProvider.class);

  private RequiredActionContext mockContext() {
    RequiredActionContext context = mock(RequiredActionContext.class);
    AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
    when(context.form()).thenReturn(form);
    when(context.getAuthenticationSession()).thenReturn(authSession);
    when(authSession.getAuthNote("email")).thenReturn("voter@example.com");
    return context;
  }

  private AuthenticatorConfigModel configWith(Map<String, String> values) {
    AuthenticatorConfigModel config = new AuthenticatorConfigModel();
    config.setConfig(values);
    return config;
  }

  @Test
  void otpFormFallsBackToDefaultsWhenTheConfigIsMissingKeys() {
    action.createOTPForm(mockContext(), null, configWith(new HashMap<>()));

    verify(form).setAttribute("resendTimer", Utils.RESEND_ACTIVATION_TIMER_DEFAULT);
    verify(form).setAttribute("codeLength", Utils.CODE_LENGTH_DEFAULT);
    verify(form).setAttribute("ttl", Utils.CODE_TTL_DEFAULT);
  }

  @Test
  void otpFormFallsBackToDefaultsWhenThereIsNoConfig() {
    action.createOTPForm(mockContext(), null, null);

    verify(form).setAttribute("resendTimer", Utils.RESEND_ACTIVATION_TIMER_DEFAULT);
    verify(form).setAttribute("codeLength", Utils.CODE_LENGTH_DEFAULT);
    verify(form).setAttribute("ttl", Utils.CODE_TTL_DEFAULT);
  }

  @Test
  void otpFormUsesTheConfiguredValues() {
    Map<String, String> values = new HashMap<>();
    values.put(Utils.RESEND_ACTIVATION_TIMER, "90");
    values.put(Utils.CODE_LENGTH, "4");
    values.put(Utils.CODE_TTL, "120");

    action.createOTPForm(mockContext(), null, configWith(values));

    verify(form).setAttribute("resendTimer", "90");
    verify(form).setAttribute("codeLength", "4");
    verify(form).setAttribute("ttl", "120");
  }
}
