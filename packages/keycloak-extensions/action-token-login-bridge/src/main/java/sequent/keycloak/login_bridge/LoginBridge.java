// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.login_bridge;

import jakarta.ws.rs.core.UriBuilder;
import jakarta.ws.rs.core.UriInfo;
import java.net.URI;
import java.util.OptionalInt;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.Config;
import org.keycloak.common.util.Time;
import org.keycloak.models.ClientModel;
import org.keycloak.models.Constants;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.protocol.oidc.OIDCLoginProtocol;
import org.keycloak.protocol.oidc.utils.RedirectUtils;
import org.keycloak.services.Urls;
import org.keycloak.services.resources.LoginActionsService;
import org.keycloak.services.resources.RealmsResource;
import org.keycloak.sessions.AuthenticationSessionModel;

/** Neutral helper for turning a trusted user/client pair into a Keycloak login session. */
@JBossLog
public final class LoginBridge {

  private LoginBridge() {}

  public static LoginBridgeActionToken createActionToken(
      UserModel user,
      String clientId,
      OptionalInt validity,
      Boolean rememberMe,
      AuthenticationSessionModel authSession,
      Boolean isActionTokenPersistent,
      Boolean markEmailVerified) {
    String redirectUri = authSession.getRedirectUri();
    String scopes = authSession.getClientNote(OIDCLoginProtocol.SCOPE_PARAM);
    String state = authSession.getClientNote(OIDCLoginProtocol.STATE_PARAM);
    String nonce = authSession.getClientNote(OIDCLoginProtocol.NONCE_PARAM);
    log.debugf(
        "Creating login bridge token for user %s, %s, %s", user.getId(), clientId, redirectUri);
    log.debugf("Login bridge token extra vars %s %s %s %b", scopes, state, nonce, rememberMe);
    return createActionToken(
        user,
        clientId,
        redirectUri,
        validity,
        scopes,
        nonce,
        state,
        rememberMe,
        isActionTokenPersistent,
        markEmailVerified);
  }

  public static LoginBridgeActionToken createActionToken(
      UserModel user,
      String clientId,
      String redirectUri,
      OptionalInt validity,
      String scopes,
      String nonce,
      String state,
      Boolean rememberMe,
      Boolean persistent,
      Boolean markEmailVerified) {
    int validityInSecs = validity.orElse(60 * 60 * 24);
    int absoluteExpirationInSecs = Time.currentTime() + validityInSecs;
    return new LoginBridgeActionToken(
        user.getId(),
        absoluteExpirationInSecs,
        nonce,
        clientId,
        markEmailVerified,
        redirectUri,
        scopes,
        state,
        rememberMe,
        persistent);
  }

  public static String linkFromActionToken(
      KeycloakSession session, RealmModel realm, LoginBridgeActionToken token) {
    UriInfo uriInfo = session.getContext().getUri();

    RealmModel originalRealm = session.getContext().getRealm();
    log.debugf(
        "realm %s session.context.realm %s",
        realm.getName(), originalRealm == null ? null : originalRealm.getName());

    if (Config.getAdminRealm().equals(realm.getName())) {
      throw new IllegalStateException(
          String.format("Login bridge tokens not allowed for %s realm", Config.getAdminRealm()));
    }
    session.getContext().setRealm(realm);
    try {
      UriBuilder builder =
          actionTokenBuilder(
              uriInfo.getBaseUri(), token.serialize(session, realm, uriInfo), token.getIssuedFor());
      return builder.build(realm.getName()).toString();
    } finally {
      session.getContext().setRealm(originalRealm);
    }
  }

  public static boolean validateRedirectUri(
      KeycloakSession session, String redirectUri, ClientModel client) {
    String redirect = RedirectUtils.verifyRedirectUri(session, redirectUri, client);
    log.debugf("Redirect after verify %s -> %s", redirectUri, redirect);
    return redirectUri.equals(redirect);
  }

  private static UriBuilder actionTokenBuilder(URI baseUri, String tokenString, String clientId) {
    log.debugf("baseUri: %s, tokenString: %s, clientId: %s", baseUri, tokenString, clientId);
    return Urls.realmBase(baseUri)
        .path(RealmsResource.class, "getLoginActionsService")
        .path(LoginActionsService.class, "executeActionToken")
        .queryParam(Constants.KEY, tokenString)
        .queryParam(Constants.CLIENT_ID, clientId);
  }
}
