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
import jakarta.ws.rs.PathParam;
import jakarta.ws.rs.QueryParam;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import java.io.IOException;
import java.util.Optional;
import java.util.function.BiFunction;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.models.KeycloakSession;
import org.keycloak.services.resource.RealmResourceProvider;
import sequent.keycloak.authenticator.diagnostics.ScanovateDiagnostics;

/**
 * Endpoints called server to server by the on-premise Scanovate services, OCR and Liveness Plus,
 * see {@link OnPremSessions}: {@code <service>/verify} is their {@code
 * onprem.token_verification_url} and {@code <service>/callback} their {@code onprem.callback_url}.
 *
 * <p>They are only meant for the services: they check a secret that the voter's browser never sees,
 * and should not be exposed by the public reverse proxy either. Their tokens are not tied to a
 * realm, so any realm can serve them.
 */
@JBossLog
public class IdentityVerificationResource implements RealmResourceProvider {
  static final String VERIFY_PATH = "{service}/verify";
  static final String CALLBACK_PATH = "{service}/callback";
  static final String SECRET_QUERY_PARAM = "secret";

  private static final ObjectMapper MAPPER = new ObjectMapper();

  private final KeycloakSession session;
  private final BiFunction<OnPremService, KeycloakSession, OnPremSessions> sessionsFactory;

  public IdentityVerificationResource(KeycloakSession session) {
    this(session, ScanovateAuthenticator::defaultSessions);
  }

  IdentityVerificationResource(
      KeycloakSession session,
      BiFunction<OnPremService, KeycloakSession, OnPremSessions> sessionsFactory) {
    this.session = session;
    this.sessionsFactory = sessionsFactory;
  }

  @Override
  public Object getResource() {
    return this;
  }

  /**
   * Token verification of a service ({@code onprem.token_verification_url}): 200 lets the voter
   * start a session, 401 rejects it.
   */
  @GET
  @Path(VERIFY_PATH)
  public Response verifyToken(
      @PathParam("service") String path,
      @HeaderParam("X-token") String token,
      @HeaderParam("case-id") String caseId,
      @QueryParam(SECRET_QUERY_PARAM) String secret) {
    Optional<OnPremService> service = OnPremService.fromPath(path);
    if (service.isEmpty()) {
      return json(Response.Status.NOT_FOUND);
    }
    if (!sessionsFactory.apply(service.get(), session).verify(token, caseId, secret)) {
      log.warnv(
          "verifyToken: rejected a {0} token for case {1}",
          service.get().path(), ScanovateDiagnostics.caseId(caseId));
      return json(Response.Status.UNAUTHORIZED);
    }
    return json(Response.Status.OK);
  }

  /** Start and result callbacks of a service ({@code onprem.callback_url}). */
  @POST
  @Path(CALLBACK_PATH)
  @Consumes(MediaType.APPLICATION_JSON)
  public Response callback(
      @PathParam("service") String path,
      @HeaderParam("x-token") String token,
      @QueryParam(SECRET_QUERY_PARAM) String secret,
      String body) {
    Optional<OnPremService> service = OnPremService.fromPath(path);
    if (service.isEmpty()) {
      return json(Response.Status.NOT_FOUND);
    }
    JsonNode json;
    try {
      json = MAPPER.readTree(body);
    } catch (IOException | IllegalArgumentException e) {
      log.warnv("callback: invalid {0} JSON body", service.get().path());
      return json(Response.Status.BAD_REQUEST);
    }
    if (token == null && json != null) {
      JsonNode bodyToken = json.at("/onprem_params/token");
      token = bodyToken.isTextual() ? bodyToken.asText() : null;
    }
    return switch (sessionsFactory.apply(service.get(), session).record(token, secret, json)) {
      case RECORDED, IGNORED -> json(Response.Status.OK);
      case UNAUTHORIZED -> {
        log.warnv("callback: rejected an unauthorized {0} callback", service.get().path());
        yield json(Response.Status.UNAUTHORIZED);
      }
      case INVALID -> {
        log.warnv("callback: rejected an invalid {0} callback", service.get().path());
        yield json(Response.Status.BAD_REQUEST);
      }
    };
  }

  /**
   * A response with a small JSON body. Keycloak's security headers turn a response without a media
   * type into a 500, which the services would read as a rejected token or a failed callback.
   */
  private static Response json(Response.Status status) {
    return Response.status(status)
        .entity("{\"status\":\"" + status.getReasonPhrase() + "\"}")
        .type(MediaType.APPLICATION_JSON_TYPE)
        .build();
  }

  @Override
  public void close() {}
}
