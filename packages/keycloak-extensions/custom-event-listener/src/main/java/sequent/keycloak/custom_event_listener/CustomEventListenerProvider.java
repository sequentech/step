// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import static sequent.keycloak.authenticator.Utils.AUTH_NOTE_DENY_TYPE;
import static sequent.keycloak.authenticator.Utils.CA_CERT_ISSUER_CN;
import static sequent.keycloak.authenticator.Utils.VOTER_CERT_SUBJECT_DN;
import static sequent.keycloak.authenticator.Utils.sendErrorNotificationToUser;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.rabbitmq.client.AMQP;
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
  private final RabbitMqEventPublisher rabbitMqEventPublisher;

  // Environment variables (read once for performance)
  private static final String TASK_NAME =
      Optional.ofNullable(System.getenv("ELECTORAL_LOG_TASK"))
          .orElse("enqueue_electoral_log_event")
          .trim();

  private final ObjectMapper om = new ObjectMapper();

  CustomEventListenerProvider(
      KeycloakSession session, RabbitMqEventPublisher rabbitMqEventPublisher) {
    this.session = session;
    this.rabbitMqEventPublisher = rabbitMqEventPublisher;
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

    String userId = event.getUserId();

    if (userId != null) {
      var user = session.users().getUserById(session.realms().getRealm(event.getRealmId()), userId);
      if (user != null && user.getUsername() != null) {
        // Override with the actual username from session if available
        username = user.getUsername();
      }
    }
    // Prepare message body based on event type.
    Map<String, String> details =
        event.getDetails() != null ? event.getDetails() : Collections.emptyMap();
    boolean isLoginWithCertificate =
        details.containsKey(VOTER_CERT_SUBJECT_DN) && details.containsKey(CA_CERT_ISSUER_CN);
    String body;
    if (Utils.EVENT_TYPE_COMMUNICATIONS.equals(details.isEmpty() ? null : details.get("type"))) {
      String msgBody = Optional.ofNullable(details.get("msgBody")).orElse("");
      body = String.format("%s %s", Utils.EVENT_TYPE_COMMUNICATIONS, msgBody);
    } else if (event.getType() == EventType.LOGIN && isLoginWithCertificate) {
      String certInfo =
          VOTER_CERT_SUBJECT_DN
              + "="
              + details.get(VOTER_CERT_SUBJECT_DN)
              + " "
              + CA_CERT_ISSUER_CN
              + "="
              + details.get(CA_CERT_ISSUER_CN);
      body = certInfo;
    } else if (event.getType() == EventType.LOGIN_ERROR && isLoginWithCertificate) {
      String denyType = details.getOrDefault(AUTH_NOTE_DENY_TYPE, "none");
      if (userId == null) {
        log.warn(
            "Login error event with certificate details but no userId. Cannot retrieve username.");
      }
      String certInfo =
          AUTH_NOTE_DENY_TYPE
              + "="
              + denyType
              + " "
              + VOTER_CERT_SUBJECT_DN
              + "="
              + details.getOrDefault(VOTER_CERT_SUBJECT_DN, "unknown")
              + " "
              + CA_CERT_ISSUER_CN
              + "="
              + details.getOrDefault(CA_CERT_ISSUER_CN, "unknown");
      body = event.getError() + " " + certInfo;
    } else {
      // Use the event error (or another appropriate field) as body for
      // non-communications events.
      body = event.getError();
    }

    // Publish the event to RabbitMQ with the complete JSON structure.
    logEvent(
        electionEventId,
        event.getType().toString(),
        body,
        event.getUserId(),
        tenantId,
        username,
        event.getTime());
  }

  @Override
  public void onEvent(AdminEvent event, boolean includeRepresentation) {
    log.info("An admin event was fired, realmId: " + event.getAuthDetails().getRealmId());
  }

  /**
   * Builds the Celery message for an event. Its input holds election_event_id, message_type, body,
   * user_id, tenant_id and username as strings, a missing one as the string "null" so that the log
   * reader can deserialize it, and event_time_ms, when Keycloak saw the event in milliseconds since
   * the Unix epoch, as a number.
   */
  static List<Object> buildMessage(
      String electionEventId,
      String messageType,
      String body,
      String userId,
      String tenantId,
      String username,
      long eventTimeMs) {
    Map<String, Object> input = new HashMap<>();
    input.put("election_event_id", Optional.ofNullable(electionEventId).orElse("null"));
    input.put("message_type", Optional.ofNullable(messageType).orElse("null"));
    input.put("body", Optional.ofNullable(body).orElse("null"));
    input.put("user_id", Optional.ofNullable(userId).orElse("null"));
    input.put("tenant_id", Optional.ofNullable(tenantId).orElse("null"));
    input.put("username", Optional.ofNullable(username).orElse("null"));
    input.put("event_time_ms", eventTimeMs);

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
    return message;
  }

  /** Publishes the event message to the RabbitMQ queue; see {@link #buildMessage}. */
  private void logEvent(
      String electionEventId,
      String messageType,
      String body,
      String userId,
      String tenantId,
      String username,
      long eventTimeMs) {
    log.info("logEvent: start");
    log.infov(
        "logEvent: details electionEventId: {0} messageType: {1} body: {2} userId: {3} tenantId: {4} username: {5}",
        electionEventId, messageType, body, userId, tenantId, username);

    List<Object> message =
        buildMessage(
            electionEventId, messageType, body, userId, tenantId, username, eventTimeMs);

    // Generate a correlation ID.
    String correlationId = UUID.randomUUID().toString();

    try {
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

      rabbitMqEventPublisher.publish(props, om.writeValueAsBytes(message));
      log.infov("Audit event published to RabbitMQ: correlationId={0}", correlationId);
    } catch (Exception e) {
      log.errorv(
          e,
          "Audit event was not delivered to RabbitMQ: correlationId={0}, tenantId={1}, electionEventId={2}, messageType={3}, userId={4}, username={5}, body={6}",
          correlationId,
          tenantId,
          electionEventId,
          messageType,
          userId,
          username,
          body);
    }
  }

  @Override
  public void close() {}
}
