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

  private PgmqEventPublisher pgmqEventPublisher;
  private SigninNetworkDispatcher signinNetworkDispatcher;

  // Quarkus instantiates factories during image augmentation without runtime configuration.
  public CustomEventListenerProviderFactory() {}

  CustomEventListenerProviderFactory(PgmqEventPublisher pgmqEventPublisher) {
    this.pgmqEventPublisher = pgmqEventPublisher;
  }

  @Override
  public EventListenerProvider create(KeycloakSession session) {
    return new CustomEventListenerProvider(session, pgmqEventPublisher, signinNetworkDispatcher);
  }

  @Override
  public void init(Scope config) {}

  @Override
  public void postInit(KeycloakSessionFactory factory) {
    // Exclusive provider settings can prevent either privacy factory from being loaded.
    // Enforce activation from this existing SPI as well, before any authentication is served.
    PrivacyEventStoreProviderFactory.requireSelected(factory);
    PrivacyUserSessionProviderFactory.requireSelected(factory);
    if (pgmqEventPublisher == null) {
      pgmqEventPublisher = PgmqEventPublisher.fromEnvironment();
    }
    pgmqEventPublisher.checkQueueDatabase();
    signinNetworkDispatcher = new SigninNetworkDispatcher(SigninNetworkSender.fromEnvironment());
  }

  @Override
  public void close() {
    if (signinNetworkDispatcher != null) {
      signinNetworkDispatcher.close();
    }
    if (pgmqEventPublisher != null) {
      pgmqEventPublisher.close();
    }
  }

  @Override
  public String getId() {
    return "custom-event-listener";
  }
}
