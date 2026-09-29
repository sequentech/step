// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import jakarta.ws.rs.Consumes;
import jakarta.ws.rs.GET;
import jakarta.ws.rs.HeaderParam;
import jakarta.ws.rs.POST;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.QueryParam;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import java.io.IOException;
import java.net.URI;
import java.util.function.Function;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.models.KeycloakSession;
import org.keycloak.services.resource.RealmResourceProvider;
import org.keycloak.services.resources.LoginActionsService;

/**
 * Endpoint B-Trust redirects the voter to, see {@link ReturnUrl}, and endpoints called server to
 * server by the Liveness Plus service, see {@link LivenessSessions}.
 *
 * <p>The liveness endpoints are only meant for Liveness Plus: they check a secret that the voter's
 * browser never sees, and should not be exposed by the public reverse proxy either. Their tokens
 * are not tied to a realm, so any realm can serve them.
 */
@JBossLog
public class ScanovateReturnResource implements RealmResourceProvider {
  static final String RETURN_PATH = "return";
  static final String LIVENESS_VERIFY_PATH = "liveness/verify";
  static final String LIVENESS_CALLBACK_PATH = "liveness/callback";
  static final String SECRET_QUERY_PARAM = "secret";

  private static final ObjectMapper MAPPER = new ObjectMapper();

  private final KeycloakSession session;
  private final Function<KeycloakSession, LivenessSessions> livenessFactory;

  public ScanovateReturnResource(KeycloakSession session) {
    this(session, ScanovateAuthenticator::defaultLiveness);
  }

  ScanovateReturnResource(
      KeycloakSession session, Function<KeycloakSession, LivenessSessions> livenessFactory) {
    this.session = session;
    this.livenessFactory = livenessFactory;
  }

  @Override
  public Object getResource() {
    return this;
  }

  @GET
  @Path(RETURN_PATH)
  public Response returnFromBTrust() {
    URI loginActionsBase =
        LoginActionsService.loginActionsBaseUrl(session.getContext().getUri())
            .build(session.getContext().getRealm().getName());
    return ReturnUrl.toActionUrl(
            loginActionsBase, session.getContext().getUri().getQueryParameters())
        .map(url -> Response.seeOther(url).build())
        .orElseGet(
            () -> {
              log.warn("returnFromBTrust: invalid return URL");
              return Response.status(Response.Status.BAD_REQUEST).build();
            });
  }

  /**
   * Token verification of Liveness Plus ({@code onprem.token_verification_url}): 200 lets the voter
   * start a liveness session, 401 rejects it.
   */
  @GET
  @Path(LIVENESS_VERIFY_PATH)
  public Response verifyLivenessToken(
      @HeaderParam("X-token") String token,
      @HeaderParam("case-id") String caseId,
      @QueryParam(SECRET_QUERY_PARAM) String secret) {
    if (!livenessFactory.apply(session).verify(token, caseId, secret)) {
      log.warnv("verifyLivenessToken: rejected a token for case {0}", caseId);
      return Response.status(Response.Status.UNAUTHORIZED).build();
    }
    return Response.ok().build();
  }

  /** Start and result callbacks of Liveness Plus ({@code onprem.callback_url}). */
  @POST
  @Path(LIVENESS_CALLBACK_PATH)
  @Consumes(MediaType.APPLICATION_JSON)
  public Response livenessCallback(
      @HeaderParam("x-token") String token,
      @QueryParam(SECRET_QUERY_PARAM) String secret,
      String body) {
    JsonNode json;
    try {
      json = MAPPER.readTree(body);
    } catch (IOException e) {
      log.warn("livenessCallback: invalid JSON body");
      return Response.status(Response.Status.BAD_REQUEST).build();
    }
    if (token == null && json != null) {
      JsonNode bodyToken = json.at("/onprem_params/token");
      token = bodyToken.isTextual() ? bodyToken.asText() : null;
    }
    return switch (livenessFactory.apply(session).record(token, secret, json)) {
      case RECORDED, IGNORED -> Response.ok().build();
      case UNAUTHORIZED -> {
        log.warn("livenessCallback: rejected an unauthorized callback");
        yield Response.status(Response.Status.UNAUTHORIZED).build();
      }
      case INVALID -> {
        log.warn("livenessCallback: rejected an invalid callback");
        yield Response.status(Response.Status.BAD_REQUEST).build();
      }
    };
  }

  @Override
  public void close() {}
}
