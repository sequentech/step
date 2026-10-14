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
import jakarta.ws.rs.PUT;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.PathParam;
import jakarta.ws.rs.QueryParam;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import java.io.IOException;
import java.io.InputStream;
import java.util.function.Function;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.models.KeycloakSession;
import org.keycloak.services.resource.RealmResourceProvider;

/**
 * The endpoint the capture page uploads its files to, see {@link CaptureUploads}, and endpoints
 * called server to server by the Liveness Plus service, see {@link LivenessSessions}.
 *
 * <p>The liveness endpoints are only meant for Liveness Plus: they check a secret that the voter's
 * browser never sees, and should not be exposed by the public reverse proxy either. Their tokens
 * are not tied to a realm, so any realm can serve them. The same goes for the capture tokens.
 */
@JBossLog
public class IdentityVerificationResource implements RealmResourceProvider {
  static final String LIVENESS_VERIFY_PATH = "liveness/verify";
  static final String LIVENESS_CALLBACK_PATH = "liveness/callback";
  static final String SECRET_QUERY_PARAM = "secret";
  static final String CAPTURE_PATH = "capture";
  static final String CAPTURE_TOKEN_HEADER = "X-Capture-Token";

  private static final ObjectMapper MAPPER = new ObjectMapper();

  private final KeycloakSession session;
  private final Function<KeycloakSession, LivenessSessions> livenessFactory;
  private final Function<KeycloakSession, CaptureUploads> uploadsFactory;

  public IdentityVerificationResource(KeycloakSession session) {
    this(session, ScanovateAuthenticator::defaultLiveness, ScanovateAuthenticator::defaultUploads);
  }

  IdentityVerificationResource(
      KeycloakSession session,
      Function<KeycloakSession, LivenessSessions> livenessFactory,
      Function<KeycloakSession, CaptureUploads> uploadsFactory) {
    this.session = session;
    this.livenessFactory = livenessFactory;
    this.uploadsFactory = uploadsFactory;
  }

  @Override
  public Object getResource() {
    return this;
  }

  /**
   * A file of the capture page, one of the {@link MediaKind#formPart()} parts, uploaded as the raw
   * request body with the page's capture token in the {@value #CAPTURE_TOKEN_HEADER} header. Its
   * format is detected from its content, whatever its content type says.
   */
  @PUT
  @Path(CAPTURE_PATH + "/{part}")
  public Response uploadCapture(
      @PathParam("part") String part,
      @HeaderParam(CAPTURE_TOKEN_HEADER) String token,
      InputStream body) {
    byte[] content;
    try {
      content = body == null ? null : body.readNBytes(CaptureUploads.MAX_UPLOAD_BYTES + 1);
    } catch (IOException e) {
      log.warn("uploadCapture: could not read the upload");
      return json(Response.Status.BAD_REQUEST);
    }
    CaptureUploads.UploadOutcome outcome =
        uploadsFactory.apply(session).store(token, part, content);
    if (outcome != CaptureUploads.UploadOutcome.STORED) {
      log.warnv(
          "uploadCapture: rejected the {0} upload: {1}",
          MediaKind.fromFormPart(part).map(MediaKind::formPart).orElse("unknown"), outcome);
    }
    return switch (outcome) {
      case STORED -> json(Response.Status.OK);
      case UNAUTHORIZED -> json(Response.Status.UNAUTHORIZED);
      case UNKNOWN_PART, EMPTY -> json(Response.Status.BAD_REQUEST);
      case TOO_LARGE -> json(Response.Status.REQUEST_ENTITY_TOO_LARGE);
      case INVALID_FORMAT -> json(Response.Status.UNSUPPORTED_MEDIA_TYPE);
    };
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
      return json(Response.Status.UNAUTHORIZED);
    }
    return json(Response.Status.OK);
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
      return json(Response.Status.BAD_REQUEST);
    }
    if (token == null && json != null) {
      JsonNode bodyToken = json.at("/onprem_params/token");
      token = bodyToken.isTextual() ? bodyToken.asText() : null;
    }
    return switch (livenessFactory.apply(session).record(token, secret, json)) {
      case RECORDED, IGNORED -> json(Response.Status.OK);
      case UNAUTHORIZED -> {
        log.warn("livenessCallback: rejected an unauthorized callback");
        yield json(Response.Status.UNAUTHORIZED);
      }
      case INVALID -> {
        log.warn("livenessCallback: rejected an invalid callback");
        yield json(Response.Status.BAD_REQUEST);
      }
    };
  }

  /**
   * A response with a small JSON body. Keycloak's security headers turn a response without a media
   * type into a 500, which Liveness Plus would read as a rejected token or a failed callback.
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
