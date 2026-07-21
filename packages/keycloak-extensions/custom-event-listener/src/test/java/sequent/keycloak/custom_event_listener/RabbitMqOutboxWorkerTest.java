// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import com.rabbitmq.client.AMQP;
import com.rabbitmq.client.Channel;
import com.rabbitmq.client.Connection;
import com.rabbitmq.client.ConnectionFactory;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.Optional;
import java.util.concurrent.TimeoutException;
import java.util.function.IntToLongFunction;
import java.util.function.LongSupplier;
import java.util.function.Supplier;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.models.KeycloakSessionFactory;
import org.mockito.InOrder;

class RabbitMqOutboxWorkerTest {

  private static final String AMQP_URI = "amqp://guest:guest@rabbitmq:5672/%2f";
  private static final String QUEUE_NAME = "test_electoral_log_event_queue";
  private static final long NOW = 10_000L;
  private static final long CLAIM_LEASE_MILLIS = 60_000L;
  private static final long CONFIRM_TIMEOUT_MILLIS = 5_000L;

  private AuditOutboxStore outboxStore;
  private Supplier<ConnectionFactory> connectionFactorySupplier;
  private ConnectionFactory connectionFactory;
  private Connection connection;
  private Channel channel;
  private KeycloakSessionFactory sessionFactory;
  private RabbitMqOutboxMessage message;
  private RabbitMqOutboxWorker worker;

  @SuppressWarnings("unchecked")
  @BeforeEach
  void setUp() throws Exception {
    outboxStore = mock(AuditOutboxStore.class);
    connectionFactorySupplier = mock(Supplier.class);
    connectionFactory = mock(ConnectionFactory.class);
    connection = mock(Connection.class);
    channel = mock(Channel.class);
    sessionFactory = mock(KeycloakSessionFactory.class);
    LongSupplier clock = () -> NOW;
    IntToLongFunction retryDelay = ignored -> 1_000L;

    when(connectionFactorySupplier.get()).thenReturn(connectionFactory);
    when(connectionFactory.newConnection()).thenReturn(connection);
    when(connection.createChannel()).thenReturn(channel);
    when(connection.isOpen()).thenReturn(true);
    when(channel.isOpen()).thenReturn(true);

    message =
        new RabbitMqOutboxMessage(
            "event-id",
            QUEUE_NAME,
            "audit-task",
            "event".getBytes(StandardCharsets.UTF_8),
            1,
            "claim-token");
    worker =
        new RabbitMqOutboxWorker(
            AMQP_URI, outboxStore, connectionFactorySupplier, clock, retryDelay);
  }

  @Test
  void publishesWithConfirmsBeforeDeletingTheOutboxEntry() throws Exception {
    when(outboxStore.claimNext(sessionFactory, NOW, CLAIM_LEASE_MILLIS))
        .thenReturn(Optional.of(message));

    RabbitMqOutboxWorker.DeliveryResult result = worker.deliverNext(sessionFactory);

    assertEquals(RabbitMqOutboxWorker.DeliveryResult.DELIVERED, result);
    verify(channel).queueDeclare(QUEUE_NAME, true, false, false, null);
    verify(channel).confirmSelect();
    verify(channel)
        .basicPublish(
            eq(""), eq(QUEUE_NAME), eq(true), any(AMQP.BasicProperties.class), eq(message.body()));
    InOrder confirmedBeforeDelete = org.mockito.Mockito.inOrder(channel, outboxStore);
    confirmedBeforeDelete.verify(channel).waitForConfirmsOrDie(CONFIRM_TIMEOUT_MILLIS);
    confirmedBeforeDelete
        .verify(outboxStore)
        .markDelivered(sessionFactory, "event-id", "claim-token");
    verify(outboxStore, never())
        .markFailed(any(), any(), any(), org.mockito.ArgumentMatchers.anyLong(), any());
  }

  @Test
  void retainsAndSchedulesAnEventForRetryWhenConfirmationFails() throws Exception {
    when(outboxStore.claimNext(sessionFactory, NOW, CLAIM_LEASE_MILLIS))
        .thenReturn(Optional.of(message));
    org.mockito.Mockito.doThrow(new TimeoutException("confirm timeout"))
        .when(channel)
        .waitForConfirmsOrDie(CONFIRM_TIMEOUT_MILLIS);

    RabbitMqOutboxWorker.DeliveryResult result = worker.deliverNext(sessionFactory);

    assertEquals(RabbitMqOutboxWorker.DeliveryResult.FAILED, result);
    verify(outboxStore, never()).markDelivered(any(), any(), any());
    verify(outboxStore)
        .markFailed(sessionFactory, "event-id", "claim-token", NOW + 1_000L, "confirm timeout");
    verify(channel).abort();
    verify(connection).abort();
  }

  @Test
  void retriesTheSameDurableEventAndDeletesItOnlyAfterConfirmation() throws Exception {
    ConnectionFactory replacementFactory = mock(ConnectionFactory.class);
    Connection replacementConnection = mock(Connection.class);
    Channel replacementChannel = mock(Channel.class);
    when(outboxStore.claimNext(sessionFactory, NOW, CLAIM_LEASE_MILLIS))
        .thenReturn(Optional.of(message), Optional.of(message));
    when(connectionFactorySupplier.get()).thenReturn(connectionFactory, replacementFactory);
    when(replacementFactory.newConnection()).thenReturn(replacementConnection);
    when(replacementConnection.createChannel()).thenReturn(replacementChannel);
    when(replacementConnection.isOpen()).thenReturn(true);
    when(replacementChannel.isOpen()).thenReturn(true);
    org.mockito.Mockito.doThrow(new IOException("connection lost"))
        .doNothing()
        .when(channel)
        .basicPublish(
            eq(""), eq(QUEUE_NAME), eq(true), any(AMQP.BasicProperties.class), eq(message.body()));

    assertEquals(RabbitMqOutboxWorker.DeliveryResult.FAILED, worker.deliverNext(sessionFactory));
    assertEquals(RabbitMqOutboxWorker.DeliveryResult.DELIVERED, worker.deliverNext(sessionFactory));

    verify(connectionFactorySupplier, times(2)).get();
    verify(replacementChannel).waitForConfirmsOrDie(CONFIRM_TIMEOUT_MILLIS);
    verify(outboxStore).markDelivered(sessionFactory, "event-id", "claim-token");
  }

  @Test
  void configuresBoundedConnectionAndRpcTimeouts() throws Exception {
    when(outboxStore.claimNext(sessionFactory, NOW, CLAIM_LEASE_MILLIS))
        .thenReturn(Optional.of(message));

    worker.deliverNext(sessionFactory);

    verify(connectionFactory).setConnectionTimeout(5_000);
    verify(connectionFactory).setHandshakeTimeout(5_000);
    verify(connectionFactory).setChannelRpcTimeout(5_000);
    verify(connectionFactory).setShutdownTimeout(1_000);
    verify(connectionFactory).setAutomaticRecoveryEnabled(false);
    verify(connectionFactory).setTopologyRecoveryEnabled(false);
  }
}
