// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.idp_initiated_sso;

import jakarta.ws.rs.*;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import java.net.URI;
import java.net.URISyntaxException;
import org.jboss.logging.Logger;
import org.keycloak.http.HttpRequest;
import org.keycloak.models.ClientModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserSessionModel;
import org.keycloak.protocol.oidc.utils.RedirectUtils;
import org.keycloak.services.managers.AuthenticationManager;

public class SamlRedirectProvider implements org.keycloak.services.resource.RealmResourceProvider {

  private static final Logger logger = Logger.getLogger(SamlRedirectProvider.class);
  private final KeycloakSession session;

  public SamlRedirectProvider(KeycloakSession session) {
    logger.infof("Created redirect provider");

    this.session = session;
  }

  @Override
  public Object getResource() {
    return this;
  }

  /**
   * This method handles the POST request to /redirect-me It consumes a standard HTML form
   * submission.
   */
  @POST
  @Path("redirect")
  @Consumes(MediaType.APPLICATION_FORM_URLENCODED)
  @Produces(MediaType.APPLICATION_JSON)
  public Response handleRedirect(@FormParam("RelayState") String relayState) {

    RealmModel realm = session.getContext().getRealm();

    // Get remote address for security logging
    String remoteAddr = session.getContext().getConnection().getRemoteHost();

    // Log the incoming request with security context
    logger.infof(
        "SECURITY: Received redirect request - RelayState: %s, RemoteAddr: %s, Realm: %s",
        relayState, remoteAddr, realm.getName());

    // Get the client config to verify redirect uri's
    logger.infof("Current realm: %s", realm.getName());
    ClientModel vpssoClient = realm.getClientByClientId("vp-sso");
    logger.infof("Has vp-sso client? %b", vpssoClient != null);

    if (vpssoClient == null) {
      logger.errorf(
          "SECURITY: vp-sso client not found - Realm: %s",
          realm.getName());
      String jsonError = "{\"error\": \"Service configuration error.\"}";
      return Response.status(Response.Status.INTERNAL_SERVER_ERROR).entity(jsonError).build();
    }

    // Returns null if either the client is not set or the redirect uri is invalid.
    String verifiedUrl = RedirectUtils.verifyRedirectUri(session, relayState, vpssoClient);

    // Check if the RelayState was provided
    if (verifiedUrl == null || verifiedUrl.isEmpty()) {
      logger.warnf(
          "SECURITY: Invalid redirect URL - RelayState: %s, RemoteAddr: %s",
          relayState, remoteAddr);
      String jsonError =
          "{\"error\": \"RelayState has an invalid redirect uri or vp-sso saml client is not set.\"}";
      return Response.status(Response.Status.BAD_REQUEST).entity(jsonError).build();
    }

    // If it exists redirect to it
    try {
      URI location = new URI(verifiedUrl);
      logger.infof(
          "SECURITY: Successful redirect - Target: %s",
          verifiedUrl);
      return Response.status(Response.Status.FOUND) // 302 Redirect
          .location(location)
          .build();
    } catch (URISyntaxException e) {
      logger.errorf(
          e,
          "SECURITY: Invalid URI syntax - RelayState: %s, RemoteAddr: %s",
          relayState, remoteAddr);
      String jsonError = "{\"error\": \"Invalid RelayState URL format.\"}";
      return Response.status(Response.Status.BAD_REQUEST).entity(jsonError).build();
    }
  }

  @Override
  public void close() {
    // No-op
  }
}
