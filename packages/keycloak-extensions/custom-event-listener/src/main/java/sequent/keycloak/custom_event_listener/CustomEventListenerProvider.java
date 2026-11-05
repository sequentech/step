// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import static sequent.keycloak.authenticator.Utils.AUTH_NOTE_DENY_TYPE;
import static sequent.keycloak.authenticator.Utils.CA_CERT_ISSUER_CN;
import static sequent.keycloak.authenticator.Utils.VOTER_CERT_SUBJECT_DN;
import static sequent.keycloak.authenticator.Utils.sendErrorNotificationToUser;

import java.util.Collections;
import java.util.Map;
import java.util.Optional;
import java.util.UUID;
import java.util.function.Consumer;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.events.Details;
import org.keycloak.events.Event;
import org.keycloak.events.EventListenerProvider;
import org.keycloak.events.EventType;
import org.keycloak.events.admin.AdminEvent;
import org.keycloak.models.KeycloakSession;

@JBossLog
public class CustomEventListenerProvider implements EventListenerProvider {

  private final KeycloakSession session;
  private final RabbitMqEventPublisher rabbitMqEventPublisher;

  private static final String PIN_LINK_STAGE_DETAIL = "pin_link_stage";

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

  /**
   * Turns a Keycloak event into an electoral-log message for the realm's election event and
   * publishes it. The body is the communication, the certificate login details, the stage of a PIN
   * link or else the event's error. A failed registration of an unknown user also sends error
   * notifications.
   */
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
    } else if (details.containsKey(PIN_LINK_STAGE_DETAIL)) {
      body = pinLinkBody(details);
    } else {
      // Use the event error (or another appropriate field) as body for
      // non-communications events.
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

  /** What happened on a voter's PIN link and why; its details never hold the token or the PIN. */
  static String pinLinkBody(Map<String, String> details) {
    String reason = details.get(Details.REASON);
    String stage = "pin_link " + details.get(PIN_LINK_STAGE_DETAIL);
    return reason == null ? stage : stage + " " + reason;
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

    // Generate a correlation ID.
    String correlationId = UUID.randomUUID().toString();
    Consumer<Exception> onFailure =
        e ->
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

    try {
      rabbitMqEventPublisher.publishInBackground(
          RabbitMqEventPublisher.electoralLogProperties(correlationId),
          RabbitMqEventPublisher.electoralLogBody(
              electionEventId, messageType, body, userId, tenantId, username),
          onFailure);
    } catch (Exception e) {
      onFailure.accept(e);
    }
  }

  @Override
  public void close() {}
}
