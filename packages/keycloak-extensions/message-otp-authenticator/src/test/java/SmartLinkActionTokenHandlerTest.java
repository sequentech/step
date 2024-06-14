// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

import jakarta.ws.rs.core.Response;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.actiontoken.ActionTokenContext;
import org.keycloak.models.*;
import org.keycloak.protocol.oidc.OIDCLoginProtocol;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.mockito.Mock;
import org.mockito.MockitoAnnotations;

public class SmartLinkActionTokenHandlerTest {

    private SmartLinkActionTokenHandler handler;

    @Mock
    private ActionTokenContext<SmartLinkActionToken> tokenContext;

    @Mock
    private AuthenticationSessionModel authSession;

    @Mock
    private KeycloakSession session;

    @Mock
    private ClientModel client;

    @Mock
    private UserModel user;

    @BeforeEach
    public void setup() {
        MockitoAnnotations.openMocks(this);
        handler = new SmartLinkActionTokenHandler();

        when(tokenContext.getSession()).thenReturn(session);
        when(tokenContext.getAuthenticationSession()).thenReturn(authSession);
        when(authSession.getClient()).thenReturn(client);
        when(authSession.getAuthenticatedUser()).thenReturn(user);
    }

    @Test
    public void testStartFreshAuthenticationSession() {
        SmartLinkActionToken token = mock(SmartLinkActionToken.class);
        when(token.getIssuedFor()).thenReturn("client-id");

        AuthenticationSessionModel result = handler.startFreshAuthenticationSession(token, tokenContext);
        assertNotNull(result);
    }

    @Test
    public void testCanUseTokenRepeatedly() {
        SmartLinkActionToken token = mock(SmartLinkActionToken.class);
        when(token.getPersistent()).thenReturn(true);

        assertTrue(handler.canUseTokenRepeatedly(token, tokenContext));
        when(token.getPersistent()).thenReturn(false);
        assertFalse(handler.canUseTokenRepeatedly(token, tokenContext));
    }

    @Test
    public void testHandleToken() {
        SmartLinkActionToken token = new SmartLinkActionToken(
            "user-id", 3600, "nonce", "client-id", true, "http://localhost", "openid", "state123", true, true
        );

        when(client.getRootUrl()).thenReturn("http://localhost");
        when(client.getBaseUrl()).thenReturn("/base");

        Response response = handler.handleToken(token, tokenContext);

        assertNotNull(response);
        verify(user).setEmailVerified(true);
        verify(authSession).setAuthNote(AuthenticationManager.SET_REDIRECT_URI_AFTER_REQUIRED_ACTIONS, "true");
    }
}
