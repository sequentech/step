// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.inetum_authenticator;

import static sequent.keycloak.authenticator.Utils.sendErrorNotificationToUser;

import com.rabbitmq.client.AMQP;
import com.rabbitmq.client.Channel;
import com.rabbitmq.client.Connection;
import com.rabbitmq.client.ConnectionFactory;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
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
  private static final String TASK_NAME = "enqueue_electoral_log_event";
  private static final String QUEUE_NAME = "electoral_log_event_queue";

  // RabbitMQ connection fields
  private Connection rabbitConnection;
  private Channel rabbitChannel;
  private ConnectionFactory rabbitFactory;

  public CustomEventListenerProvider(KeycloakSession session) {
    this.session = session;
    initializeRabbitMQConnection();
  }

  /** Initializes (or reinitializes) the RabbitMQ connection and channel using AMQP_ADDR. */
  private synchronized void initializeRabbitMQConnection() {
    try {
      rabbitFactory = new ConnectionFactory();
      rabbitFactory.setUri(AMQP_URI);
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
            .orElse("unknown");

    // Prepare message body based on event type.
    String body;
    if (Utils.EVENT_TYPE_COMMUNICATIONS.equals(
        event.getDetails() != null ? event.getDetails().get("type") : null)) {
      String msgBody = Optional.ofNullable(event.getDetails().get("msgBody")).orElse("");
      body = String.format("%s %s", Utils.EVENT_TYPE_COMMUNICATIONS, msgBody).replace("\n", " ").replace("\\\"", "\"");
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
    String message =
        String.format(
            """
            [
                [],
                {
                    "input": {
                        "election_event_id":"%s",
                        "message_type":"%s",
                        "body":"%s",
                        "user_id":"%s",
                        "tenant_id":"%s",
                        "username":"%s"
                    }
                },
                {
                    "callbacks": null,
                    "errbacks": null,
                    "chain": null,
                    "chord": null
                }
            ]
            """,
            Utils.escapeJson(electionEventId),
            Utils.escapeJson(messageType),
            Utils.escapeJson(body),
            Utils.escapeJson(userId),
            Utils.escapeJson(tenantId),
            Utils.escapeJson(username));
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
      channel.basicPublish("", QUEUE_NAME, props, message.getBytes(StandardCharsets.UTF_8));
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
