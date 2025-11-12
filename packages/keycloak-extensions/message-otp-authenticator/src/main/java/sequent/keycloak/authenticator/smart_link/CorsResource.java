// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link;

import jakarta.ws.rs.*;
import jakarta.ws.rs.core.Response;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.models.KeycloakSession;
import org.keycloak.services.cors.Cors;
import org.keycloak.services.resources.admin.AdminAuth;

@JBossLog
public class CorsResource {

  private final KeycloakSession session;

  public CorsResource(KeycloakSession session) {
    this.session = session;
  }

  public static final String[] METHODS = {
    "GET", "HEAD", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"
  };

  @OPTIONS
  @Path("{any:.*}")
  public Response preflight() {
    log.debug("CORS OPTIONS preflight request");
    return session
        .getProvider(Cors.class)
        .auth()
        .allowedMethods(METHODS)
        .preflight()
        .add(Response.ok());
  }

  public static void setupCors(KeycloakSession session, AdminAuth auth) {
    Cors cors = session.getProvider(Cors.class);
    cors.allowedOrigins(auth.getToken())
        .allowedMethods(METHODS)
        .exposedHeaders("Location")
        .auth()
        .add();
  }
}
