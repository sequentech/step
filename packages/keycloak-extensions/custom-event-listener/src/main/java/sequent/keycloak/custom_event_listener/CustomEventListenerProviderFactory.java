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

@AutoService(EventListenerProviderFactory.class)
public class CustomEventListenerProviderFactory implements EventListenerProviderFactory {

  private final RabbitMqEventPublisher rabbitMqEventPublisher;

  public CustomEventListenerProviderFactory() {
    this(RabbitMqEventPublisher.fromEnvironment());
  }

  CustomEventListenerProviderFactory(RabbitMqEventPublisher rabbitMqEventPublisher) {
    this.rabbitMqEventPublisher = rabbitMqEventPublisher;
  }

  @Override
  public EventListenerProvider create(KeycloakSession session) {
    return new CustomEventListenerProvider(session, rabbitMqEventPublisher);
  }

  @Override
  public void init(Scope config) {}

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public void close() {
    rabbitMqEventPublisher.close();
  }

  @Override
  public String getId() {
    return "custom-event-listener";
  }
}
