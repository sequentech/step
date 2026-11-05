// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import com.rabbitmq.client.AMQP;
import com.rabbitmq.client.Channel;
import com.rabbitmq.client.Connection;
import com.rabbitmq.client.ConnectionFactory;
import com.rabbitmq.client.ReturnListener;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.time.Duration;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.RejectedExecutionException;
import java.util.concurrent.TimeUnit;
import java.util.function.Supplier;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

class RabbitMqEventPublisherTest {

  private static final String AMQP_URI = "amqp://guest:guest@rabbitmq:5672/%2f";
  private static final String QUEUE_NAME = "test_electoral_log_event_queue";

  private Supplier<ConnectionFactory> connectionFactorySupplier;
  private ConnectionFactory connectionFactory;
  private Connection connection;
  private Channel channel;
  private RabbitMqEventPublisher publisher;

  @SuppressWarnings("unchecked")
  @BeforeEach
  void setUp() throws Exception {
    connectionFactorySupplier = mock(Supplier.class);
    connectionFactory = mock(ConnectionFactory.class);
    connection = mock(Connection.class);
    channel = mock(Channel.class);

    when(connectionFactorySupplier.get()).thenReturn(connectionFactory);
    when(connectionFactory.newConnection()).thenReturn(connection);
    when(connection.createChannel()).thenReturn(channel);
    when(connection.isOpen()).thenReturn(true);
    when(channel.isOpen()).thenReturn(true);

    publisher = new RabbitMqEventPublisher(AMQP_URI, QUEUE_NAME, connectionFactorySupplier);
  }

  @Test
  void reusesOneConnectionAndChannelAcrossPublishes() throws Exception {
    AMQP.BasicProperties properties = new AMQP.BasicProperties.Builder().build();
    byte[] body = "event".getBytes(StandardCharsets.UTF_8);

    publisher.publish(properties, body);
    publisher.publish(properties, body);

    verify(connectionFactorySupplier, times(1)).get();
    verify(connectionFactory, times(1)).newConnection();
    verify(connection, times(1)).createChannel();
    verify(channel, times(2)).basicPublish("", QUEUE_NAME, properties, body);
    verify(connectionFactory).setAutomaticRecoveryEnabled(false);
    verify(connectionFactory).setTopologyRecoveryEnabled(false);
    verify(connectionFactory).setConnectionTimeout(5_000);
    verify(connectionFactory).setHandshakeTimeout(5_000);
    verify(connectionFactory).setChannelRpcTimeout(5_000);
  }

  @Test
  void replacesAClosedConnectionBeforePublishingAgain() throws Exception {
    ConnectionFactory replacementFactory = mock(ConnectionFactory.class);
    Connection replacementConnection = mock(Connection.class);
    Channel replacementChannel = mock(Channel.class);
    AMQP.BasicProperties properties = new AMQP.BasicProperties.Builder().build();
    byte[] body = "event".getBytes(StandardCharsets.UTF_8);

    when(connectionFactorySupplier.get()).thenReturn(connectionFactory, replacementFactory);
    when(replacementFactory.newConnection()).thenReturn(replacementConnection);
    when(replacementConnection.createChannel()).thenReturn(replacementChannel);
    when(replacementConnection.isOpen()).thenReturn(true);
    when(replacementChannel.isOpen()).thenReturn(true);

    publisher.publish(properties, body);
    when(connection.isOpen()).thenReturn(false);
    publisher.publish(properties, body);

    verify(connectionFactory, times(1)).newConnection();
    verify(replacementFactory, times(1)).newConnection();
    verify(channel, times(1)).basicPublish("", QUEUE_NAME, properties, body);
    verify(replacementChannel, times(1)).basicPublish("", QUEUE_NAME, properties, body);
    verify(replacementFactory).setAutomaticRecoveryEnabled(false);
  }

  /**
   * Makes the first write wait, as one does once a broker that stopped reading fills the socket.
   */
  private CountDownLatch aWriteTheBrokerDoesNotRead(byte[] body) throws Exception {
    CountDownLatch brokerReads = new CountDownLatch(1);
    org.mockito.Mockito.doAnswer(
            invocation -> {
              brokerReads.await();
              return null;
            })
        .when(channel)
        .basicPublish(
            org.mockito.ArgumentMatchers.eq(""),
            org.mockito.ArgumentMatchers.eq(QUEUE_NAME),
            org.mockito.ArgumentMatchers.any(AMQP.BasicProperties.class),
            org.mockito.ArgumentMatchers.eq(body));
    return brokerReads;
  }

  @Test
  void aWriteTheBrokerDoesNotReadHoldsNoCallerAndTheOrderIsKept() throws Exception {
    AMQP.BasicProperties properties = new AMQP.BasicProperties.Builder().build();
    byte[] first = "first".getBytes(StandardCharsets.UTF_8);
    byte[] second = "second".getBytes(StandardCharsets.UTF_8);
    CountDownLatch brokerReads = aWriteTheBrokerDoesNotRead(first);

    org.junit.jupiter.api.Assertions.assertTimeoutPreemptively(
        Duration.ofSeconds(2),
        () -> {
          publisher.publishInBackground(properties, first, failure -> {});
          publisher.publishInBackground(properties, second, failure -> {});
        });
    brokerReads.countDown();

    org.mockito.InOrder order = org.mockito.Mockito.inOrder(channel);
    order
        .verify(channel, org.mockito.Mockito.timeout(5_000))
        .basicPublish("", QUEUE_NAME, properties, first);
    order
        .verify(channel, org.mockito.Mockito.timeout(5_000))
        .basicPublish("", QUEUE_NAME, properties, second);
  }

  @Test
  void aFullQueueFailsTheMessageAtOnce() throws Exception {
    AMQP.BasicProperties properties = new AMQP.BasicProperties.Builder().build();
    byte[] body = "event".getBytes(StandardCharsets.UTF_8);
    CountDownLatch brokerReads = aWriteTheBrokerDoesNotRead(body);
    CompletableFuture<Exception> failure = new CompletableFuture<>();
    try {
      // the one being written and the ones behind it
      for (int i = 0; i < RabbitMqEventPublisher.BACKGROUND_QUEUE_CAPACITY + 1; i++) {
        publisher.publishInBackground(properties, body, failure::complete);
      }
      org.junit.jupiter.api.Assertions.assertFalse(failure.isDone());

      publisher.publishInBackground(properties, body, failure::complete);

      org.junit.jupiter.api.Assertions.assertInstanceOf(
          RejectedExecutionException.class, failure.getNow(null));
    } finally {
      brokerReads.countDown();
    }
  }

  @Test
  void aFailedWriteReachesTheCallerAndDropsTheConnection() throws Exception {
    AMQP.BasicProperties properties = new AMQP.BasicProperties.Builder().build();
    byte[] body = "event".getBytes(StandardCharsets.UTF_8);
    IOException broken = new IOException("broken pipe");
    org.mockito.Mockito.doThrow(broken)
        .when(channel)
        .basicPublish("", QUEUE_NAME, properties, body);
    CompletableFuture<Exception> failure = new CompletableFuture<>();

    publisher.publishInBackground(properties, body, failure::complete);

    org.junit.jupiter.api.Assertions.assertSame(broken, failure.get(5, TimeUnit.SECONDS));
    verify(connection, org.mockito.Mockito.timeout(5_000)).abort(5_000);
  }

  @Test
  void closingWaitsForTheQueueThenClosesTheConnection() throws Exception {
    AMQP.BasicProperties properties = new AMQP.BasicProperties.Builder().build();
    byte[] body = "event".getBytes(StandardCharsets.UTF_8);

    publisher.publishInBackground(properties, body, failure -> {});
    publisher.close();

    verify(channel).basicPublish("", QUEUE_NAME, properties, body);
    verify(connection).abort(5_000);
  }

  /** That write holds the publisher's lock, so closing can't take it and waits no longer. */
  @Test
  void closingDoesNotWaitForAWriteTheBrokerDoesNotRead() throws Exception {
    byte[] body = "event".getBytes(StandardCharsets.UTF_8);
    CountDownLatch brokerReads = aWriteTheBrokerDoesNotRead(body);
    try {
      publisher.publishInBackground(
          new AMQP.BasicProperties.Builder().build(), body, failure -> {});

      org.junit.jupiter.api.Assertions.assertTimeoutPreemptively(
          Duration.ofSeconds(8), () -> publisher.close());
    } finally {
      brokerReads.countDown();
    }
  }

  @Test
  void closesTheSharedConnectionAtShutdown() throws Exception {
    publisher.publish(
        new AMQP.BasicProperties.Builder().build(), "event".getBytes(StandardCharsets.UTF_8));

    publisher.close();

    verify(connection).abort(5_000);
  }

  @Test
  void waitsForTheBrokerToConfirmAnElectoralLogEntry() throws Exception {
    publisher.publishElectoralLogConfirmed("event", "TYPE", "{}", "user", "tenant", "name", 5_000);

    org.mockito.InOrder order = org.mockito.Mockito.inOrder(channel);
    order.verify(channel).confirmSelect();
    order
        .verify(channel)
        .basicPublish(
            org.mockito.ArgumentMatchers.eq(""),
            org.mockito.ArgumentMatchers.eq(QUEUE_NAME),
            org.mockito.ArgumentMatchers.eq(true),
            org.mockito.ArgumentMatchers.any(AMQP.BasicProperties.class),
            org.mockito.ArgumentMatchers.any(byte[].class));
    order.verify(channel).waitForConfirmsOrDie(5_000);
  }

  /** With no queue to take a message, the broker returns it and still confirms it. */
  @Test
  void anElectoralLogEntryTheBrokerReturnsFailsAndDropsTheChannel() throws Exception {
    org.mockito.ArgumentCaptor<ReturnListener> listener =
        org.mockito.ArgumentCaptor.forClass(ReturnListener.class);
    org.mockito.Mockito.doAnswer(
            invocation -> {
              verify(channel).addReturnListener(listener.capture());
              listener.getValue().handleReturn(312, "NO_ROUTE", "", QUEUE_NAME, null, new byte[0]);
              return null;
            })
        .when(channel)
        .basicPublish(
            org.mockito.ArgumentMatchers.eq(""),
            org.mockito.ArgumentMatchers.eq(QUEUE_NAME),
            org.mockito.ArgumentMatchers.eq(true),
            org.mockito.ArgumentMatchers.any(AMQP.BasicProperties.class),
            org.mockito.ArgumentMatchers.any(byte[].class));

    org.junit.jupiter.api.Assertions.assertThrows(
        java.io.IOException.class,
        () ->
            publisher.publishElectoralLogConfirmed(
                "event", "TYPE", "{}", "user", "tenant", "name", 5_000));

    verify(channel).removeReturnListener(listener.getValue());
    verify(connection).abort(5_000);
  }

  @Test
  void anUnconfirmedElectoralLogEntryFailsAndDropsTheChannel() throws Exception {
    org.mockito.Mockito.doThrow(new java.io.IOException("nack"))
        .when(channel)
        .waitForConfirmsOrDie(5_000);

    org.junit.jupiter.api.Assertions.assertThrows(
        java.io.IOException.class,
        () ->
            publisher.publishElectoralLogConfirmed(
                "event", "TYPE", "{}", "user", "tenant", "name", 5_000));

    verify(connection).abort(5_000);
  }

  @Test
  void anElectoralLogEntryIsPersistentAndNamesItsTask() {
    AMQP.BasicProperties properties = RabbitMqEventPublisher.electoralLogProperties("id");
    org.junit.jupiter.api.Assertions.assertEquals(2, properties.getDeliveryMode());
    org.junit.jupiter.api.Assertions.assertEquals("id", properties.getCorrelationId());
    org.junit.jupiter.api.Assertions.assertEquals(
        "enqueue_electoral_log_event", String.valueOf(properties.getHeaders().get("task")));
  }
}
