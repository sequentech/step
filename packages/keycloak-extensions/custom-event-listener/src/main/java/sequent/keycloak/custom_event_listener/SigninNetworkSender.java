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

  enum Delivery {
    COMPLETE,
    RETRY
  }

  record Evidence(
      String tenantId,
      String electionEventId,
      String voterId,
      String deliveryId,
      long occurredAtMs,
      String ip,
      String country,
      String userAgent) {
    @Override
    public String toString() {
      return "SigninEvidence[redacted]";
    }
  }

  /** Copy bounded request fields while the session is live; the worker never holds a session. */
  static Evidence snapshot(KeycloakSession session, LogRealm realm, Event event) {
    if (!eligible(realm, event)) return null;
    String ip = event.getIpAddress();
    if (ip == null || ip.isBlank() || ip.length() > 45) return null;
    HttpHeaders headers =
        session.getContext() == null ? null : session.getContext().getRequestHeaders();
    String country = headers == null ? null : headers.getHeaderString("CF-IPCountry");
    if (country != null && !country.matches("[A-Za-z]{2}")) country = null;
    String userAgent = headers == null ? null : headers.getHeaderString("User-Agent");
    if (userAgent != null && userAgent.getBytes(StandardCharsets.UTF_8).length > 1024)
      userAgent = null;
    String deliveryId =
        event.getId() == null || event.getId().isBlank() || event.getId().length() > 128
            ? UUID.randomUUID().toString()
            : event.getId();
    return new Evidence(
        realm.tenantId(),
        realm.electionEventId(),
        event.getUserId(),
        deliveryId,
        event.getTime(),
        ip,
        country,
        userAgent);
  }

  /**
   * Every attempt rechecks policy before transmitting attribution; identity stays stable on retry.
   */
  Delivery deliver(Evidence evidence) {
    if (harvest == null || evidence == null) return Delivery.COMPLETE;
    try {
      String token =
          tokens.fetchAccessToken(ServiceAccountTokenClient.tenantRealmName(evidence.tenantId()));
      HttpRequest preflight =
          request(
                  PATH
                      + "/policy?tenant_id="
                      + evidence.tenantId()
                      + "&election_event_id="
                      + evidence.electionEventId(),
                  token)
              .GET()
              .build();
      HttpResponse<String> policy = http.send(preflight, HttpResponse.BodyHandlers.ofString());
      if (policy.statusCode() != 200) {
        log.warn("Sign-in forensic policy could not be checked; attribution was not sent");
        return retryable(policy.statusCode()) ? Delivery.RETRY : Delivery.COMPLETE;
      }
      if (!FORENSIC_STORE.equals(
          JSON.readTree(policy.body()).path("network_data_policy").asText())) {
        return Delivery.COMPLETE;
      }
      Map<String, Object> payload = new LinkedHashMap<>();
      payload.put("tenant_id", evidence.tenantId());
      payload.put("election_event_id", evidence.electionEventId());
      payload.put("voter_id", evidence.voterId());
      payload.put("delivery_id", evidence.deliveryId());
      payload.put("occurred_at_ms", evidence.occurredAtMs());
      payload.put("ip", evidence.ip());
      payload.put("country", evidence.country());
      payload.put("user_agent", evidence.userAgent());
      HttpRequest post =
          request(PATH, token)
              .header("Content-Type", "application/json")
              .POST(HttpRequest.BodyPublishers.ofString(JSON.writeValueAsString(payload)))
              .build();
      HttpResponse<Void> response = http.send(post, HttpResponse.BodyHandlers.discarding());
      if (response.statusCode() != 204) {
        log.warn("Sign-in forensic attribution was not accepted");
      }
      return retryable(response.statusCode()) ? Delivery.RETRY : Delivery.COMPLETE;
    } catch (InterruptedException interrupted) {
      Thread.currentThread().interrupt();
      log.warn("Sign-in forensic capture was interrupted");
      return Delivery.COMPLETE;
    } catch (IOException failure) {
      // Never print exceptions: transport errors may contain URLs, headers or request bodies.
      log.warn("Sign-in forensic capture was unavailable");
      return Delivery.RETRY;
    } catch (RuntimeException failure) {
      log.warn("Sign-in forensic capture could not be prepared");
      return Delivery.COMPLETE;
    }
  }

  private static boolean retryable(int status) {
    return status == 429 || status >= 500;
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
