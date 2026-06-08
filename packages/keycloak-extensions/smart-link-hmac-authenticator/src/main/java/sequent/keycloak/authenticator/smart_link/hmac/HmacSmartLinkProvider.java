// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link.hmac;

import org.keycloak.models.KeycloakSession;
import sequent.keycloak.login_bridge.BaseRealmResourceProvider;

/**
 * Realm resource provider for the HMAC Smart Link endpoint.
 *
 * <p>Extends {@link BaseRealmResourceProvider} so OPTIONS preflight requests are answered by the
 * shared CORS resource and everything else is routed to {@link HmacSmartLinkResource}.
 */
public class HmacSmartLinkProvider extends BaseRealmResourceProvider {

  public HmacSmartLinkProvider(KeycloakSession session) {
    super(session);
  }

  @Override
  protected Object getRealmResource() {
    return new HmacSmartLinkResource(session);
  }
}
