// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.login_timeout;

import jakarta.ws.rs.GET;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.QueryParam;
import jakarta.ws.rs.core.CacheControl;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import java.util.Map;
import org.keycloak.authentication.AuthenticationProcessor;
import org.keycloak.common.util.Time;
import org.keycloak.cookie.CookieProvider;
import org.keycloak.cookie.CookieType;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.services.managers.AuthenticationSessionManager;
import org.keycloak.services.managers.ClientSessionCode;
import org.keycloak.services.resource.RealmResourceProvider;
import org.keycloak.services.resources.LoginActionsService;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.sessions.RootAuthenticationSessionModel;

/**
 * Serves {@code GET /realms/{realm}/login-timeout?tab_id=...}: how many seconds the browser's
 * current login has left before Keycloak rejects its next form with "login timeout". Login pages
 * use it to start over before the voter types into an expired form.
 *
 * <p>The login is found through the browser's own authentication session cookie, so a caller only
 * learns about its own login. Reading it does not extend it. Without that cookie, or for a tab
 * whose login is gone, the answer is 204 No Content.
 */
public class LoginTimeoutResourceProvider implements RealmResourceProvider {
  static final String SECONDS_LEFT = "secondsLeft";

  private final KeycloakSession session;

  public LoginTimeoutResourceProvider(KeycloakSession session) {
    this.session = session;
  }

  @Override
  public Object getResource() {
    return this;
  }

  @Override
  public void close() {}

  @GET
  @Path("/")
  @Produces(MediaType.APPLICATION_JSON)
  public Response getSecondsLeft(@QueryParam("tab_id") String tabId) {
    CacheControl noStore = new CacheControl();
    noStore.setNoStore(true);
    if (session.getProvider(CookieProvider.class).get(CookieType.AUTH_SESSION_ID) == null) {
      // No login to ask about, for example with cookies blocked. Answering "expired" would make
      // the page restart over and over.
      return Response.noContent().cacheControl(noStore).build();
    }
    RealmModel realm = session.getContext().getRealm();
    RootAuthenticationSessionModel rootSession =
        new AuthenticationSessionManager(session).getCurrentRootAuthenticationSession(realm);
    if (rootSession == null) {
      // Keycloak drops a login once it expires.
      return Response.ok(Map.of(SECONDS_LEFT, 0)).cacheControl(noStore).build();
    }
    AuthenticationSessionModel authSession =
        tabId == null ? null : rootSession.getAuthenticationSessions().get(tabId);
    if (tabId != null && authSession == null) {
      // That tab's login already finished or was replaced: nothing to restart.
      return Response.noContent().cacheControl(noStore).build();
    }
    ClientSessionCode.ActionType actionType =
        authSession == null
            ? ClientSessionCode.ActionType.LOGIN
            : actionTypeOf(
                authSession.getAction(),
                authSession.getAuthNote(AuthenticationProcessor.CURRENT_FLOW_PATH));
    int secondsLeft =
        secondsLeft(rootSession.getTimestamp(), lifespanOf(realm, actionType), Time.currentTime());
    return Response.ok(Map.of(SECONDS_LEFT, secondsLeft)).cacheControl(noStore).build();
  }

  /** The lifespan Keycloak's login actions check the next form against. */
  static ClientSessionCode.ActionType actionTypeOf(String action, String flowPath) {
    if (AuthenticationSessionModel.Action.REQUIRED_ACTIONS.name().equals(action)
        || LoginActionsService.RESET_CREDENTIALS_PATH.equals(flowPath)) {
      return ClientSessionCode.ActionType.USER;
    }
    return ClientSessionCode.ActionType.LOGIN;
  }

  static int lifespanOf(RealmModel realm, ClientSessionCode.ActionType actionType) {
    return lifespanOf(
        actionType,
        realm.getAccessCodeLifespanLogin(),
        realm.getAccessCodeLifespanUserAction(),
        realm.getAccessCodeLifespan());
  }

  /** Mirrors {@link ClientSessionCode#isActionActive}'s choice of lifespan. */
  static int lifespanOf(
      ClientSessionCode.ActionType actionType, int login, int userAction, int client) {
    return switch (actionType) {
      case USER -> userAction;
      case CLIENT -> client;
      // A realm without its own login timeout uses the action timeout.
      case LOGIN -> login > 0 ? login : userAction;
    };
  }

  /** Mirrors {@link ClientSessionCode#isActionActive}: active while lifespan > now - timestamp. */
  static int secondsLeft(int timestamp, int lifespan, int now) {
    return Math.max(0, timestamp + lifespan - now);
  }
}
