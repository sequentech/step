// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.email.EmailException;
import org.keycloak.email.EmailTemplateProvider;
import org.keycloak.models.*;
import org.keycloak.services.resources.RealmsResource;
import org.keycloak.sessions.AuthenticationSessionModel;

import jakarta.ws.rs.core.UriInfo;
import java.util.OptionalInt;

public class SmartLinkTest {

    private KeycloakSession session;
    private RealmModel realm;
    private UserModel user;
    private AuthenticationSessionModel authSession;
    private EmailTemplateProvider emailTemplateProvider;
    private UriInfo uriInfo;

    @BeforeEach
    public void setup() {
        session = mock(KeycloakSession.class);
        realm = mock(RealmModel.class);
        user = mock(UserModel.class);
        authSession = mock(AuthenticationSessionModel.class);
        emailTemplateProvider = mock(EmailTemplateProvider.class);
        uriInfo = mock(UriInfo.class);

        when(session.getContext()).thenReturn(mock(KeycloakContext.class));
        when(session.getContext().getUri()).thenReturn(uriInfo);
        when(session.getContext().getRealm()).thenReturn(realm);
        when(session.getProvider(EmailTemplateProvider.class)).thenReturn(emailTemplateProvider);
        when(user.getId()).thenReturn("user-id");
    }

    @Test
    public void testCreateActionToken() {
        SmartLinkActionToken token = SmartLink.createActionToken(
                user, 
                "client-id", 
                OptionalInt.of(3600), 
                false, 
                authSession, 
                true, 
                true);

        assertNotNull(token);
        assertEquals("user-id", token.getUserId());
    }

    @Test
    public void testGetOrCreate() {
        when(KeycloakModelUtils.findUserByNameOrEmail(session, realm, "test@example.com")).thenReturn(null);
        when(session.users().addUser(realm, "test@example.com")).thenReturn(user);

        UserModel result = SmartLink.getOrCreate(session, realm, "test@example.com", true, true, true, userModel -> {});
        assertNotNull(result);
        verify(session.users()).addUser(realm, "test@example.com");
    }

    @Test
    public void testLinkFromActionToken() {
        SmartLinkActionToken token = new SmartLinkActionToken("user-id", 3600, "nonce", "client-id", true, "redirect-uri", "scopes", "state", false, true);
        when(uriInfo.getBaseUri()).thenReturn(URI.create("http://localhost"));

        String link = SmartLink.linkFromActionToken(session, realm, token);
        assertNotNull(link);
        assertTrue(link.contains("http://localhost"));
    }

    @Test
    public void testSendSmartLinkNotification() throws EmailException {
        when(realm.getName()).thenReturn("realmName");

        boolean result = SmartLink.sendSmartLinkNotification(session, user, "http://link");
        assertTrue(result);
        verify(emailTemplateProvider).send(
                anyString(), 
                anyList(), 
                anyString(), 
                anyMap());
    }
}
