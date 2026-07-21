// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import com.google.auto.service.AutoService;
import org.keycloak.Config.Scope;
import org.keycloak.connections.jpa.entityprovider.JpaEntityProvider;
import org.keycloak.connections.jpa.entityprovider.JpaEntityProviderFactory;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;

@AutoService(JpaEntityProviderFactory.class)
public class AuditOutboxJpaEntityProviderFactory implements JpaEntityProviderFactory {

  @Override
  public JpaEntityProvider create(KeycloakSession session) {
    return new AuditOutboxJpaEntityProvider();
  }

  @Override
  public void init(Scope config) {}

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public void close() {}

  @Override
  public String getId() {
    return AuditOutboxJpaEntityProvider.FACTORY_ID;
  }
}
