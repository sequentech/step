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
 * learns about its own login. Reading it does not extend it.
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
    RealmModel realm = session.getContext().getRealm();
    RootAuthenticationSessionModel rootSession =
        new AuthenticationSessionManager(session).getCurrentRootAuthenticationSession(realm);
    int secondsLeft = 0;
    if (rootSession != null) {
      AuthenticationSessionModel authSession =
          tabId == null ? null : rootSession.getAuthenticationSessions().get(tabId);
      if (tabId == null || authSession != null) {
        ClientSessionCode.ActionType actionType =
            authSession == null
                ? ClientSessionCode.ActionType.LOGIN
                : actionTypeOf(
                    authSession.getAction(),
                    authSession.getAuthNote(AuthenticationProcessor.CURRENT_FLOW_PATH));
        secondsLeft =
            secondsLeft(
                rootSession.getTimestamp(), lifespanOf(realm, actionType), Time.currentTime());
      }
    }
    CacheControl noStore = new CacheControl();
    noStore.setNoStore(true);
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
    return switch (actionType) {
      case USER -> realm.getAccessCodeLifespanUserAction();
      case CLIENT -> realm.getAccessCodeLifespan();
      case LOGIN -> realm.getAccessCodeLifespanLogin();
    };
  }

  /** Mirrors {@link ClientSessionCode#isActionActive}: active while lifespan > now - timestamp. */
  static int secondsLeft(int timestamp, int lifespan, int now) {
    return Math.max(0, timestamp + lifespan - now);
  }
}
