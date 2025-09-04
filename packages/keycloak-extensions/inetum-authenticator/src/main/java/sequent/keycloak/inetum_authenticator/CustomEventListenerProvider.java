// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.inetum_authenticator;

import static sequent.keycloak.authenticator.Utils.sendErrorNotificationToUser;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.rabbitmq.client.AMQP;
import com.rabbitmq.client.Channel;
import com.rabbitmq.client.Connection;
import com.rabbitmq.client.ConnectionFactory;
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
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.events.Event;
import org.keycloak.events.EventListenerProvider;
import org.keycloak.events.EventType;
import org.keycloak.events.admin.AdminEvent;
import org.keycloak.models.KeycloakSession;

@JBossLog
public class CustomEventListenerProvider implements EventListenerProvider {

  private final KeycloakSession session;

  // Environment variables (read once for performance)
  private static final String AMQP_URI = System.getenv("AMQP_ADDR");
  private static final String TASK_NAME =
      Optional.ofNullable(System.getenv("ELECTORAL_LOG_TASK"))
          .orElse("enqueue_electoral_log_event")
          .trim();
  private static final String QUEUE_NAME;

  static {
    final String envSlug = System.getenv("ENV_SLUG");
    final String baseQueueName =
        Optional.ofNullable(System.getenv("ELECTORAL_LOG_QUEUE"))
            .orElse("electoral_log_event_queue")
            .trim();
    if (envSlug != null && !envSlug.trim().isEmpty()) {
      QUEUE_NAME = envSlug.trim() + "_" + baseQueueName;
    } else {
      QUEUE_NAME = baseQueueName;
    }
  }

  // RabbitMQ connection fields
  private Connection rabbitConnection;
  private Channel rabbitChannel;
  private ConnectionFactory rabbitFactory;

  private ObjectMapper om = new ObjectMapper();

  public CustomEventListenerProvider(KeycloakSession session) {
    this.session = session;
    initializeRabbitMQConnection();
  }

  /**
   * Parses a raw AMQP URI string and returns a new URI string with the user info (user and
   * password) percent-encoded.
   *
   * @param rawAmqpUri The raw AMQP URI from environment variables.
   * @return A URI string safe to be used with ConnectionFactory.setUri().
   */
  private String createEncodedAmqpUri(String rawAmqpUri) {
    if (rawAmqpUri == null || !rawAmqpUri.startsWith("amqp://")) {
      return rawAmqpUri; // Return as-is or throw an exception for invalid format
    }

    try {
      // Extract the part after "amqp://"
      String afterScheme = rawAmqpUri.substring("amqp://".length());
      int atIndex = afterScheme.indexOf('@');
      if (atIndex == -1) {
        log.info("encoding Amqp Uri: No user info present");
        return rawAmqpUri; // No user info present
      }

      // Split into user info and the rest (host:port/path)
      String userInfo = afterScheme.substring(0, atIndex);
      String afterUserInfo = afterScheme.substring(atIndex + 1);

      // Split user info into user and password
      String[] userPass = userInfo.split(":", 2);
      String user = userPass[0];
      String password = userPass.length > 1 ? userPass[1] : "";

      // Percent-encode user and password
      String encodedUser = URLEncoder.encode(user, StandardCharsets.UTF_8.name());
      String encodedPassword = URLEncoder.encode(password, StandardCharsets.UTF_8.name());
      String encodedUserInfo = encodedUser + (password.isEmpty() ? "" : ":" + encodedPassword);

      // Reconstruct the URI
      return "amqp://" + encodedUserInfo + "@" + afterUserInfo;
    } catch (UnsupportedEncodingException e) {
      throw new RuntimeException("UTF-8 encoding not supported", e);
    }
  }

  /** Initializes (or reinitializes) the RabbitMQ connection and channel using AMQP_ADDR. */
  private synchronized void initializeRabbitMQConnection() {
    try {
      log.debug("initializeRabbitMQConnection");
      rabbitFactory = new ConnectionFactory();
      String amqpUri = createEncodedAmqpUri(AMQP_URI);
      log.debug("Encoded Amqp Uri: " + amqpUri);
      rabbitFactory.setUri(amqpUri);
      rabbitConnection = rabbitFactory.newConnection();
      rabbitChannel = rabbitConnection.createChannel();
      rabbitChannel.queueDeclare(QUEUE_NAME, true, false, false, null);
      log.info("RabbitMQ connection and channel initialized.");
    } catch (Exception e) {
      log.error("Error initializing RabbitMQ connection", e);
    }
  }

  /** Returns an open RabbitMQ channel, reconnecting if necessary. */
  private synchronized Channel getRabbitChannel() throws Exception {
    if (rabbitConnection == null || !rabbitConnection.isOpen()) {
      log.warn("RabbitMQ connection is closed or null. Reinitializing connection.");
      initializeRabbitMQConnection();
    }
    if (rabbitChannel == null || !rabbitChannel.isOpen()) {
      rabbitChannel = rabbitConnection.createChannel();
      rabbitChannel.queueDeclare(QUEUE_NAME, true, false, false, null);
    }
    return rabbitChannel;
  }

  /**
   * Parses the realm name (from the realm ID) to extract tenant_id and election_event_id. Expected
   * format: "tenant-<tenant_uuid>-event-<election_event_uuid>"
   *
   * @param realmId the realm identifier
   * @return a String array where index 0 is tenant_id and index 1 is election_event_id.
   */
  private String[] parseRealm(String realmId) {
    String realmName = session.realms().getRealm(realmId).getName();
    if (realmName != null && realmName.startsWith("tenant-") && realmName.contains("-event-")) {
      int eventIndex = realmName.indexOf("-event-");
      String tenantId = realmName.substring("tenant-".length(), eventIndex);
      String electionEventId = realmName.substring(eventIndex + "-event-".length());
      return new String[] {tenantId, electionEventId};
    }
    return new String[] {"", ""};
  }

  @Override
  public void onEvent(Event event) {
    log.info("onEvent: start");
    // For REGISTER_ERROR events with "userNotFound", send error notifications.
    if (event.getType() == EventType.REGISTER_ERROR && "userNotFound".equals(event.getError())) {
      try {
        sendErrorNotificationToUser(session, event.getRealmId(), event);
      } catch (Exception e) {
        log.error("Failed to send error notification", e);
      }
    }

    // Extract tenant_id and election_event_id from the realm.
    String[] tenantAndEvent = parseRealm(event.getRealmId());
    String tenantId = tenantAndEvent[0];
    String electionEventId = tenantAndEvent[1];

    // Extract username from event details (default to "unknown" if absent)
    String username =
        Optional.ofNullable(event.getDetails())
            .map(details -> details.get("username"))
            .orElseGet(
                () -> {
                  if (event.getUserId() != null) {
                    var user =
                        session
                            .users()
                            .getUserById(
                                session.realms().getRealm(event.getRealmId()), event.getUserId());
                    if (user != null) {
                      return user.getEmail();
                    }
                  }
                  return "unknown";
                });

    // Prepare message body based on event type.
    String body;
    if (Utils.EVENT_TYPE_COMMUNICATIONS.equals(
        event.getDetails() != null ? event.getDetails().get("type") : null)) {
      String msgBody = Optional.ofNullable(event.getDetails().get("msgBody")).orElse("");
      body = String.format("%s %s", Utils.EVENT_TYPE_COMMUNICATIONS, msgBody);
    } else {
      // Use the event error (or another appropriate field) as body for non-communications events.
      body = event.getError();
    }

    // Publish the event to RabbitMQ with the complete JSON structure.
    logEvent(
        electionEventId, event.getType().toString(), body, event.getUserId(), tenantId, username);
  }

  @Override
  public void onEvent(AdminEvent event, boolean includeRepresentation) {
    log.info("An admin event was fired, realmId: " + event.getAuthDetails().getRealmId());
  }

  /**
   * Publishes the event message to the RabbitMQ queue. The JSON message includes:
   * election_event_id, message_type, body, user_id, tenant_id, and username.
   */
  private void logEvent(
      String electionEventId,
      String messageType,
      String body,
      String userId,
      String tenantId,
      String username) {
    log.info("logEvent: start");
    log.infov(
        "logEvent: details electionEventId: {0} messageType: {1} body: {2} userId: {3} tenantId: {4} username: {5}",
        electionEventId, messageType, body, userId, tenantId, username);

    // We make sure variables are not null otherwise log reporting will give an error when
    // deserializing
    electionEventId = Optional.ofNullable(electionEventId).orElse("null");
    messageType = Optional.ofNullable(messageType).orElse("null");
    body = Optional.ofNullable(body).orElse("null");
    userId = Optional.ofNullable(userId).orElse("null");
    tenantId = Optional.ofNullable(tenantId).orElse("null");
    username = Optional.ofNullable(username).orElse("null");

    // Build message object
    List<Object> message = new ArrayList<>();

    Map<String, Object> inputObject = new HashMap<>();

    Map<String, String> input = new HashMap<>();
    input.put("election_event_id", electionEventId);
    input.put("message_type", messageType);
    input.put("body", body);
    input.put("user_id", userId);
    input.put("tenant_id", tenantId);
    input.put("username", username);

    inputObject.put("input", input);

    Map<String, String> annotations = new HashMap<>();
    annotations.put("callbacks", null);
    annotations.put("errbacks", null);
    annotations.put("chain", null);
    annotations.put("chord", null);

    message.add(Collections.emptyList());
    message.add(inputObject);
    message.add(annotations);

    try {
      // Generate a correlation ID.
      String correlationId = UUID.randomUUID().toString();

      // Build headers map.
      Map<String, Object> headers = new HashMap<>();
      headers.put("id", correlationId);
      headers.put("task", TASK_NAME);
      headers.put("timelimit", "undefined");

      // Build properties.
      AMQP.BasicProperties props =
          new AMQP.BasicProperties.Builder()
              .correlationId(correlationId)
              .priority(0)
              .deliveryMode(2)
              .contentEncoding("utf-8")
              .contentType("application/json")
              .headers(headers)
              .build();

      Channel channel = getRabbitChannel();
      channel.basicPublish("", QUEUE_NAME, props, om.writeValueAsBytes(message));
      log.info("Message sent to RabbitMQ queue: " + QUEUE_NAME);
    } catch (Exception e) {
      log.error("Failed to send message to RabbitMQ queue: " + QUEUE_NAME, e);
    }
  }

  @Override
  public void close() {
    log.info("close()");
    try {
      if (rabbitChannel != null && rabbitChannel.isOpen()) {
        rabbitChannel.close();
      }
      if (rabbitConnection != null && rabbitConnection.isOpen()) {
        rabbitConnection.close();
      }
    } catch (Exception e) {
      log.error("Error closing RabbitMQ connection", e);
    }
  }
}
