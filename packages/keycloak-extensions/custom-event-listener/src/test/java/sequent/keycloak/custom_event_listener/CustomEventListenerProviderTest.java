// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.doThrow;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import com.rabbitmq.client.AMQP;
import java.util.Collections;
import org.junit.jupiter.api.Test;
import org.keycloak.events.Event;
import org.keycloak.events.EventType;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.RealmProvider;

class CustomEventListenerProviderTest {

  @Test
  void propagatesOutboxFailureInsteadOfSilentlyDroppingTheAuditEvent() {
    KeycloakSession session = mock(KeycloakSession.class);
    RealmProvider realmProvider = mock(RealmProvider.class);
    RealmModel realm = mock(RealmModel.class);
    RabbitMqEventPublisher publisher = mock(RabbitMqEventPublisher.class);
    Event event = new Event();
    event.setType(EventType.LOGIN);
    event.setRealmId("realm-id");
    event.setDetails(Collections.emptyMap());
    when(session.realms()).thenReturn(realmProvider);
    when(realmProvider.getRealm("realm-id")).thenReturn(realm);
    when(realm.getName()).thenReturn("tenant-tenant-id-event-election-id");
    doThrow(new AuditEventPersistenceException("outbox unavailable"))
        .when(publisher)
        .publish(eq(session), any(AMQP.BasicProperties.class), any(byte[].class));
    CustomEventListenerProvider provider = new CustomEventListenerProvider(session, publisher);

    assertThrows(AuditEventPersistenceException.class, () -> provider.onEvent(event));
  }
}
