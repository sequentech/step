// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.inetum_authenticator;

import static sequent.keycloak.authenticator.Utils.sendErrorNotificationToUser;

import java.io.IOException;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.util.HashMap;
import java.util.Map;
import java.util.Optional;
import java.util.concurrent.CompletableFuture;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.events.Event;
import org.keycloak.events.EventListenerProvider;
import org.keycloak.events.EventType;
import org.keycloak.events.admin.AdminEvent;
import org.keycloak.models.KeycloakSession;
import org.keycloak.util.JsonSerialization;

@JBossLog
public class CustomEventListenerProvider implements EventListenerProvider {

  private final KeycloakSession session;
  private String keycloakUrl = System.getenv("KEYCLOAK_URL");
  private String tenantId = System.getenv("SUPER_ADMIN_TENANT_ID");
  private String clientId = System.getenv("KEYCLOAK_CLIENT_ID");
  private String clientSecret = System.getenv("KEYCLOAK_CLIENT_SECRET");
  private String harvestUrl = System.getenv("HARVEST_DOMAIN");
  private String access_token;

  public CustomEventListenerProvider(KeycloakSession session) {
    this.session = session;
  }

  @Override
  public void close() {}

  @Override
  public void onEvent(Event event) {
    if (this.access_token == null) {
      authenticate();
    }

    if (event.getType() == EventType.REGISTER_ERROR && "userNotFound".equals(event.getError())) {
      try {
        sendErrorNotificationToUser(session, event.getRealmId(), event);
      } catch (Exception e) {
        log.error("Failed to send error notification", e);
      }
    }
    if (event.getDetails() == null) {
      logEvent(
          getElectionEventId(event.getRealmId()),
          event.getType(),
          event.getError(),
          event.getUserId());
    }

    log.infov("onEvent() event details to string: {0}", event.getDetails().toString());
    log.infov("onEvent() event getType: {0}", event.getType().toString());
    log.infov("onEvent() event getUserId: {0}", event.getUserId());
    String eventType = event.getDetails().get("type");
    log.infov("onEvent() event type: {0}", eventType);
    if (Utils.EVENT_TYPE_COMMUNICATIONS.equals(eventType)) {
      handleCommunicationsEvent(event);
    } else {
      String body = Optional.ofNullable(event.getDetails().get("msgBody")).orElse("-").replace("\n", " ");
      logEvent(
          getElectionEventId(event.getRealmId()),
          event.getType(),
          body,
          event.getUserId());
    }
  }

  private void handleCommunicationsEvent(Event event) {
    String msgBody = Optional.ofNullable(event.getDetails().get("msgBody")).orElse("");

    String body =
        String.format("%s %s", Utils.EVENT_TYPE_COMMUNICATIONS, msgBody).replace("\n", " ");

    logEvent(getElectionEventId(event.getRealmId()), event.getType(), body, event.getUserId());
  }

  public void authenticate() {
    HttpClient client = HttpClient.newHttpClient();
    String url =
        this.keycloakUrl
            + "/realms/"
            + getTenantRealmName(this.tenantId)
            + "/protocol/openid-connect/token";
    Map<Object, Object> data = new HashMap<>();
    data.put("client_id", this.clientId);
    data.put("scope", "openid");
    data.put("client_secret", this.clientSecret);
    data.put("grant_type", "client_credentials");

    String form =
        data.entrySet().stream()
            .map(entry -> entry.getKey() + "=" + entry.getValue())
            .reduce((entry1, entry2) -> entry1 + "&" + entry2)
            .orElse("");
    log.info(form);
    HttpRequest request =
        HttpRequest.newBuilder()
            .uri(URI.create(url))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .POST(HttpRequest.BodyPublishers.ofString(form))
            .build();

    CompletableFuture<HttpResponse<String>> responseFuture;
    responseFuture = client.sendAsync(request, HttpResponse.BodyHandlers.ofString());
    String responseBody = responseFuture.join().body();
    Object accessToken;
    try {
      log.info("responseBody " + responseBody);
      accessToken = JsonSerialization.readValue(responseBody, Map.class).get("access_token");
      log.info("authenticate " + accessToken.toString());
      this.access_token = accessToken.toString();
    } catch (IOException e) {
      e.printStackTrace();
    }
  }

  @Override
  public void onEvent(AdminEvent event, boolean includeRepresentation) {
    log.info("an admin event was fired, realmName: ");
  }

  private String getTenantRealmName(String realmName) {
    return "tenant-" + tenantId;
  }

  private String getElectionEventId(String realmId) {
    String realmName = session.realms().getRealm(realmId).getName();
    String[] parts = realmName.split("event-");
    if (parts.length > 1) {
      return parts[1];
    }
    return null;
  }

  private void logEvent(String electionEventId, EventType eventType, String body, String userId) {

    log.infov("logEvent(): user id: {0}", userId);
    log.infov("logEvent(): body: {0}", body);
    HttpClient client = HttpClient.newHttpClient();
    String url = "http://" + this.harvestUrl + "/immudb/log-event";
    String requestBody =
        String.format(
            "{\"election_event_id\": \"%s\", \"message_type\": \"%s\", \"body\" : \"%s\", \"user_id\": \"%s\"}",
            Utils.escapeJson(electionEventId),
            Utils.escapeJson(eventType.toString()),
            Utils.escapeJson(body),
            Utils.escapeJson(userId));
    HttpRequest request =
        HttpRequest.newBuilder()
            .uri(URI.create(url))
            .header("Content-Type", "application/json")
            .header("Authorization", "Bearer " + this.access_token)
            .POST(HttpRequest.BodyPublishers.ofString(requestBody))
            .build();
    CompletableFuture<HttpResponse<String>> response =
        client.sendAsync(request, HttpResponse.BodyHandlers.ofString());
    response
        .thenAccept(
            res -> {
              log.info("success");
            })
        .exceptionally(
            e -> {
              log.error(e);
              return null;
            });
  }
}
