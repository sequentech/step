// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link.hmac;

import jakarta.ws.rs.GET;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.QueryParam;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import java.net.URI;
import java.util.OptionalInt;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.common.util.Time;
import org.keycloak.models.ClientModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import sequent.keycloak.authenticator.smart_link.SmartLink;
import sequent.keycloak.authenticator.smart_link.SmartLinkActionToken;

/**
 * Public, unauthenticated endpoint that consumes an externally generated HMAC Smart Link and logs
 * the voter in.
 *
 * <p>Exposed at {@code GET /realms/{realm}/smart-link/login?auth-token=<urlencoded khmac>}. This is
 * the second-generation analogue of the first-generation {@code /election/<eid>/public/login}
 * route, hosted inside Keycloak because that is where the per-event shared secret, the census
 * (realm users) and the session all live.
 *
 * <p>Design: this resource only does the security-critical work — validate the symmetric token and
 * resolve the user. To actually establish the session and produce the OIDC code it bridges into the
 * existing, audited Smart Link <em>action token</em> machinery ({@link SmartLink#createActionToken}
 * / {@link SmartLink#linkFromActionToken}) by issuing a short-lived, single-use internal token and
 * 302-redirecting the browser to it. No session-bootstrapping logic is re-implemented here.
 */
@JBossLog
public class HmacSmartLinkResource {

  private final KeycloakSession session;

  public HmacSmartLinkResource(KeycloakSession session) {
    this.session = session;
  }

  private static final String OPENID_SCOPE = "openid";
  private static final String DEFAULT_CLIENT_ID = "voting-portal";

  /** The internal bridge token is consumed immediately by the redirect chain, so keep it short. */
  private static final int INTERNAL_TOKEN_VALIDITY_SECONDS = 60;

  @GET
  @Path("login")
  @Produces(MediaType.APPLICATION_JSON)
  public Response login(
      @QueryParam("auth-token") String authToken,
      @QueryParam("redirect_uri") String redirectUriParam) {

    RealmModel realm = session.getContext().getRealm();
    String realmName = realm.getName();

    String sharedSecret = realm.getAttribute(HmacSmartLink.ATTR_SHARED_SECRET);
    long timeout =
        readLong(realm, HmacSmartLink.ATTR_TIMEOUT_SECONDS, HmacSmartLink.DEFAULT_TIMEOUT_SECONDS);
    long clockSkew =
        readLong(
            realm,
            HmacSmartLink.ATTR_CLOCK_SKEW_SECONDS,
            HmacSmartLink.DEFAULT_CLOCK_SKEW_SECONDS);
    String expectedEventId = HmacSmartLink.electionEventIdFromRealm(realmName);
    long now = Time.currentTime();

    // Step 1: validate the symmetric token (envelope, signature, event binding, time window).
    HmacSmartLink.ValidatedSmartLink validated;
    try {
      validated =
          HmacSmartLink.validate(authToken, sharedSecret, expectedEventId, now, timeout, clockSkew);
    } catch (SmartLinkValidationException error) {
      log.warnf(
          "SmartLink HMAC rejected: error=%s detail=%s realm=%s",
          error.getError(), error.getMessage(), realmName);
      return genericError();
    }

    // Step 2: resolve the OIDC client and the census user, then bridge to an internal action token.
    try {
      String clientId = orDefault(realm.getAttribute(HmacSmartLink.ATTR_CLIENT_ID), DEFAULT_CLIENT_ID);
      ClientModel client = session.clients().getClientByClientId(realm, clientId);
      if (client == null) {
        log.warnf(
            "SmartLink HMAC rejected: error=%s client=%s realm=%s",
            SmartLinkError.CLIENT_NOT_FOUND, clientId, realmName);
        return genericError();
      }

      // Default false: like the first generation, an unknown user id is an authorization failure
      // (the user must be in the census), not a silent account creation.
      boolean forceCreate = Boolean.parseBoolean(realm.getAttribute(HmacSmartLink.ATTR_FORCE_CREATE));

      UserModel user =
          SmartLink.getOrCreate(session, realm, validated.userId(), forceCreate, false, false, null);
      if (user == null) {
        log.warnf(
            "SmartLink HMAC rejected: error=%s realm=%s", SmartLinkError.USER_NOT_FOUND, realmName);
        return genericError();
      }
      if (!user.isEnabled()) {
        log.warnf(
            "SmartLink HMAC rejected: error=%s realm=%s", SmartLinkError.USER_DISABLED, realmName);
        return genericError();
      }

      // Optional caller-supplied redirect, validated against the client. Defaults to null so the
      // action token handler resolves the client's own base URL (the event's voting portal).
      String redirectUri = null;
      if (redirectUriParam != null && !redirectUriParam.isEmpty()) {
        if (!SmartLink.validateRedirectUri(session, redirectUriParam, client)) {
          log.warnf(
              "SmartLink HMAC rejected: error=%s redirect_uri=%s realm=%s",
              SmartLinkError.REDIRECT_NOT_ALLOWED, redirectUriParam, realmName);
          return genericError();
        }
        redirectUri = redirectUriParam;
      }

      // Internal bridge token: short-lived and NOT persistent, so it cannot be replayed.
      SmartLinkActionToken token =
          SmartLink.createActionToken(
              user,
              clientId,
              redirectUri,
              OptionalInt.of(INTERNAL_TOKEN_VALIDITY_SECONDS),
              OPENID_SCOPE,
              /* nonce= */ null,
              /* state= */ null,
              /* rememberMe= */ false,
              /* persistent= */ false,
              /* markEmailVerified= */ true);
      String link = SmartLink.linkFromActionToken(session, realm, token);

      return Response.status(Response.Status.FOUND).location(URI.create(link)).build();
    } catch (Exception error) {
      log.warnf(
          error, "SmartLink HMAC internal error: error=%s realm=%s",
          SmartLinkError.INTERNAL_ERROR, realmName);
      return genericError();
    }
  }

  /**
   * A single, vague error for every failure mode. Mirrors the first generation: never reveal which
   * check failed, to avoid handing an attacker an oracle (valid user ids, secret proximity, etc.).
   */
  private static Response genericError() {
    return Response.status(Response.Status.UNAUTHORIZED)
        .type(MediaType.APPLICATION_JSON)
        .entity("{\"error\":\"authentication_failed\"}")
        .build();
  }

  private static long readLong(RealmModel realm, String attribute, long defaultValue) {
    String raw = realm.getAttribute(attribute);
    if (raw == null || raw.isEmpty()) {
      return defaultValue;
    }
    try {
      return Long.parseLong(raw.trim());
    } catch (NumberFormatException e) {
      log.warnf("Ignoring non-numeric realm attribute %s=%s", attribute, raw);
      return defaultValue;
    }
  }

  private static String orDefault(String value, String defaultValue) {
    return (value == null || value.isEmpty()) ? defaultValue : value;
  }
}
