// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import jakarta.ws.rs.GET;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.core.Response;
import java.net.URI;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.models.KeycloakSession;
import org.keycloak.services.resource.RealmResourceProvider;
import org.keycloak.services.resources.LoginActionsService;

/** Endpoint B-Trust redirects the voter to, see {@link ReturnUrl}. */
@JBossLog
public class ScanovateReturnResource implements RealmResourceProvider {
  static final String RETURN_PATH = "return";

  private final KeycloakSession session;

  public ScanovateReturnResource(KeycloakSession session) {
    this.session = session;
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

  @Override
  public void close() {}
}
