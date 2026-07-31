// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.conditional_authenticators;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import jakarta.ws.rs.core.Response;
import jakarta.ws.rs.core.UriInfo;
import java.net.URI;
import java.util.LinkedHashMap;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.models.ClientModel;
import org.keycloak.models.RealmModel;
import org.keycloak.protocol.oidc.endpoints.AuthorizationEndpoint;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.mockito.ArgumentCaptor;

class RedirectToRegisterAuthenticatorTest {

  @Test
  void redirectKeepsTheAuthenticationSessionAndItsLoginHintClientNotes() {
    AuthenticationFlowContext context = mock(AuthenticationFlowContext.class);
    AuthenticationSessionModel authenticationSession = mock(AuthenticationSessionModel.class);
    ClientModel client = mock(ClientModel.class);
    RealmModel realm = mock(RealmModel.class);
    UriInfo uriInfo = mock(UriInfo.class);
    Map<String, String> clientNotes = new LinkedHashMap<>();
    clientNotes.put(clientNote("username"), "voter@example.com");

    when(context.getUriInfo()).thenReturn(uriInfo);
    when(uriInfo.getBaseUri()).thenReturn(URI.create("https://id.example/auth/"));
    when(context.getAuthenticationSession()).thenReturn(authenticationSession);
    when(authenticationSession.getClient()).thenReturn(client);
    when(authenticationSession.getTabId()).thenReturn("same-tab");
    when(authenticationSession.getClientNotes()).thenReturn(clientNotes);
    when(client.getClientId()).thenReturn("voting-portal");
    when(context.getRealm()).thenReturn(realm);
    when(realm.getName()).thenReturn("election");

    new RedirectToRegisterAuthenticator().authenticate(context);

    ArgumentCaptor<Response> responseCaptor = ArgumentCaptor.forClass(Response.class);
    verify(context).challenge(responseCaptor.capture());
    Response response = responseCaptor.getValue();
    String location = response.getLocation().toString();

    assertEquals(Response.Status.FOUND.getStatusCode(), response.getStatus());
    assertTrue(location.contains("/realms/election/login-actions/registration"));
    assertTrue(location.contains("client_id=voting-portal"));
    assertTrue(location.contains("tab_id=same-tab"));
    assertEquals(Map.of(clientNote("username"), "voter@example.com"), clientNotes);
    verify(authenticationSession, never()).setClientNote(anyString(), anyString());
    verify(authenticationSession, never()).removeClientNote(anyString());
  }

  private static String clientNote(String attributeName) {
    return AuthorizationEndpoint.LOGIN_SESSION_NOTE_ADDITIONAL_REQ_PARAMS_PREFIX
        + "login_hint__"
        + attributeName;
  }
}
