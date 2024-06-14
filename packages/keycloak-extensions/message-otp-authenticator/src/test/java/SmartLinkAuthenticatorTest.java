// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.authentication.authenticators.browser.UsernamePasswordForm;
import org.keycloak.events.EventBuilder;
import org.keycloak.models.*;
import org.keycloak.services.managers.AuthenticationManager;

public class SmartLinkAuthenticatorTest {

    private SmartLinkAuthenticator authenticator;
    private AuthenticationFlowContext context;

    @BeforeEach
    public void setup() {
        authenticator = new SmartLinkAuthenticator();
        context = mock(AuthenticationFlowContext.class);

        KeycloakSession session = mock(KeycloakSession.class);
        RealmModel realm = mock(RealmModel.class);
        ClientModel client = mock(ClientModel.class);
        AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);

        when(context.getSession()).thenReturn(session);
        when(session.getContext()).thenReturn(new ClientSessionContext(realm, client, authSession));
        when(context.getRealm()).thenReturn(realm);
        when(context.getAuthenticationSession()).thenReturn(authSession);
        when(context.form()).thenReturn(mock(LoginFormsProvider.class));
        when(context.getHttpRequest()).thenReturn(mock(HttpRequest.class));
        when(context.getEvent()).thenReturn(mock(EventBuilder.class));
    }

    @Test
    public void testAuthenticateWithAttemptedUsername() {
        // Simulate a scenario where attemptedUsername is set
        when(context.getAuthenticationSession().getAuthNote(AbstractUsernameFormAuthenticator.ATTEMPTED_USERNAME))
            .thenReturn("testuser@example.com");

        authenticator.authenticate(context);

        verify(context, never()).failureChallenge(any(), any());
        verify(context).challenge(any());
    }

    @Test
    public void testAuthenticateWithoutAttemptedUsername() {
        // Simulate a scenario where attemptedUsername is not set
        when(context.getAuthenticationSession().getAuthNote(AbstractUsernameFormAuthenticator.ATTEMPTED_USERNAME))
            .thenReturn(null);

        authenticator.authenticate(context);

        verify(context, never()).challenge(any());
        verify(context).failureChallenge(any(), any());
    }

    @Test
    public void testActionInvalidEmail() {
        // Simulate invalid email scenario
        MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
        formData.add(AuthenticationManager.FORM_USERNAME, "invalidemail");

        when(context.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        authenticator.action(context);

        verify(context).failureChallenge(any(), any());
        verify(context.getEvent()).event(EventType.LOGIN_ERROR);
    }

    @Test
    public void testActionValidFlow() {
        // Simulate valid email scenario
        MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
        formData.add(AuthenticationManager.FORM_USERNAME, "validemail@example.com");

        when(context.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);
        when(context.getSession().users().getUserByUsername(any(), any())).thenReturn(mock(UserModel.class));
        when(context.getAuthenticationSession().getClient()).thenReturn(mock(ClientModel.class));
        when(context.form().createForm(anyString())).thenReturn(mock(LoginFormsProvider.class));

        authenticator.action(context);

        verify(context).challenge(any());
        verify(context.getAuthenticationSession()).setAuthNote(anyString(), anyString());
    }
}
