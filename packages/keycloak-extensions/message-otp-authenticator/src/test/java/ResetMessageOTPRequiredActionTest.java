// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.RequiredActionContext;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.*;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.mockito.ArgumentCaptor;

import jakarta.ws.rs.core.Response;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

public class ResetMessageOTPRequiredActionTest {

    private RequiredActionContext context;
    private KeycloakSession session;
    private UserModel userModel;
    private AuthenticationSessionModel authSessionModel;
    private LoginFormsProvider formProvider;

    @BeforeEach
    public void setup() {
        context = mock(RequiredActionContext.class);
        session = mock(KeycloakSession.class);
        userModel = mock(UserModel.class);
        authSessionModel = mock(AuthenticationSessionModel.class);
        formProvider = mock(LoginFormsProvider.class);

        when(context.getSession()).thenReturn(session);
        when(context.getUser()).thenReturn(userModel);
        when(context.getAuthenticationSession()).thenReturn(authSessionModel);
        when(context.form()).thenReturn(formProvider);
    }

    @Test
    public void testProcessActionSuccess() {
        when(authSessionModel.getAuthNote(Utils.CODE)).thenReturn("123456");
        when(authSessionModel.getAuthNote(Utils.CODE_TTL)).thenReturn(String.valueOf(System.currentTimeMillis() + 10000));
        when(context.getHttpRequest().getDecodedFormParameters().getFirst(Utils.CODE)).thenReturn("123456");

        ResetMessageOTPRequiredAction action = new ResetMessageOTPRequiredAction();
        action.processAction(context);

        verify(authSessionModel).removeAuthNote(Utils.CODE);
        verify(authSessionModel).removeRequiredAction(ResetMessageOTPRequiredAction.PROVIDER_ID);
        verify(userModel).removeRequiredAction(ResetMessageOTPRequiredAction.PROVIDER_ID);
        verify(context).success();
    }

    @Test
    public void testProcessActionCodeExpired() {
        when(authSessionModel.getAuthNote(Utils.CODE)).thenReturn("123456");
        when(authSessionModel.getAuthNote(Utils.CODE_TTL)).thenReturn(String.valueOf(System.currentTimeMillis() - 10000));
        when(context.getHttpRequest().getDecodedFormParameters().getFirst(Utils.CODE)).thenReturn("123456");

        ResetMessageOTPRequiredAction action = new ResetMessageOTPRequiredAction();
        action.processAction(context);

        ArgumentCaptor<Response> captor = ArgumentCaptor.forClass(Response.class);
        verify(context).challenge(captor.capture());

        Response response = captor.getValue();
        assertNotNull(response);
        assertEquals(Response.Status.BAD_REQUEST.getStatusCode(), response.getStatus());
    }

    @Test
    public void testProcessActionCodeInvalid() {
        when(authSessionModel.getAuthNote(Utils.CODE)).thenReturn("123456");
        when(authSessionModel.getAuthNote(Utils.CODE_TTL)).thenReturn(String.valueOf(System.currentTimeMillis() + 10000));
        when(context.getHttpRequest().getDecodedFormParameters().getFirst(Utils.CODE)).thenReturn("654321");

        ResetMessageOTPRequiredAction action = new ResetMessageOTPRequiredAction();
        action.processAction(context);

        ArgumentCaptor<Response> captor = ArgumentCaptor.forClass(Response.class);
        verify(context).challenge(captor.capture());

        Response response = captor.getValue();
        assertNotNull(response);
        assertEquals(Response.Status.BAD_REQUEST.getStatusCode(), response.getStatus());
    }

    @Test
    public void testCreateForm() {
        ResetMessageOTPRequiredAction action = new ResetMessageOTPRequiredAction();
        Response response = action.createForm(context, form -> form.setAttribute("attribute", "value"));

        verify(formProvider).setAttribute("realm", context.getRealm());
        verify(formProvider).setAttribute("attribute", "value");
        assertNotNull(response);
    }
}
