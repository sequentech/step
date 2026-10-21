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
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.UUID;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.events.Event;
import org.keycloak.events.EventListenerProvider;
import org.keycloak.events.EventType;
import org.keycloak.events.admin.AdminEvent;
import org.keycloak.events.admin.AuthDetails;
import org.keycloak.models.ClientModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;

@JBossLog
public class CustomEventListenerProvider implements EventListenerProvider {

  static final String ADMIN_EVENT_MESSAGE_TYPE = "KEYCLOAK_ADMIN_EVENT";

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

  private static LogRealm logRealm(RealmModel realm) {
    return LogRealm.fromName(realm == null ? null : realm.getName());
  }

  /** The account's username, when the realm has an account with that id. */
  private Optional<String> usernameOf(RealmModel realm, String userId) {
    if (realm == null || userId == null) {
      return Optional.empty();
    }
    UserModel user = session.users().getUserById(realm, userId);
    return Optional.ofNullable(user).map(UserModel::getUsername);
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

    RealmModel realmModel = session.realms().getRealm(event.getRealmId());
    LogRealm realm = logRealm(realmModel);
    if (realm.kind() == LogRealm.Kind.OTHER) {
      return;
    }
    if (realm.kind() == LogRealm.Kind.TENANT && !TenantRealmEvents.keeps(event)) {
      return;
    }

    Map<String, String> details =
        event.getDetails() != null ? event.getDetails() : Collections.emptyMap();
    String userId = event.getUserId();
    Optional<String> accountUsername = usernameOf(realmModel, userId);

    // The tenant log never records what was typed for an account that does not exist; an election
    // event's log keeps the username of the details, or "unknown".
    UserEventDetails.Account account =
        realm.kind() == LogRealm.Kind.TENANT && accountUsername.isEmpty()
            ? UserEventDetails.Account.UNKNOWN
            : UserEventDetails.Account.KNOWN;
    String username =
        switch (account) {
          case UNKNOWN -> null;
          case KNOWN ->
              accountUsername.orElseGet(
                  () -> Optional.ofNullable(details.get("username")).orElse("unknown"));
        };

    // Prepare message body based on event type.
    boolean isLoginWithCertificate =
        details.containsKey(VOTER_CERT_SUBJECT_DN) && details.containsKey(CA_CERT_ISSUER_CN);
    String body;
    if (Utils.isCommunication(details)) {
      // The tenant log records that a message was sent to the account, never its text, so the
      // text does not leave Keycloak. An election event's log keeps the (masked) text.
      String msgBody =
          realm.kind() == LogRealm.Kind.TENANT
              ? ""
              : Optional.ofNullable(details.get(Utils.EVENT_DETAIL_MSG_BODY)).orElse("");
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
        taskMessage(
            realm.electionEventId(),
            event.getType().toString(),
            body,
            userId,
            realm.tenantId(),
            username,
            event.getTime(),
            Map.of("details", UserEventDetails.of(event, account))));
  }

  @Override
  public void onEvent(AdminEvent event, boolean includeRepresentation) {
    LogRealm realm = logRealm(session.realms().getRealm(event.getRealmId()));
    if (realm.kind() == LogRealm.Kind.OTHER) {
      return;
    }

    AuthDetails auth = Optional.ofNullable(event.getAuthDetails()).orElseGet(AuthDetails::new);
    RealmModel authRealm =
        auth.getRealmId() == null ? null : session.realms().getRealm(auth.getRealmId());
    String username = usernameOf(authRealm, auth.getUserId()).orElse(null);

    Map<String, Object> actor = new LinkedHashMap<>();
    actor.put("user_id", auth.getUserId());
    actor.put("username", username);
    actor.put("client_id", clientIdOf(authRealm, auth.getClientId()));
    actor.put("realm", authRealm == null ? auth.getRealmId() : authRealm.getName());
    actor.put("ip_address", auth.getIpAddress());

    Map<String, Object> adminEvent = new LinkedHashMap<>();
    adminEvent.put(
        "operation", event.getOperationType() == null ? null : event.getOperationType().toString());
    adminEvent.put("resource_type", event.getResourceTypeAsString());
    adminEvent.put("resource_path", event.getResourcePath());
    adminEvent.put(
        "representation",
        includeRepresentation ? AdminRepresentation.forLog(event.getRepresentation()) : null);
    adminEvent.put("error", event.getError());
    adminEvent.put("actor", actor);

    logEvent(
        taskMessage(
            realm.electionEventId(),
            ADMIN_EVENT_MESSAGE_TYPE,
            null,
            auth.getUserId(),
            realm.tenantId(),
            username,
            event.getTime(),
            Map.of("admin_event", adminEvent)));
  }

  /** Keycloak gives the client's internal id; the log names the client as administrators do. */
  private static String clientIdOf(RealmModel realm, String clientInternalId) {
    if (realm == null || clientInternalId == null) {
      return clientInternalId;
    }
    ClientModel client = realm.getClientById(clientInternalId);
    return client == null ? clientInternalId : client.getClientId();
  }

  /** Publishes the event message to the PGMQ queue; see {@link #taskMessage}. */
  private void logEvent(List<Object> message) {
    log.info("logEvent: start");

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
    return taskMessage(
        electionEventId, messageType, body, userId, tenantId, username, eventTimeMs, Map.of());
  }

  /**
   * As {@link #taskMessage(String, String, String, String, String, String, long)}, with the
   * optional objects of the input: {@code details} for a user event, {@code admin_event} for an
   * admin event.
   */
  static List<Object> taskMessage(
      String electionEventId,
      String messageType,
      String body,
      String userId,
      String tenantId,
      String username,
      long eventTimeMs,
      Map<String, Object> optional) {
    Map<String, Object> input = new HashMap<>();
    input.put("election_event_id", Optional.ofNullable(electionEventId).orElse("null"));
    input.put("message_type", Optional.ofNullable(messageType).orElse("null"));
    input.put("body", Optional.ofNullable(body).orElse("null"));
    input.put("user_id", Optional.ofNullable(userId).orElse("null"));
    input.put("tenant_id", Optional.ofNullable(tenantId).orElse("null"));
    input.put("username", Optional.ofNullable(username).orElse("null"));
    input.put("event_time_ms", eventTimeMs);
    input.putAll(optional);

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
