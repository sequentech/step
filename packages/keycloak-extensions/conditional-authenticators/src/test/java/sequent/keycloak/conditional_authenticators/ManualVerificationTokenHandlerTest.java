// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.conditional_authenticators;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

import jakarta.ws.rs.BadRequestException;
import java.net.URI;
import java.util.Set;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.actiontoken.ActionTokenContext;
import org.keycloak.models.ClientModel;
import org.keycloak.models.KeycloakContext;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakUriInfo;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;

class ManualVerificationTokenHandlerTest {
  private ActionTokenContext<ManualVerificationToken> tokenContext;
  private AuthenticationSessionModel authSession;
  private UserModel user;
  private ManualVerificationTokenHandler handler;

  @BeforeEach
  void setUp() {
    tokenContext = mock(ActionTokenContext.class);
    authSession = mock(AuthenticationSessionModel.class);
    KeycloakSession session = mock(KeycloakSession.class);
    KeycloakContext context = mock(KeycloakContext.class);
    KeycloakUriInfo uri = mock(KeycloakUriInfo.class);
    ClientModel client = mock(ClientModel.class);
    user = mock(UserModel.class);
    handler = new ManualVerificationTokenHandler();
    when(tokenContext.getAuthenticationSession()).thenReturn(authSession);
    when(tokenContext.getSession()).thenReturn(session);
    when(authSession.getAuthenticatedUser()).thenReturn(user);
    when(authSession.getClient()).thenReturn(client);
    when(client.getRedirectUris()).thenReturn(Set.of("https://vote.example/*"));
    when(session.getContext()).thenReturn(context);
    when(context.getUri()).thenReturn(uri);
    when(uri.getBaseUri()).thenReturn(URI.create("https://id.example/"));
  }

  @Test
  void validPortalAndAbsentRedirectsPassRedirectVerifier() throws Exception {
    assertTrue(handler.getVerifiers(tokenContext)[0].test(token("https://vote.example/login")));
    assertTrue(handler.getVerifiers(tokenContext)[0].test(token(null)));
  }

  @Test
  void externalRedirectFailsRedirectVerifier() {
    assertThrows(
        org.keycloak.common.VerificationException.class,
        () -> handler.getVerifiers(tokenContext)[0].test(token("https://attacker.example/login")));
  }

  @Test
  void invalidRedirectDoesNotVerifyUserOrSetPasswordAction() {
    assertThrows(
        BadRequestException.class,
        () -> handler.handleToken(token("javascript:alert(1)"), tokenContext));
    verify(user, never()).setEmailVerified(anyBoolean());
    verify(user, never()).setAttribute(anyString(), any());
    verify(user, never()).addRequiredAction(anyString());
    verify(authSession, never()).setRedirectUri(anyString());
    verify(authSession, never()).addRequiredAction(anyString());
  }

  private ManualVerificationToken token(String redirect) {
    return new ManualVerificationToken("voter", 3600, redirect);
  }
}
