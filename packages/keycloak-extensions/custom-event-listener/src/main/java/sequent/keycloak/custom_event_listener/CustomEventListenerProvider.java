// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import static sequent.keycloak.authenticator.Utils.AUTH_NOTE_DENY_TYPE;
import static sequent.keycloak.authenticator.Utils.CA_CERT_ISSUER_CN;
import static sequent.keycloak.authenticator.Utils.VOTER_CERT_SUBJECT_DN;
import static sequent.keycloak.authenticator.Utils.sendRegistrationErrorToSupport;

import com.fasterxml.jackson.databind.ObjectMapper;
import java.util.ArrayList;
import java.util.Collections;
import java.util.EnumSet;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
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
  private final PgmqEventPublisher pgmqEventPublisher;

  // Environment variables (read once for performance)
  private static final String TASK_NAME =
      Optional.ofNullable(System.getenv("ELECTORAL_LOG_TASK"))
          .orElse("enqueue_electoral_log_event")
          .trim();

  private final ObjectMapper om = new ObjectMapper();

  CustomEventListenerProvider(KeycloakSession session, PgmqEventPublisher pgmqEventPublisher) {
    this.session = session;
    this.pgmqEventPublisher = pgmqEventPublisher;
  }

  private static final String TENANT_REALM_PREFIX = "tenant-";
  private static final String EVENT_REALM_INFIX = "-event-";

  /**
   * Token housekeeping of a signed-in administrator, which a tenant's log leaves out: it is not
   * something the administrator did. Their failures are separate event types and are kept.
   */
  private static final Set<EventType> TENANT_REALM_SKIPPED_EVENTS =
      EnumSet.of(
          EventType.CODE_TO_TOKEN,
          EventType.REFRESH_TOKEN,
          EventType.INTROSPECT_TOKEN,
          EventType.USER_INFO_REQUEST,
          EventType.CLIENT_LOGIN,
          EventType.TOKEN_EXCHANGE,
          EventType.PERMISSION_TOKEN);

  /**
   * Parses the realm name (from the realm ID) to extract tenant_id and election_event_id; see
   * {@link #tenantAndEvent}.
   *
   * @param realmId the realm identifier
   * @return a String array where index 0 is tenant_id and index 1 is election_event_id.
   */
  private String[] parseRealm(String realmId) {
    return tenantAndEvent(session.realms().getRealm(realmId).getName());
  }

  /**
   * The tenant and the election event a realm belongs to. An election event's realm is named
   * "tenant-&lt;tenant_uuid&gt;-event-&lt;election_event_uuid&gt;". A tenant's realm,
   * "tenant-&lt;tenant_uuid&gt;", has no election event: its events go to the tenant's log. Any
   * other realm has neither.
   *
   * @return a String array where index 0 is tenant_id and index 1 is election_event_id.
   */
  static String[] tenantAndEvent(String realmName) {
    if (realmName == null || !realmName.startsWith(TENANT_REALM_PREFIX)) {
      return new String[] {"", ""};
    }
    int eventIndex = realmName.indexOf(EVENT_REALM_INFIX);
    if (eventIndex < 0) {
      return new String[] {realmName.substring(TENANT_REALM_PREFIX.length()), ""};
    }
    String tenantId = realmName.substring(TENANT_REALM_PREFIX.length(), eventIndex);
    String electionEventId = realmName.substring(eventIndex + EVENT_REALM_INFIX.length());
    return new String[] {tenantId, electionEventId};
  }

  /**
   * Whether an event is sent to the logs. An election event's realm sends every event. A tenant's
   * realm leaves out token housekeeping.
   */
  static boolean isLogged(String tenantId, String electionEventId, EventType type) {
    boolean tenantRealm = !tenantId.isEmpty() && electionEventId.isEmpty();
    return !(tenantRealm && TENANT_REALM_SKIPPED_EVENTS.contains(type));
  }

  @Override
  public void onEvent(Event event) {
    log.info("onEvent: start");
    // For REGISTER_ERROR events with "userNotFound", tell support (never the voter: R7-18).
    if (event.getType() == EventType.REGISTER_ERROR && "userNotFound".equals(event.getError())) {
      try {
        sendRegistrationErrorToSupport(session, event.getRealmId(), event);
      } catch (Exception e) {
        log.error("Failed to send error notification", e);
      }
    }

    // Extract tenant_id and election_event_id from the realm.
    String[] tenantAndEvent = parseRealm(event.getRealmId());
    String tenantId = tenantAndEvent[0];
    String electionEventId = tenantAndEvent[1];
    if (!isLogged(tenantId, electionEventId, event.getType())) {
      return;
    }

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

    // Publish the event to PGMQ with the complete JSON structure.
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

  /** Publishes the event message to the PGMQ queue; see {@link #taskMessage}. */
  private void logEvent(
      String electionEventId,
      String messageType,
      String body,
      String userId,
      String tenantId,
      String username,
      long eventTimeMs) {
    log.info("logEvent: start");
    List<Object> message =
        taskMessage(electionEventId, messageType, body, userId, tenantId, username, eventTimeMs);

    // Generate a correlation ID.
    String correlationId = UUID.randomUUID().toString();

    try {
      pgmqEventPublisher.publish(session, correlationId, TASK_NAME, om.writeValueAsBytes(message));
      log.infov("Audit event handed to PGMQ: correlationId={0}", correlationId);
    } catch (Exception e) {
      session.getTransactionManager().setRollbackOnly();
      throw new IllegalStateException("Unable to enqueue electoral audit event", e);
    }
  }

  /**
   * The Celery arguments of the {@code enqueue_electoral_log_event} task. Its input holds
   * election_event_id, message_type, body, user_id, tenant_id and username as strings, a missing
   * one as the string "null" so that the log reader can deserialize it, and event_time_ms, when
   * Keycloak saw the event in milliseconds since the Unix epoch, as a number.
   */
  static List<Object> taskMessage(
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

  @Override
  public void close() {}
}
