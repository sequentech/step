// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.verifyNoInteractions;

import com.rabbitmq.client.AMQP;
import java.nio.charset.StandardCharsets;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.mockito.ArgumentCaptor;

class RabbitMqEventPublisherTest {

  private static final String QUEUE_NAME = "test_electoral_log_event_queue";

  @Test
  void durablyEnqueuesWithoutContactingRabbitMq() {
    AuditOutboxStore outboxStore = mock(AuditOutboxStore.class);
    RabbitMqOutboxWorker worker = mock(RabbitMqOutboxWorker.class);
    RabbitMqEventPublisher publisher = new RabbitMqEventPublisher(QUEUE_NAME, outboxStore, worker);
    KeycloakSession session = mock(KeycloakSession.class);
    byte[] body = "event".getBytes(StandardCharsets.UTF_8);
    AMQP.BasicProperties properties =
        new AMQP.BasicProperties.Builder()
            .correlationId("event-id")
            .headers(Map.of("task", "audit-task"))
            .build();

    publisher.publish(session, properties, body);

    ArgumentCaptor<RabbitMqOutboxMessage> messageCaptor =
        ArgumentCaptor.forClass(RabbitMqOutboxMessage.class);
    verify(outboxStore).enqueue(eq(session), messageCaptor.capture());
    RabbitMqOutboxMessage message = messageCaptor.getValue();
    assertEquals("event-id", message.id());
    assertEquals(QUEUE_NAME, message.queueName());
    assertEquals("audit-task", message.taskName());
    assertArrayEquals(body, message.body());
    verifyNoInteractions(worker);
  }

  @Test
  void startsAndStopsTheBackgroundWorker() {
    AuditOutboxStore outboxStore = mock(AuditOutboxStore.class);
    RabbitMqOutboxWorker worker = mock(RabbitMqOutboxWorker.class);
    RabbitMqEventPublisher publisher = new RabbitMqEventPublisher(QUEUE_NAME, outboxStore, worker);
    KeycloakSessionFactory sessionFactory = mock(KeycloakSessionFactory.class);

    publisher.start(sessionFactory);
    publisher.close();

    verify(worker).start(sessionFactory);
    verify(worker).close();
  }
}
