// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.login_bridge;

import lombok.extern.jbosslog.JBossLog;
import org.keycloak.http.HttpRequest;
import org.keycloak.models.KeycloakSession;
import org.keycloak.services.resource.RealmResourceProvider;

@JBossLog
public abstract class BaseRealmResourceProvider implements RealmResourceProvider {

  protected final KeycloakSession session;

  public BaseRealmResourceProvider(KeycloakSession session) {
    this.session = session;
  }

  @Override
  public void close() {}

  protected abstract Object getRealmResource();

  @Override
  public Object getResource() {
    HttpRequest request = session.getContext().getHttpRequest();
    String method = request == null ? null : request.getHttpMethod();
    log.debugf("request method %s", method);
    if ("OPTIONS".equals(method)) {
      return new CorsResource(session);
    } else {
      return getRealmResource();
    }
  }
}
