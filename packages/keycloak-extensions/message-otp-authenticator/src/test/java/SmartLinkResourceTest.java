// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.mockito.Mock;
import org.mockito.MockitoAnnotations;
import org.keycloak.models.ClientModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.services.ErrorResponse;
import org.keycloak.services.ForbiddenException;
import org.keycloak.services.NotFoundException;
import org.keycloak.services.BadRequestException;
import org.keycloak.services.util.ResolveRelative;

import javax.ws.rs.core.MediaType;
import javax.ws.rs.core.Response;

import java.util.OptionalInt;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

public class SmartLinkResourceTest {

    @Mock
    private KeycloakSession session;

    @Mock
    private RealmModel realm;

    @Mock
    private ClientModel client;

    @Mock
    private UserModel user;

    @Mock
    private EventBuilder event;

    private SmartLinkResource smartLinkResource;

    @BeforeEach
    public void setUp() {
        MockitoAnnotations.openMocks(this);
        smartLinkResource = new SmartLinkResource(session);
    }

    @Test
    public void testCreateSmartLink_Success() {
        // Mocking request
        SmartLinkRequest request = new SmartLinkRequest();
        request.setUsername("testuser");
        request.setClientId("client1");
        request.setRedirectUri("http://example.com/redirect");
        request.setEmailOrUsername("test@example.com");

        // Mocking session methods
        when(session.clients().getClientByClientId(realm, "client1")).thenReturn(client);
        when(client.getRootUrl()).thenReturn("http://example.com");
        when(client.getBaseUrl()).thenReturn("/");
        when(SmartLink.validateRedirectUri(session, "http://example.com/redirect", client)).thenReturn(true);
        when(SmartLink.getOrCreate(session, realm, "test@example.com", false, false, false, event)).thenReturn(user);
        when(SmartLink.createActionToken(user, "client1", "http://example.com/redirect", OptionalInt.of(86400), null, null, null, false, true, true)).thenReturn(mock(SmartLinkActionToken.class));
        when(SmartLink.linkFromActionToken(session, realm, any())).thenReturn("http://example.com/smart-link");

        // Test
        SmartLinkResponse response = smartLinkResource.createSmartLink(request);

        assertNotNull(response);
        assertEquals(user.getId(), response.getUserId());
        assertEquals("http://example.com/smart-link", response.getLink());
        assertFalse(response.isSent());
    }

    @Test
    public void testCreateSmartLink_ClientNotFound() {
        SmartLinkRequest request = new SmartLinkRequest();
        request.setClientId("invalidClient");

        when(session.clients().getClientByClientId(realm, "invalidClient")).thenReturn(null);

        assertThrows(NotFoundException.class, () -> {
            smartLinkResource.createSmartLink(request);
        });
    }

    @Test
    public void testCreateSmartLink_InvalidRedirectUri() {
        SmartLinkRequest request = new SmartLinkRequest();
        request.setClientId("client1");
        request.setRedirectUri("http://invalid.com");

        when(session.clients().getClientByClientId(realm, "client1")).thenReturn(client);
        when(SmartLink.validateRedirectUri(session, "http://invalid.com", client)).thenReturn(false);

        assertThrows(BadRequestException.class, () -> {
            smartLinkResource.createSmartLink(request);
        });
    }

    @Test
    public void testCreateSmartLink_UserNotFound() {
        SmartLinkRequest request = new SmartLinkRequest();
        request.setEmailOrUsername("unknown@example.com");

        when(session.clients().getClientByClientId(realm, any())).thenReturn(client);
        when(SmartLink.validateRedirectUri(session, any(), any())).thenReturn(true);
        when(SmartLink.getOrCreate(session, realm, "unknown@example.com", false, false, false, event)).thenReturn(null);

        assertThrows(NotFoundException.class, () -> {
            smartLinkResource.createSmartLink(request);
        });
    }

    @Test
    public void testCreateSmartLink_Forbidden() {
        SmartLinkRequest request = new SmartLinkRequest();
        request.setClientId("client1");

        when(session.clients().getClientByClientId(realm, "client1")).thenReturn(client);
        when(SmartLink.validateRedirectUri(session, any(), any())).thenReturn(true);
        when(permissions.users().canManage()).thenReturn(false);

        assertThrows(ForbiddenException.class, () -> {
            smartLinkResource.createSmartLink(request);
        });
    }
}

