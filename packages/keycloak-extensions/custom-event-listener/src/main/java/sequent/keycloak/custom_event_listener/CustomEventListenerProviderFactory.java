// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.custom_event_listener;

import com.google.auto.service.AutoService;
import org.keycloak.Config.Scope;
import org.keycloak.events.EventListenerProvider;
import org.keycloak.events.EventListenerProviderFactory;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.utils.KeycloakModelUtils;

@AutoService(EventListenerProviderFactory.class)
public class CustomEventListenerProviderFactory implements EventListenerProviderFactory {

  private PgmqEventPublisher pgmqEventPublisher;

  // Quarkus instantiates factories during image augmentation without runtime configuration.
  public CustomEventListenerProviderFactory() {}

  CustomEventListenerProviderFactory(PgmqEventPublisher pgmqEventPublisher) {
    this.pgmqEventPublisher = pgmqEventPublisher;
  }

  @Override
  public EventListenerProvider create(KeycloakSession session) {
    return new CustomEventListenerProvider(session, pgmqEventPublisher);
  }

  @Override
  public void init(Scope config) {}

  @Override
  public void postInit(KeycloakSessionFactory factory) {
    if (pgmqEventPublisher == null) {
      pgmqEventPublisher = PgmqEventPublisher.fromEnvironment();
    }
    KeycloakModelUtils.runJobInTransaction(factory, pgmqEventPublisher::initialize);
  }

  @Override
  public void close() {}

  @Override
  public String getId() {
    return "custom-event-listener";
  }
}
