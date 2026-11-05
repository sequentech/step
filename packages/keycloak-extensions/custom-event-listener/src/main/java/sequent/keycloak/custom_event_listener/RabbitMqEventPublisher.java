// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.rabbitmq.client.AMQP;
import com.rabbitmq.client.Channel;
import com.rabbitmq.client.Connection;
import com.rabbitmq.client.ConnectionFactory;
import com.rabbitmq.client.ReturnListener;
import java.io.IOException;
import java.io.UnsupportedEncodingException;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.UUID;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.function.Supplier;
import lombok.extern.jbosslog.JBossLog;

@JBossLog
public class RabbitMqEventPublisher implements AutoCloseable {

  private static final String DEFAULT_QUEUE_NAME = "electoral_log_event_queue";
  private static final ObjectMapper JSON = new ObjectMapper();

  /**
   * A broker gone without closing its sockets would otherwise hold every event for the client's
   * default of 60 seconds, since one connection attempt runs at a time.
   */
  private static final int CONNECT_TIMEOUT_MILLIS = 5_000;

  // Read once for performance
  private static final String TASK_NAME =
      Optional.ofNullable(System.getenv("ELECTORAL_LOG_TASK"))
          .orElse("enqueue_electoral_log_event")
          .trim();

  private final String amqpUri;
  private final String queueName;
  private final Supplier<ConnectionFactory> connectionFactorySupplier;

  private Connection rabbitConnection;
  private Channel rabbitChannel;

  /**
   * A publisher for the broker at {@code AMQP_ADDR}. The queue is {@code ELECTORAL_LOG_QUEUE} or
   * its default, prefixed with {@code ENV_SLUG} when that is set.
   */
  public static RabbitMqEventPublisher fromEnvironment() {
    String envSlug = System.getenv("ENV_SLUG");
    String baseQueueName =
        Optional.ofNullable(System.getenv("ELECTORAL_LOG_QUEUE")).orElse(DEFAULT_QUEUE_NAME).trim();
    String queueName =
        envSlug == null || envSlug.trim().isEmpty()
            ? baseQueueName
            : envSlug.trim() + "_" + baseQueueName;
    return new RabbitMqEventPublisher(
        System.getenv("AMQP_ADDR"), queueName, ConnectionFactory::new);
  }

  RabbitMqEventPublisher(
      String amqpUri, String queueName, Supplier<ConnectionFactory> connectionFactorySupplier) {
    this.amqpUri = encodeAmqpUri(amqpUri);
    this.queueName = queueName;
    this.connectionFactorySupplier = connectionFactorySupplier;
  }

  /** The properties of a Celery message for the electoral-log task. */
  static AMQP.BasicProperties electoralLogProperties(String correlationId) {
    Map<String, Object> headers = new HashMap<>();
    headers.put("id", correlationId);
    headers.put("task", TASK_NAME);
    headers.put("timelimit", "undefined");
    return new AMQP.BasicProperties.Builder()
        .correlationId(correlationId)
        .priority(0)
        .deliveryMode(2)
        .contentEncoding("utf-8")
        .contentType("application/json")
        .headers(headers)
        .build();
  }

  /**
   * The body of a Celery message for the electoral-log task. Nulls become the string "null", since
   * log reporting would otherwise fail to deserialize them.
   */
  static byte[] electoralLogBody(
      String electionEventId,
      String messageType,
      String body,
      String userId,
      String tenantId,
      String username)
      throws JsonProcessingException {
    Map<String, String> input = new HashMap<>();
    input.put("election_event_id", Optional.ofNullable(electionEventId).orElse("null"));
    input.put("message_type", Optional.ofNullable(messageType).orElse("null"));
    input.put("body", Optional.ofNullable(body).orElse("null"));
    input.put("user_id", Optional.ofNullable(userId).orElse("null"));
    input.put("tenant_id", Optional.ofNullable(tenantId).orElse("null"));
    input.put("username", Optional.ofNullable(username).orElse("null"));

    Map<String, Object> inputObject = new HashMap<>();
    inputObject.put("input", input);

    Map<String, String> annotations = new HashMap<>();
    annotations.put("callbacks", null);
    annotations.put("errbacks", null);
    annotations.put("chain", null);
    annotations.put("chord", null);

    List<Object> message = new ArrayList<>();
    message.add(Collections.emptyList());
    message.add(inputObject);
    message.add(annotations);
    return JSON.writeValueAsBytes(message);
  }

  /**
   * Publishes a message without waiting for the broker to confirm it. A failure drops the
   * connection, so the next call connects again.
   */
  synchronized void publish(AMQP.BasicProperties properties, byte[] body) throws Exception {
    try {
      getRabbitChannel().basicPublish("", queueName, properties, body);
    } catch (Exception exception) {
      closeRabbitMqResources();
      throw exception;
    }
  }

  /**
   * Publishes an electoral-log entry and waits until the broker has accepted it. The queue is
   * durable and the message persistent, but a classic queue confirms before it syncs to disk, so a
   * crash right after the confirm can still lose the entry. An action that must not happen
   * unaudited runs only after this returns. No AMQP type appears here because this jar's RabbitMQ
   * classes are not for other extensions to depend on.
   */
  public synchronized void publishElectoralLogConfirmed(
      String electionEventId,
      String messageType,
      String body,
      String userId,
      String tenantId,
      String username,
      long timeoutMillis)
      throws Exception {
    publishConfirmed(
        electoralLogProperties(UUID.randomUUID().toString()),
        electoralLogBody(electionEventId, messageType, body, userId, tenantId, username),
        timeoutMillis);
  }

  /**
   * Publishes a message and waits up to {@code timeoutMillis} for the broker to confirm it. The
   * broker also confirms a message it could not route, so one it returned counts as a failure. Like
   * {@link #publish}, a failure drops the connection, and the next call declares the queue again.
   */
  synchronized void publishConfirmed(
      AMQP.BasicProperties properties, byte[] body, long timeoutMillis) throws Exception {
    try {
      Channel channel = getRabbitChannel();
      channel.confirmSelect();
      AtomicBoolean returned = new AtomicBoolean();
      ReturnListener onReturn =
          (replyCode, replyText, exchange, routingKey, returnedProperties, returnedBody) ->
              returned.set(true);
      channel.addReturnListener(onReturn);
      try {
        channel.basicPublish("", queueName, true, properties, body);
        channel.waitForConfirmsOrDie(timeoutMillis);
      } finally {
        channel.removeReturnListener(onReturn);
      }
      if (returned.get()) {
        throw new IOException("The broker returned the message: no queue took it");
      }
    } catch (Exception exception) {
      closeRabbitMqResources();
      throw exception;
    }
  }

  private Channel getRabbitChannel() throws Exception {
    if (rabbitConnection == null || !rabbitConnection.isOpen()) {
      initializeRabbitMqConnection();
    } else if (rabbitChannel == null || !rabbitChannel.isOpen()) {
      closeRabbitChannel();
      rabbitChannel = rabbitConnection.createChannel();
      declareQueue();
    }
    return rabbitChannel;
  }

  private void initializeRabbitMqConnection() throws Exception {
    closeRabbitMqResources();

    ConnectionFactory rabbitFactory = connectionFactorySupplier.get();
    rabbitFactory.setAutomaticRecoveryEnabled(false);
    rabbitFactory.setTopologyRecoveryEnabled(false);
    rabbitFactory.setConnectionTimeout(CONNECT_TIMEOUT_MILLIS);
    rabbitFactory.setUri(amqpUri);

    try {
      rabbitConnection = rabbitFactory.newConnection();
      rabbitChannel = rabbitConnection.createChannel();
      declareQueue();
      log.info("RabbitMQ connection and channel initialized.");
    } catch (Exception exception) {
      closeRabbitMqResources();
      throw exception;
    }
  }

  private void declareQueue() throws Exception {
    rabbitChannel.queueDeclare(queueName, true, false, false, null);
  }

  @Override
  public synchronized void close() {
    closeRabbitMqResources();
  }

  private void closeRabbitMqResources() {
    closeRabbitChannel();

    Connection connectionToClose = rabbitConnection;
    rabbitConnection = null;
    if (connectionToClose == null || !connectionToClose.isOpen()) {
      return;
    }
    try {
      connectionToClose.close();
    } catch (Exception exception) {
      log.warn("Error closing RabbitMQ connection", exception);
    }
  }

  private void closeRabbitChannel() {
    Channel channelToClose = rabbitChannel;
    rabbitChannel = null;
    if (channelToClose == null || !channelToClose.isOpen()) {
      return;
    }
    try {
      channelToClose.close();
    } catch (Exception exception) {
      log.warn("Error closing RabbitMQ channel", exception);
    }
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
