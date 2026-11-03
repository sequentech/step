// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import com.google.auto.service.AutoService;
import java.util.Objects;
import org.keycloak.Config;
import org.keycloak.events.EventStoreProvider;
import org.keycloak.events.EventStoreProviderFactory;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;

@AutoService(EventStoreProviderFactory.class)
public final class PrivacyEventStoreProviderFactory implements EventStoreProviderFactory {
  public static final String ID = "sequent-privacy";

  @Override
  public EventStoreProvider create(KeycloakSession session) {
    EventStoreProvider jpa =
        Objects.requireNonNull(
            session.getProvider(EventStoreProvider.class, "jpa"),
            "The privacy event store requires the JPA delegate");
    return new PrivacyEventStoreProvider(session, jpa);
  }

  @Override
  public void init(Config.Scope config) {}

  @Override
  public void postInit(KeycloakSessionFactory factory) {
    requireSelected(factory);
  }

  static void requireSelected(KeycloakSessionFactory factory) {
    if (!(factory.getProviderFactory(EventStoreProvider.class)
        instanceof PrivacyEventStoreProviderFactory)) {
      throw new IllegalStateException(
          "The sequent-privacy event store must be selected before authentication is served");
    }
    if (factory.getProviderFactory(EventStoreProvider.class, "jpa") == null) {
      throw new IllegalStateException(
          "The privacy event store requires JPA; use spi-events-store--provider-default, not provider");
    }
  }

  @Override
  public String getId() {
    return ID;
  }

  @Override
  public void close() {}
}
