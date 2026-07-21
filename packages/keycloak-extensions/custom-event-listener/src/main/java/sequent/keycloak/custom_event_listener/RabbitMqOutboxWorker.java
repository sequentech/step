// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import com.rabbitmq.client.AMQP;
import com.rabbitmq.client.Channel;
import com.rabbitmq.client.Connection;
import com.rabbitmq.client.ConnectionFactory;
import java.io.UnsupportedEncodingException;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.ThreadLocalRandom;
import java.util.concurrent.TimeUnit;
import java.util.function.IntToLongFunction;
import java.util.function.LongSupplier;
import java.util.function.Supplier;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.models.KeycloakSessionFactory;

@JBossLog
class RabbitMqOutboxWorker implements AutoCloseable {

  private static final int CONNECTION_TIMEOUT_MILLIS = 5_000;
  private static final int HANDSHAKE_TIMEOUT_MILLIS = 5_000;
  private static final int CHANNEL_RPC_TIMEOUT_MILLIS = 5_000;
  private static final int SHUTDOWN_TIMEOUT_MILLIS = 1_000;
  private static final long CONFIRM_TIMEOUT_MILLIS = 5_000L;
  private static final long CLAIM_LEASE_MILLIS = 60_000L;
  private static final long IDLE_POLL_MILLIS = 500L;
  private static final long MAX_RETRY_DELAY_MILLIS = 30_000L;
  private static final long SHUTDOWN_WAIT_MILLIS = 5_000L;

  enum DeliveryResult {
    IDLE,
    DELIVERED,
    FAILED
  }

  private final String amqpUri;
  private final AuditOutboxStore outboxStore;
  private final Supplier<ConnectionFactory> connectionFactorySupplier;
  private final LongSupplier clock;
  private final IntToLongFunction retryDelay;
  private final ExecutorService executor;
  private final Set<String> declaredQueues = new HashSet<>();

  private volatile boolean running;
  private Connection rabbitConnection;
  private Channel rabbitChannel;

  RabbitMqOutboxWorker(
      String amqpUri,
      AuditOutboxStore outboxStore,
      Supplier<ConnectionFactory> connectionFactorySupplier) {
    this(
        amqpUri,
        outboxStore,
        connectionFactorySupplier,
        System::currentTimeMillis,
        RabbitMqOutboxWorker::retryDelayWithJitter);
  }

  RabbitMqOutboxWorker(
      String amqpUri,
      AuditOutboxStore outboxStore,
      Supplier<ConnectionFactory> connectionFactorySupplier,
      LongSupplier clock,
      IntToLongFunction retryDelay) {
    this.amqpUri = encodeAmqpUri(amqpUri);
    this.outboxStore = outboxStore;
    this.connectionFactorySupplier = connectionFactorySupplier;
    this.clock = clock;
    this.retryDelay = retryDelay;
    executor =
        Executors.newSingleThreadExecutor(
            runnable -> {
              Thread thread = new Thread(runnable, "sequent-audit-outbox-publisher");
              thread.setDaemon(true);
              return thread;
            });
  }

  synchronized void start(KeycloakSessionFactory sessionFactory) {
    if (running) {
      return;
    }
    if (executor.isShutdown()) {
      throw new IllegalStateException("Audit outbox worker cannot be restarted after shutdown");
    }
    running = true;
    executor.submit(() -> run(sessionFactory));
    log.info("RabbitMQ audit outbox worker started.");
  }

  DeliveryResult deliverNext(KeycloakSessionFactory sessionFactory) {
    long now = clock.getAsLong();
    Optional<RabbitMqOutboxMessage> claimedMessage =
        outboxStore.claimNext(sessionFactory, now, CLAIM_LEASE_MILLIS);
    if (claimedMessage.isEmpty()) {
      return DeliveryResult.IDLE;
    }

    RabbitMqOutboxMessage message = claimedMessage.get();
    try {
      Channel channel = getRabbitChannel();
      declareQueue(channel, message.queueName());
      channel.basicPublish(
          "", message.queueName(), true, createProperties(message), message.body());
      channel.waitForConfirmsOrDie(CONFIRM_TIMEOUT_MILLIS);
      outboxStore.markDelivered(sessionFactory, message.id(), message.claimToken());
      log.infov("Confirmed RabbitMQ delivery for audit event {0}", message.id());
      return DeliveryResult.DELIVERED;
    } catch (Exception exception) {
      abortRabbitMqResources();
      long retryAt = now + retryDelay.applyAsLong(message.attempt());
      String error = describe(exception);
      try {
        outboxStore.markFailed(sessionFactory, message.id(), message.claimToken(), retryAt, error);
      } catch (RuntimeException rescheduleException) {
        log.errorv(
            rescheduleException,
            "RabbitMQ delivery failed for audit event {0}, and its retry could not be scheduled; the durable claim will expire",
            message.id());
        return DeliveryResult.FAILED;
      }
      log.errorv(
          exception,
          "RabbitMQ delivery failed for audit event {0}; retained in the outbox for retry at {1}",
          message.id(),
          retryAt);
      return DeliveryResult.FAILED;
    }
  }

  private void run(KeycloakSessionFactory sessionFactory) {
    while (running && !Thread.currentThread().isInterrupted()) {
      try {
        DeliveryResult result = deliverNext(sessionFactory);
        if (result != DeliveryResult.DELIVERED) {
          Thread.sleep(IDLE_POLL_MILLIS);
        }
      } catch (InterruptedException exception) {
        Thread.currentThread().interrupt();
      } catch (RuntimeException exception) {
        log.error(
            "Audit outbox worker cycle failed; durable events remain available for retry",
            exception);
        try {
          Thread.sleep(IDLE_POLL_MILLIS);
        } catch (InterruptedException interruptedException) {
          Thread.currentThread().interrupt();
        }
      }
    }
  }

  private synchronized Channel getRabbitChannel() throws Exception {
    if (rabbitConnection == null || !rabbitConnection.isOpen()) {
      initializeRabbitMqConnection();
    } else if (rabbitChannel == null || !rabbitChannel.isOpen()) {
      abortRabbitChannel();
      rabbitChannel = rabbitConnection.createChannel();
      rabbitChannel.confirmSelect();
      declaredQueues.clear();
    }
    return rabbitChannel;
  }

  private void initializeRabbitMqConnection() throws Exception {
    abortRabbitMqResources();

    ConnectionFactory rabbitFactory = connectionFactorySupplier.get();
    rabbitFactory.setAutomaticRecoveryEnabled(false);
    rabbitFactory.setTopologyRecoveryEnabled(false);
    rabbitFactory.setConnectionTimeout(CONNECTION_TIMEOUT_MILLIS);
    rabbitFactory.setHandshakeTimeout(HANDSHAKE_TIMEOUT_MILLIS);
    rabbitFactory.setChannelRpcTimeout(CHANNEL_RPC_TIMEOUT_MILLIS);
    rabbitFactory.setShutdownTimeout(SHUTDOWN_TIMEOUT_MILLIS);
    rabbitFactory.setUri(amqpUri);

    try {
      rabbitConnection = rabbitFactory.newConnection();
      rabbitChannel = rabbitConnection.createChannel();
      rabbitChannel.confirmSelect();
      declaredQueues.clear();
      log.info("RabbitMQ connection and confirmed channel initialized for the audit outbox.");
    } catch (Exception exception) {
      abortRabbitMqResources();
      throw exception;
    }
  }

  private void declareQueue(Channel channel, String queueName) throws Exception {
    if (declaredQueues.add(queueName)) {
      try {
        channel.queueDeclare(queueName, true, false, false, null);
      } catch (Exception exception) {
        declaredQueues.remove(queueName);
        throw exception;
      }
    }
  }

  private static AMQP.BasicProperties createProperties(RabbitMqOutboxMessage message) {
    Map<String, Object> headers = new HashMap<>();
    headers.put("id", message.id());
    headers.put("task", message.taskName());
    headers.put("timelimit", "undefined");
    return new AMQP.BasicProperties.Builder()
        .messageId(message.id())
        .correlationId(message.id())
        .priority(0)
        .deliveryMode(2)
        .contentEncoding("utf-8")
        .contentType("application/json")
        .headers(headers)
        .build();
  }

  @Override
  public void close() {
    running = false;
    executor.shutdownNow();
    try {
      if (!executor.awaitTermination(SHUTDOWN_WAIT_MILLIS, TimeUnit.MILLISECONDS)) {
        log.warn("Audit outbox worker did not stop before the shutdown deadline.");
      }
    } catch (InterruptedException exception) {
      Thread.currentThread().interrupt();
    }
    closeRabbitMqResources();
  }

  private synchronized void abortRabbitMqResources() {
    abortRabbitChannel();

    Connection connectionToAbort = rabbitConnection;
    rabbitConnection = null;
    if (connectionToAbort == null) {
      return;
    }
    try {
      connectionToAbort.abort();
    } catch (Exception exception) {
      log.warn("Error aborting RabbitMQ connection", exception);
    }
  }

  private void abortRabbitChannel() {
    Channel channelToAbort = rabbitChannel;
    rabbitChannel = null;
    declaredQueues.clear();
    if (channelToAbort == null) {
      return;
    }
    try {
      channelToAbort.abort();
    } catch (Exception exception) {
      log.warn("Error aborting RabbitMQ channel", exception);
    }
  }

  private synchronized void closeRabbitMqResources() {
    Channel channelToClose = rabbitChannel;
    rabbitChannel = null;
    declaredQueues.clear();
    if (channelToClose != null) {
      try {
        channelToClose.close();
      } catch (Exception exception) {
        log.warn("Error closing RabbitMQ channel", exception);
      }
    }

    Connection connectionToClose = rabbitConnection;
    rabbitConnection = null;
    if (connectionToClose != null) {
      try {
        connectionToClose.close();
      } catch (Exception exception) {
        log.warn("Error closing RabbitMQ connection", exception);
      }
    }
  }

  private static long retryDelayWithJitter(int attempt) {
    int shift = Math.max(0, Math.min(attempt - 1, 5));
    long exponentialDelay = Math.min(1_000L << shift, MAX_RETRY_DELAY_MILLIS);
    return ThreadLocalRandom.current().nextLong(exponentialDelay / 2, exponentialDelay + 1);
  }

  private static String describe(Exception exception) {
    String message = exception.getMessage();
    return message == null || message.isBlank() ? exception.getClass().getSimpleName() : message;
  }

  private static String encodeAmqpUri(String rawAmqpUri) {
    if (rawAmqpUri == null || !rawAmqpUri.startsWith("amqp://")) {
      return rawAmqpUri;
    }

    try {
      String afterScheme = rawAmqpUri.substring("amqp://".length());
      int atIndex = afterScheme.indexOf('@');
      if (atIndex == -1) {
        return rawAmqpUri;
      }

      String userInfo = afterScheme.substring(0, atIndex);
      String afterUserInfo = afterScheme.substring(atIndex + 1);
      String[] userPass = userInfo.split(":", 2);
      String user = userPass[0];
      String password = userPass.length > 1 ? userPass[1] : "";
      String encodedUser = URLEncoder.encode(user, StandardCharsets.UTF_8.name());
      String encodedPassword = URLEncoder.encode(password, StandardCharsets.UTF_8.name());
      String encodedUserInfo = encodedUser + (password.isEmpty() ? "" : ":" + encodedPassword);

      return "amqp://" + encodedUserInfo + "@" + afterUserInfo;
    } catch (UnsupportedEncodingException exception) {
      throw new IllegalStateException("UTF-8 encoding not supported", exception);
    }
  }
}
