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
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.UUID;
import java.nio.charset.StandardCharsets;
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
    // For REGISTER_ERROR events with "userNotFound", tell support (never the voter: R7-18).
    if (event.getType() == EventType.REGISTER_ERROR && "userNotFound".equals(event.getError())) {
      try {
        sendRegistrationErrorToSupport(session, event.getRealmId(), event);
      } catch (Exception e) {
        log.error("Failed to send error notification", e);
      }
    }

    // The transactional queue records failed authentication independently of
    // whether an election-specific audit board exists (staff realm included).
    if (event.getType() == EventType.LOGIN_ERROR) {
      var realm = session.realms().getRealm(event.getRealmId());
      String account = event.getUserId();
      if (account == null && event.getDetails() != null) account = event.getDetails().get("username");
      var activity = attackMessage(realm.getName(), event.getId(), account, event.getTime(),
          Math.max(1, realm.getFailureFactor()), Math.max(1, realm.getMaxDeltaTimeSeconds()));
      if (activity != null) {
        try {
          pgmqEventPublisher.publish(session, UUID.randomUUID().toString(),
              "record_attack_auth_failure", om.writeValueAsBytes(activity));
        } catch (Exception e) {
          session.getTransactionManager().setRollbackOnly();
          throw new IllegalStateException("Unable to enqueue authentication security activity", e);
        }
      }
    }

    // Extract tenant_id and election_event_id from the realm.
    String[] tenantAndEvent = parseRealm(event.getRealmId());
    String tenantId = tenantAndEvent[0];
    String electionEventId = tenantAndEvent[1];

    // Staff authentication is recorded above; an event-log task needs an event.
    if (tenantId.isEmpty() || electionEventId.isEmpty()) return;

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

  /** A separate durable activity record; no password, token, address or request URL. */
  static List<Object> attackMessage(String realm, String eventId, String account,
      long time, int failureLimit, int windowSeconds) {
    if (realm == null || !realm.startsWith("tenant-")) return null;
    String[] ids = realm.substring(7).split("-event-", -1);
    if (ids.length < 1 || ids.length > 2) return null;
    try {
      if (!UUID.fromString(ids[0]).toString().equals(ids[0])) return null;
      if (ids.length == 2 && !UUID.fromString(ids[1]).toString().equals(ids[1])) return null;
    } catch (IllegalArgumentException e) { return null; }
    if (failureLimit < 1 || failureLimit > 100000 || windowSeconds < 1 || windowSeconds > 2592000)
      throw new IllegalArgumentException("Authentication failure policy outside supported bounds");
    String identity = eventId == null ? UUID.randomUUID().toString() : eventId;
    Map<String,Object> input = new HashMap<>();
    input.put("id", UUID.nameUUIDFromBytes((realm + ":" + identity).getBytes(StandardCharsets.UTF_8)).toString());
    input.put("tenant_id", ids[0]);
    input.put("election_event_id", ids.length == 2 ? ids[1] : null);
    input.put("realm", realm);
    input.put("account", account == null || account.isBlank() || account.length() > 512 ? null : account);
    input.put("event_time_ms", time);
    input.put("failure_limit", failureLimit);
    input.put("window_seconds", windowSeconds);
    return List.of(Collections.emptyList(), Map.of("input",input), Collections.emptyMap());
  }

  @Override
  public void close() {}
}
