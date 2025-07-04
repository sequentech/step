// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

import jakarta.ws.rs.core.UriBuilder;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.models.ClientModel;
import org.mockito.ArgumentCaptor;

import jakarta.ws.rs.core.Response;
import java.net.URI;

/**
 * Unit tests for RedirectToRegisterAuthenticator.
 * Covers redirect logic, security (open redirect), and Keycloak Authenticator contract.
 */
public class RedirectToRegisterAuthenticatorTest {

    private RedirectToRegisterAuthenticator authenticator;
    private AuthenticationFlowContext context;
    private RealmModel realm;

    @BeforeEach
    void setUp() {
        // Set up mocks for context and realm
        authenticator = new RedirectToRegisterAuthenticator();
        context = mock(AuthenticationFlowContext.class);
        realm = mock(RealmModel.class);
        when(context.getRealm()).thenReturn(realm);
        when(realm.getName()).thenReturn("test-realm");
        // Mock base URI and request URI
        var uriInfo = mock(jakarta.ws.rs.core.UriInfo.class);
        when(uriInfo.getBaseUri()).thenReturn(URI.create("http://localhost:8080/auth/"));
        when(uriInfo.getRequestUri()).thenReturn(URI.create("http://localhost:8080/auth/realms/test-realm/protocol/openid-connect/auth"));
        when(uriInfo.getRequestUri().getQuery()).thenReturn(null);
        when(context.getUriInfo()).thenReturn(uriInfo);
    }

    @Test
    void testAuthenticateRedirectsToRegistration() {
        // Should redirect to the registration page for the correct realm
        ArgumentCaptor<Response> responseCaptor = ArgumentCaptor.forClass(Response.class);
        doNothing().when(context).challenge(responseCaptor.capture());

        authenticator.authenticate(context);

        Response response = responseCaptor.getValue();
        assertEquals(Response.Status.FOUND.getStatusCode(), response.getStatus());
        assertTrue(response.getLocation().toString().contains("/realms/test-realm/login-actions/registration"));
    }

    @Test
    void testAuthenticateWithQueryStringSkipsRedirectUri() {
        // Should NOT propagate redirect_uri param to prevent open redirect attacks
        var uriInfo = context.getUriInfo();
        // Mock getRequestUri() to return a URI with the query string
        when(uriInfo.getRequestUri()).thenReturn(URI.create("http://localhost:8080/auth/realms/test-realm/protocol/openid-connect/auth?redirect_uri=https://evil.com"));
        ArgumentCaptor<Response> responseCaptor = ArgumentCaptor.forClass(Response.class);
        doNothing().when(context).challenge(responseCaptor.capture());

        authenticator.authenticate(context);
        Response response = responseCaptor.getValue();
        assertFalse(response.getLocation().toString().contains("redirect_uri"));
    }

    @Test
    void testAuthenticateWithSafeQueryString() {
        // Should propagate safe query params to registration URL
        var uriInfo = context.getUriInfo();
        // Mock getRequestUri() to return a URI with the query string
        when(uriInfo.getRequestUri()).thenReturn(URI.create("http://localhost:8080/auth/realms/test-realm/protocol/openid-connect/auth?foo=bar"));
        ArgumentCaptor<Response> responseCaptor = ArgumentCaptor.forClass(Response.class);
        doNothing().when(context).challenge(responseCaptor.capture());

        authenticator.authenticate(context);
        Response response = responseCaptor.getValue();
        assertTrue(response.getLocation().toString().contains("foo=bar"));
    }

    @Test
    void testActionAlwaysSucceeds() {
        // The action method should always call context.success()
        doNothing().when(context).success();
        authenticator.action(context);
        verify(context, times(1)).success();
    }

    @Test
    void testRequiresUser() {
        // This authenticator never requires a user
        assertFalse(authenticator.requiresUser());
    }

    @Test
    void testConfiguredForAlwaysTrue() {
        // Always returns true for any session/user/realm
        KeycloakSession session = mock(KeycloakSession.class);
        UserModel user = mock(UserModel.class);
        assertTrue(authenticator.configuredFor(session, realm, user));
    }

    @Test
    void testSetRequiredActionsDoesNothing() {
        // Should be a no-op, but must not throw
        KeycloakSession session = mock(KeycloakSession.class);
        UserModel user = mock(UserModel.class);
        authenticator.setRequiredActions(session, realm, user);
    }

    @Test
    void testCloseDoesNothing() {
        // Should be a no-op, but must not throw
        authenticator.close();
    }
}
