// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import jakarta.ws.rs.BadRequestException;
import jakarta.ws.rs.ForbiddenException;
import jakarta.ws.rs.GET;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.QueryParam;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import jakarta.ws.rs.core.UriBuilder;
import jakarta.ws.rs.ext.Provider;
import java.util.HashMap;
import java.util.Map;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.common.util.Time;
import org.keycloak.models.ClientModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.UserModel;
import org.keycloak.protocol.oidc.utils.RedirectUtils;
import org.keycloak.services.resource.RealmResourceProvider;
import org.keycloak.services.resources.LoginActionsService;
import sequent.keycloak.authenticator.smart_link.AbstractAdminResource;

/*
 * curl -v http://127.0.0.1:8090/realms/master/manual-verification/ping | jq -C .
 * curl -v "http://127.0.0.1:8090/realms/master/manual-verification/generate-link?userId=d0924713-5b74-4ce5-b675-2cff1d5c8b91&redirectUri=http://127.0.0.1:3002" | jq -C .
 */
@JBossLog
@Provider
public class ManualVerificationProvider extends AbstractAdminResource
    implements RealmResourceProvider {
  static final String CLIENT_ID = "voting-portal";

  public ManualVerificationProvider(KeycloakSession session) {
    super(session);
  }

  @GET
  @Path("/ping")
  public Object ping() {
    log.info("ping");
    Map<String, String> response = new HashMap<>();
    response.put("answer", "pong");
    return Response.ok(response).build();
  }

  @GET
  @Path("/generate-link")
  @Produces(MediaType.APPLICATION_JSON)
  public Response generateLink(
      @QueryParam("userId") String userId, @QueryParam("redirectUri") String redirectUri) {
    log.info("generateLink");
    checkPermissions();

    String tokenLink = generateTokenLink(userId, redirectUri);
    if (tokenLink != null) {
      Map<String, String> response = new HashMap<>();
      response.put("link", tokenLink);
      return Response.ok(response).build();
    }
    return Response.status(Response.Status.NOT_FOUND).build();
  }

  private String generateTokenLink(String userId, String redirectUri) {
    log.info("generateTokenLink(): start");
    ClientModel client = session.clients().getClientByClientId(realm, CLIENT_ID);
    if (client == null || !client.isEnabled()) {
      throw new BadRequestException("Voting portal client is unavailable");
    }
    if (redirectUri != null) {
      redirectUri = RedirectUtils.verifyRedirectUri(session, redirectUri, client);
      if (redirectUri == null) {
        throw new BadRequestException("Invalid redirect URI");
      }
    }
    int lifespan = realm.getActionTokenGeneratedByAdminLifespan();
    int expiration = Time.currentTime() + lifespan;
    UserModel user = session.users().getUserById(realm, userId);

    if (user != null) {
      log.info(
          "generateTokenLink(): user found at realm="
              + realm.getName()
              + " with id="
              + userId
              + ", and username="
              + user.getUsername());

      // Generate the token
      ManualVerificationToken mvToken =
          new ManualVerificationToken(user.getId(), expiration, redirectUri);
      mvToken.issuedFor(client.getClientId());
      UriBuilder builder = LoginActionsService.actionTokenProcessor(session.getContext().getUri());
      builder.queryParam("key", mvToken.serialize(session, realm, session.getContext().getUri()));
      return builder.build(realm.getName()).toString();
    } else {
      log.info("generateTokenLink(): No user at realm=" + realm.getName() + " with id=" + userId);
    }
    return null;
  }

  /*
   * Throws some exception if the user does not have the admin role.
   */
  private void checkPermissions() {
    // Authenticate the issuer realm, then evaluate permissions on this target realm.
    // setup() temporarily switches the context for bearer verification.
    try {
      setup();
    } finally {
      session.getContext().setRealm(realm);
    }
    if (!permissions.users().canManage()) {
      throw new ForbiddenException("Manual verification requires manage-users");
    }
  }

  @Override
  public Object getResource() {
    log.info("getResource");
    return this;
  }

  @Override
  public void close() {
    log.info("close");
  }
}
