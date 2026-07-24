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
import java.util.Optional;
import java.util.function.Supplier;
import lombok.extern.jbosslog.JBossLog;

@JBossLog
class RabbitMqEventPublisher implements AutoCloseable {

  private static final String DEFAULT_QUEUE_NAME = "electoral_log_event_queue";

  private final String amqpUri;
  private final String queueName;
  private final Supplier<ConnectionFactory> connectionFactorySupplier;

  private Connection rabbitConnection;
  private Channel rabbitChannel;

  static RabbitMqEventPublisher fromEnvironment() {
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

  synchronized void publish(AMQP.BasicProperties properties, byte[] body) throws Exception {
    try {
      getRabbitChannel().basicPublish("", queueName, properties, body);
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
