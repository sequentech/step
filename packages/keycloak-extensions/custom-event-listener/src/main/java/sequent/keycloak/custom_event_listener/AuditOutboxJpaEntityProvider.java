// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import java.util.List;
import org.keycloak.connections.jpa.entityprovider.JpaEntityProvider;

class AuditOutboxJpaEntityProvider implements JpaEntityProvider {

  static final String FACTORY_ID = "sequent-audit-outbox";

  @Override
  public List<Class<?>> getEntities() {
    return List.of(AuditOutboxEntity.class);
  }

  @Override
  public String getChangelogLocation() {
    return "META-INF/audit-outbox-changelog.xml";
  }

  @Override
  public String getFactoryId() {
    return FACTORY_ID;
  }

  @Override
  public void close() {}
}
