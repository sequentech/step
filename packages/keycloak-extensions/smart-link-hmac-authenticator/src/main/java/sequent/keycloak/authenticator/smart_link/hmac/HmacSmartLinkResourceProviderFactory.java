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
 * Registers the HMAC Smart Link endpoint under {@code /realms/{realm}/smart-link}.
 *
 * <p>The provider id {@code smart-link} is scoped to the {@link RealmResourceProvider} SPI, so it
 * does not collide with the existing {@code smart-link} Authenticator or ActionTokenHandler
 * providers, which live in different SPIs.
 */
@AutoService(RealmResourceProviderFactory.class)
public class HmacSmartLinkResourceProviderFactory implements RealmResourceProviderFactory {

  public static final String PROVIDER_ID = "smart-link";

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
