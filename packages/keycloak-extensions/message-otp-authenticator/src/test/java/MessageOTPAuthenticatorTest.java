
// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;
import static org.mockito.Mockito.*;
import static org.junit.jupiter.api.Assertions.*;

import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.Response;
import org.keycloak.models.AuthenticationSessionModel;
 
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;

import java.util.Map;

public class MessageOTPAuthenticatorTest {

    private MessageOTPAuthenticator authenticator;
    private AuthenticationFlowContext context;
    private AuthenticatorConfigModel config;
    private UserModel user;
    private AuthenticationSessionModel authSession;

    @BeforeEach
    public void setUp() {
        authenticator = new MessageOTPAuthenticator();
        context = mock(AuthenticationFlowContext.class);
        config = mock(AuthenticatorConfigModel.class);
        user = mock(UserModel.class);
        authSession = mock(AuthenticationSessionModel.class);

        RealmModel realm = mock(RealmModel.class);
        KeycloakSession session = mock(KeycloakSession.class);

        when(context.getRealm()).thenReturn(realm);
        when(context.getSession()).thenReturn(session);
        when(context.getUser()).thenReturn(user);
        when(context.getAuthenticationSession()).thenReturn(authSession);
        when(context.getAuthenticatorConfig()).thenReturn(config);

        when(config.getConfig()).thenReturn(Map.of(
            Utils.MESSAGE_COURIER_ATTRIBUTE, "sms",
            Utils.DEFERRED_USER_ATTRIBUTE, "false"
        ));
    }

    @Test
    public void testAuthenticate() {
        authenticator.authenticate(context);
        verify(context).challenge(any(Response.class));
    }

    @Test
    public void testAction_ValidCode() {
        MultivaluedMap<String, String> formData = mock(MultivaluedMap.class);
        when(formData.getFirst(Utils.CODE)).thenReturn("123456");
        when(context.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        when(authSession.getAuthNote(Utils.CODE)).thenReturn("123456");
        when(authSession.getAuthNote(Utils.CODE_TTL)).thenReturn(Long.toString(System.currentTimeMillis() + 60000));

        authenticator.action(context);
        verify(context).success();
    }

    @Test
    public void testAction_InvalidCode() {
        MultivaluedMap<String, String> formData = mock(MultivaluedMap.class);
        when(formData.getFirst(Utils.CODE)).thenReturn("654321");
        when(context.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        when(authSession.getAuthNote(Utils.CODE)).thenReturn("123456");
        when(authSession.getAuthNote(Utils.CODE_TTL)).thenReturn(Long.toString(System.currentTimeMillis() + 60000));

        authenticator.action(context);
        verify(context).failureChallenge(eq(AuthenticationFlowError.INVALID_CREDENTIALS), any(Response.class));
    }

    @Test
    public void testAction_ExpiredCode() {
        MultivaluedMap<String, String> formData = mock(MultivaluedMap.class);
        when(formData.getFirst(Utils.CODE)).thenReturn("123456");
        when(context.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        when(authSession.getAuthNote(Utils.CODE)).thenReturn("123456");
        when(authSession.getAuthNote(Utils.CODE_TTL)).thenReturn(Long.toString(System.currentTimeMillis() - 60000));

        authenticator.action(context);
        verify(context).failureChallenge(eq(AuthenticationFlowError.EXPIRED_CODE), any(Response.class));
    }
}
 
