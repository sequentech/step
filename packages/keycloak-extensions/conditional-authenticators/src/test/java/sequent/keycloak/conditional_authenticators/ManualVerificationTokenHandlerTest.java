// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.conditional_authenticators;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

import java.net.URI;
import java.util.Set;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.TokenVerifier.Predicate;
import org.keycloak.authentication.actiontoken.ActionTokenContext;
import org.keycloak.common.VerificationException;
import org.keycloak.models.ClientModel;
import org.keycloak.models.KeycloakContext;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakUriInfo;
import org.keycloak.services.messages.Messages;
import org.keycloak.sessions.AuthenticationSessionModel;

class ManualVerificationTokenHandlerTest {
  private ActionTokenContext<ManualVerificationToken> tokenContext;
  private ManualVerificationTokenHandler handler;

  @BeforeEach
  void setUp() {
    tokenContext = mock(ActionTokenContext.class);
    AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
    KeycloakSession session = mock(KeycloakSession.class);
    KeycloakContext context = mock(KeycloakContext.class);
    KeycloakUriInfo uri = mock(KeycloakUriInfo.class);
    ClientModel client = mock(ClientModel.class);
    handler = new ManualVerificationTokenHandler();
    when(tokenContext.getAuthenticationSession()).thenReturn(authSession);
    when(tokenContext.getSession()).thenReturn(session);
    when(authSession.getClient()).thenReturn(client);
    when(client.getRedirectUris()).thenReturn(Set.of("https://vote.example/*"));
    when(session.getContext()).thenReturn(context);
    when(context.getUri()).thenReturn(uri);
    when(uri.getBaseUri()).thenReturn(URI.create("https://id.example/"));
  }

  @Test
  void validPortalAndAbsentRedirectsPassVerifiers() throws Exception {
    assertTrue(passesVerifiers(token("https://vote.example/login")));
    assertTrue(passesVerifiers(token(null)));
  }

  @Test
  void externalRedirectFailsVerifiers() {
    assertInvalidRedirect("https://untrusted.example/login");
  }

  @Test
  void scriptRedirectFailsVerifiers() {
    assertInvalidRedirect("javascript:alert(1)");
  }

  private void assertInvalidRedirect(String redirect) {
    VerificationException error =
        assertThrows(VerificationException.class, () -> passesVerifiers(token(redirect)));
    assertEquals(Messages.INVALID_REDIRECT_URI, error.getMessage());
  }

  private boolean passesVerifiers(ManualVerificationToken token) throws VerificationException {
    for (Predicate<? super ManualVerificationToken> verifier : handler.getVerifiers(tokenContext)) {
      if (!verifier.test(token)) {
        return false;
      }
    }
    return true;
  }

  private ManualVerificationToken token(String redirect) {
    return new ManualVerificationToken("voter", 3600, redirect);
  }
}
