// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.custom_event_listener;

import com.fasterxml.jackson.databind.ObjectMapper;
import jakarta.ws.rs.core.HttpHeaders;
import java.io.IOException;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import java.time.Duration;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.UUID;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.events.Event;
import org.keycloak.events.EventType;
import org.keycloak.models.KeycloakSession;
import sequent.keycloak.authenticator.harvest.ServiceAccountTokenClient;

/** Direct private attribution: no network metadata enters the ordinary audit publisher. */
@JBossLog
final class SigninNetworkSender {
  private static final String PATH = "/privacy/sign-in-network";
  private static final String FORENSIC_STORE = "forensic-store";
  private static final Duration TIMEOUT = Duration.ofSeconds(2);
  private static final ObjectMapper JSON = new ObjectMapper();
  private final URI harvest;
  private final ServiceAccountTokenClient tokens;
  private final HttpClient http;
  private final Duration timeout;

  SigninNetworkSender(
      URI harvest, ServiceAccountTokenClient tokens, HttpClient http, Duration timeout) {
    this.harvest = harvest;
    this.tokens = tokens;
    this.http = http;
    this.timeout = timeout;
  }

  static SigninNetworkSender fromEnvironment() {
    return new SigninNetworkSender(
        harvestUri(System.getenv("HARVEST_DOMAIN")),
        ServiceAccountTokenClient.shared(),
        HttpClient.newBuilder()
            .connectTimeout(TIMEOUT)
            .followRedirects(HttpClient.Redirect.NEVER)
            .build(),
        TIMEOUT);
  }

  static URI harvestUri(String configured) {
    if (configured == null || configured.isBlank()) {
      return null;
    }
    String value = configured.trim();
    URI uri = URI.create(value.contains("://") ? value : "http://" + value);
    if ((!"http".equals(uri.getScheme()) && !"https".equals(uri.getScheme()))
        || uri.getHost() == null
        || uri.getUserInfo() != null
        || uri.getQuery() != null
        || uri.getFragment() != null
        || (!uri.getPath().isEmpty() && !"/".equals(uri.getPath()))) {
      throw new IllegalArgumentException("HARVEST_DOMAIN must be an HTTP(S) service origin");
    }
    return uri;
  }

  static boolean eligible(LogRealm realm, Event event) {
    return realm.kind() == LogRealm.Kind.EVENT
        && event.getType() == EventType.LOGIN
        && event.getError() == null
        && uuid(realm.tenantId())
        && uuid(realm.electionEventId())
        && uuid(event.getUserId());
  }

  /**
   * Called after a successful authentication transaction. Policy is checked on every attempt;
   * failed checks send no metadata. Transport failure does not invalidate the completed login.
   */
  void capture(KeycloakSession session, LogRealm realm, Event event) {
    if (harvest == null || !eligible(realm, event)) {
      return;
    }
    try {
      String token =
          tokens.fetchAccessToken(ServiceAccountTokenClient.tenantRealmName(realm.tenantId()));
      HttpRequest preflight =
          request(
                  PATH
                      + "/policy?tenant_id="
                      + realm.tenantId()
                      + "&election_event_id="
                      + realm.electionEventId(),
                  token)
              .GET()
              .build();
      HttpResponse<String> policy = http.send(preflight, HttpResponse.BodyHandlers.ofString());
      if (policy.statusCode() != 200) {
        log.warn("Sign-in forensic policy could not be checked; attribution was not sent");
        return;
      }
      if (!FORENSIC_STORE.equals(
          JSON.readTree(policy.body()).path("network_data_policy").asText())) {
        return;
      }
      String ip = event.getIpAddress();
      if (ip == null || ip.isBlank() || ip.length() > 45) {
        return;
      }
      HttpHeaders headers =
          session.getContext() == null ? null : session.getContext().getRequestHeaders();
      String country = headers == null ? null : headers.getHeaderString("CF-IPCountry");
      if (country != null && !country.matches("[A-Za-z]{2}")) {
        country = null;
      }
      String userAgent = headers == null ? null : headers.getHeaderString("User-Agent");
      if (userAgent != null && userAgent.getBytes(StandardCharsets.UTF_8).length > 1024) {
        userAgent = null;
      }
      Map<String, Object> payload = new LinkedHashMap<>();
      payload.put("tenant_id", realm.tenantId());
      payload.put("election_event_id", realm.electionEventId());
      payload.put("voter_id", event.getUserId());
      payload.put(
          "delivery_id",
          event.getId() == null || event.getId().isBlank()
              ? UUID.randomUUID().toString()
              : event.getId());
      payload.put("occurred_at_ms", event.getTime());
      payload.put("ip", ip);
      payload.put("country", country);
      payload.put("user_agent", userAgent);
      HttpRequest post =
          request(PATH, token)
              .header("Content-Type", "application/json")
              .POST(HttpRequest.BodyPublishers.ofString(JSON.writeValueAsString(payload)))
              .build();
      HttpResponse<Void> response = http.send(post, HttpResponse.BodyHandlers.discarding());
      if (response.statusCode() != 204) {
        log.warn("Sign-in forensic attribution was not accepted");
      }
    } catch (InterruptedException interrupted) {
      Thread.currentThread().interrupt();
      log.warn("Sign-in forensic capture was interrupted");
    } catch (IOException | RuntimeException failure) {
      // Never print exceptions: transport errors may contain URLs, headers or request bodies.
      log.warn("Sign-in forensic capture was unavailable; attribution may be missing");
    }
  }

  private HttpRequest.Builder request(String path, String token) {
    return HttpRequest.newBuilder(harvest.resolve(path))
        .timeout(timeout)
        .header("Authorization", "Bearer " + token);
  }

  private static boolean uuid(String value) {
    try {
      return value != null && UUID.fromString(value).toString().equalsIgnoreCase(value);
    } catch (IllegalArgumentException invalid) {
      return false;
    }
  }
}
