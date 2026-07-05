// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.login_bridge;

import jakarta.ws.rs.core.Response;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.OAuth2Constants;
import org.keycloak.authentication.actiontoken.AbstractActionTokenHandler;
import org.keycloak.authentication.actiontoken.ActionTokenContext;
import org.keycloak.events.*;
import org.keycloak.models.ClientModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.UserModel;
import org.keycloak.protocol.oidc.OIDCLoginProtocol;
import org.keycloak.protocol.oidc.utils.RedirectUtils;
import org.keycloak.services.managers.AuthenticationManager;
import org.keycloak.services.messages.Messages;
import org.keycloak.services.util.ResolveRelative;
import org.keycloak.sessions.AuthenticationSessionModel;

@JBossLog
public class LoginBridgeActionTokenHandler
    extends AbstractActionTokenHandler<LoginBridgeActionToken> {
  public LoginBridgeActionTokenHandler() {
    super(
        LoginBridgeActionToken.TOKEN_TYPE,
        LoginBridgeActionToken.class,
        Messages.INVALID_REQUEST,
        EventType.EXECUTE_ACTION_TOKEN,
        Errors.INVALID_REQUEST);
  }

  @Override
  public AuthenticationSessionModel startFreshAuthenticationSession(
      LoginBridgeActionToken token, ActionTokenContext<LoginBridgeActionToken> tokenContext) {
    return tokenContext.createAuthenticationSessionForClient(token.getIssuedFor());
  }

  @Override
  public boolean canUseTokenRepeatedly(
      LoginBridgeActionToken token, ActionTokenContext<LoginBridgeActionToken> tokenContext) {
    return Boolean.TRUE.equals(token.getPersistent());
  }

  @Override
  public Response handleToken(
      LoginBridgeActionToken token, ActionTokenContext<LoginBridgeActionToken> tokenContext) {
    log.debugf(
        "handleToken(): called with iss=%s, user=%s", token.getIssuedFor(), token.getUserId());
    UserModel user = tokenContext.getAuthenticationSession().getAuthenticatedUser();

    AuthenticationSessionModel authSession = tokenContext.getAuthenticationSession();
    KeycloakSession session = tokenContext.getSession();
    ClientModel client = authSession.getClient();
    String redirectUri =
        (token.getRedirectUri() != null)
            ? token.getRedirectUri()
            : ResolveRelative.resolveRelativeUri(session, client.getRootUrl(), client.getBaseUrl());
    log.debugf("handleToken(): redirectUri=%s", redirectUri);

    String shouldRedirect = RedirectUtils.verifyRedirectUri(session, redirectUri, client);

    if (shouldRedirect != null) {
      authSession.setAuthNote(
          AuthenticationManager.SET_REDIRECT_URI_AFTER_REQUIRED_ACTIONS, "true");
      authSession.setRedirectUri(shouldRedirect);
      authSession.setClientNote(OIDCLoginProtocol.REDIRECT_URI_PARAM, redirectUri);
      if (token.getState() != null) {
        authSession.setClientNote(OIDCLoginProtocol.STATE_PARAM, token.getState());
      }
      if (token.getActionVerificationNonce() != null) {
        authSession.setClientNote(
            OIDCLoginProtocol.NONCE_PARAM, token.getActionVerificationNonce().toString());
      }
    }

    if (token.getScopes() != null) {
      authSession.setClientNote(OAuth2Constants.SCOPE, token.getScopes());
      AuthenticationManager.setClientScopesInSession(session, authSession);
    }

    if (Boolean.TRUE.equals(token.getRememberMe())) {
      authSession.setAuthNote(Details.REMEMBER_ME, "true");
      tokenContext.getEvent().detail(Details.REMEMBER_ME, "true");
    } else {
      authSession.removeAuthNote(Details.REMEMBER_ME);
    }

    if (Boolean.TRUE.equals(token.getMarkEmailVerified())) {
      user.setEmailVerified(true);
    }

    String nextAction =
        AuthenticationManager.nextRequiredAction(
            tokenContext.getSession(),
            authSession,
            tokenContext.getRequest(),
            tokenContext.getEvent());
    return AuthenticationManager.redirectToRequiredActions(
        tokenContext.getSession(),
        tokenContext.getRealm(),
        authSession,
        tokenContext.getUriInfo(),
        nextAction);
  }
}
