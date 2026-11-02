// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import static org.mockito.ArgumentMatchers.*;
import static org.mockito.Mockito.*;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.Response;
import java.util.HashMap;
import java.util.stream.Stream;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.RequiredActionContext;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticationFlowModel;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialProvider;

class OTPRequiredActionInputTest {
  private final RequiredActionContext context =
      mock(RequiredActionContext.class, RETURNS_DEEP_STUBS);
  private final MultivaluedMap<String, String> parameters = new MultivaluedHashMap<>();
  private final LoginFormsProvider form = mock(LoginFormsProvider.class, RETURNS_SELF);
  private final Response response = mock(Response.class);
  private final UserModel user = mock(UserModel.class);

  @BeforeEach
  void setup() {
    RealmModel realm = mock(RealmModel.class);
    when(context.getRealm()).thenReturn(realm);
    when(context.getAuthenticationSession().getRealm()).thenReturn(realm);
    when(context.getAuthenticationSession().getAuthNote(Utils.CODE)).thenReturn("482619");
    when(context.getAuthenticationSession().getAuthNote(Utils.CODE_TTL))
        .thenReturn(Long.toString(System.currentTimeMillis() + 300_000));
    when(context.getHttpRequest().getDecodedFormParameters()).thenReturn(parameters);
    when(context.getUser()).thenReturn(user);
    when(context.form()).thenReturn(form);
    when(form.createForm(anyString())).thenReturn(response);
    AuthenticationFlowModel flow = new AuthenticationFlowModel();
    flow.setId("flow");
    AuthenticationExecutionModel execution = new AuthenticationExecutionModel();
    execution.setAuthenticator(MessageOTPAuthenticatorFactory.PROVIDER_ID);
    execution.setAuthenticatorConfig("config");
    AuthenticatorConfigModel config = new AuthenticatorConfigModel();
    config.setConfig(new HashMap<>(MessageOTPAuthenticatorFactory.getConfigMap(null)));
    when(realm.getAuthenticationFlowsStream()).thenAnswer(i -> Stream.of(flow));
    when(realm.getAuthenticationExecutionsStream("flow")).thenAnswer(i -> Stream.of(execution));
    when(realm.getAuthenticatorConfigById("config")).thenReturn(config);
  }

  @Test
  void credentialSetupRejectsMissingCodeWithoutThrowing() {
    new ResetMessageOTPRequiredAction().processAction(context);
    verify(context).failure();
    verify(context, never()).success();
  }

  @Test
  void credentialSetupStillAcceptsValidCode() {
    parameters.putSingle(Utils.CODE, "482619");
    ResetMessageOTPRequiredAction action = spy(new ResetMessageOTPRequiredAction());
    MessageOTPCredentialProvider credentials = mock(MessageOTPCredentialProvider.class);
    org.keycloak.models.KeycloakSession session = context.getSession();
    doReturn(credentials).when(action).getCredentialProvider(session);
    action.processAction(context);
    verify(credentials).createCredential(eq(context.getRealm()), eq(user), any());
    verify(context.getAuthenticationSession()).removeAuthNote(Utils.CODE);
    verify(context).success();
  }

  @Test
  void emailVerificationRejectsMissingCodeWithoutThrowing() {
    new VerifyOTPEmailRequiredAction().processAction(context);
    verify(context).failure();
    verify(context).challenge(response);
    verify(form).setError("messageOtp.auth.codeInvalid");
    verify(context, never()).success();
  }

  @Test
  void emailVerificationStillAcceptsValidCode() {
    parameters.putSingle(Utils.CODE, "482619");
    new VerifyOTPEmailRequiredAction().processAction(context);
    verify(user).setEmailVerified(true);
    verify(context.getAuthenticationSession()).removeAuthNote(Utils.CODE);
    verify(context).success();
  }
}
