// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link.hmac;

import com.google.auto.service.AutoService;
import org.keycloak.Config;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.services.resource.RealmResourceProvider;
import org.keycloak.services.resource.RealmResourceProviderFactory;

/**
 * Registers the HMAC Smart Link endpoint under {@code /realms/{realm}/election}.
 *
 * <p>The provider id {@code election} gives the public URL the same shape as the first-generation
 * Smart Link route while still living under Keycloak's realm resource namespace.
 */
@AutoService(RealmResourceProviderFactory.class)
public class HmacSmartLinkResourceProviderFactory implements RealmResourceProviderFactory {

  public static final String PROVIDER_ID = "election";

  @Override
  public RealmResourceProvider create(KeycloakSession session) {
    return new HmacSmartLinkProvider(session);
  }

  @Override
  public void init(Config.Scope config) {}

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public void close() {}

  @Override
  public String getId() {
    return PROVIDER_ID;
  }
}
