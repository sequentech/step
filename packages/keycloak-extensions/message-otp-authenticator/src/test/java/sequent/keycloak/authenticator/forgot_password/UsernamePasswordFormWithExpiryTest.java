// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.authenticator.forgot_password;

import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.doReturn;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.spy;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import jakarta.ws.rs.core.MultivaluedHashMap;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.http.HttpRequest;
import org.keycloak.models.credential.PasswordCredentialModel;

class UsernamePasswordFormWithExpiryTest {

  @Test
  void action_validForm_recordsThePasswordCredential() {
    // Keycloak 26.7 clears a user's brute-force failures after a login only when the login
    // recorded a counted credential type.
    AuthenticationFlowContext context = mock(AuthenticationFlowContext.class);
    HttpRequest request = mock(HttpRequest.class);
    when(context.getHttpRequest()).thenReturn(request);
    when(request.getDecodedFormParameters()).thenReturn(new MultivaluedHashMap<>());
    UsernamePasswordFormWithExpiry form = spy(new UsernamePasswordFormWithExpiry());
    doReturn(true).when(form).validateForm(eq(context), any());
    doReturn(false).when(form).getDisablePassword(context);

    form.action(context);

    verify(context).success(PasswordCredentialModel.TYPE);
    verify(context, never()).success();
  }

  @Test
  void action_validFormWithPasswordDisabled_recordsNoPasswordCredential() {
    // With password validation disabled the form checks no password, so it must not claim one:
    // conditional-credential steps and brute-force accounting read the recorded credential type.
    AuthenticationFlowContext context = mock(AuthenticationFlowContext.class);
    HttpRequest request = mock(HttpRequest.class);
    when(context.getHttpRequest()).thenReturn(request);
    when(request.getDecodedFormParameters()).thenReturn(new MultivaluedHashMap<>());
    UsernamePasswordFormWithExpiry form = spy(new UsernamePasswordFormWithExpiry());
    doReturn(true).when(form).validateForm(eq(context), any());
    doReturn(true).when(form).getDisablePassword(context);

    form.action(context);

    verify(context).success();
    verify(context, never()).success(any());
  }
}
