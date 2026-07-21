// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.verifyNoInteractions;

import org.junit.jupiter.api.Test;
import org.keycloak.events.EventListenerProvider;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;

class CustomEventListenerProviderFactoryTest {

  @Test
  void ownsOnePublisherForAllProviderInstances() {
    RabbitMqEventPublisher publisher = mock(RabbitMqEventPublisher.class);
    CustomEventListenerProviderFactory factory = new CustomEventListenerProviderFactory(publisher);

    EventListenerProvider first = factory.create(mock(KeycloakSession.class));
    EventListenerProvider second = factory.create(mock(KeycloakSession.class));

    first.close();
    second.close();
    verifyNoInteractions(publisher);

    factory.close();
    verify(publisher).close();
  }

  @Test
  void startsTheOutboxWorkerAfterKeycloakInitialization() {
    RabbitMqEventPublisher publisher = mock(RabbitMqEventPublisher.class);
    CustomEventListenerProviderFactory factory = new CustomEventListenerProviderFactory(publisher);
    KeycloakSessionFactory sessionFactory = mock(KeycloakSessionFactory.class);

    factory.postInit(sessionFactory);

    verify(publisher).start(sessionFactory);
  }
}
