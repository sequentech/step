// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.custom_event_listener;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.*;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import jakarta.ws.rs.core.HttpHeaders;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.time.Duration;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.Flow;
import java.util.concurrent.atomic.AtomicInteger;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.events.Event;
import org.keycloak.events.EventType;
import org.keycloak.models.KeycloakContext;
import org.keycloak.models.KeycloakSession;
import sequent.keycloak.authenticator.harvest.ServiceAccountTokenClient;

class SigninNetworkSenderTest {
  private static final String TENANT = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
  private static final String EVENT = "6f1c3a6e-2a61-4d6b-9a52-3f0f8a3c2b10";
  private static final String VOTER = "0b9f6c5e-7d3a-4a1e-8c2f-5e4d3c2b1a09";
  private final ObjectMapper json = new ObjectMapper();
  private final List<HttpRequest> requests = new ArrayList<>();
  private HttpClient http;
  private ServiceAccountTokenClient tokens;
  private KeycloakSession session;
  private HttpHeaders headers;
  private Event event;
  private LogRealm realm;
  private SigninNetworkSender sender;
  private String policy;
  private int policyStatus;

  @BeforeEach
  void setup() throws Exception {
    http = mock(HttpClient.class);
    tokens = mock(ServiceAccountTokenClient.class);
    session = mock(KeycloakSession.class);
    KeycloakContext context = mock(KeycloakContext.class);
    headers = mock(HttpHeaders.class);
    when(session.getContext()).thenReturn(context);
    when(context.getRequestHeaders()).thenReturn(headers);
    when(headers.getHeaderString("User-Agent")).thenReturn("Synthetic browser");
    when(headers.getHeaderString("CF-IPCountry")).thenReturn("ES");
    when(tokens.fetchAccessToken("tenant-" + TENANT)).thenReturn("synthetic-service-token");
    policy = "{\"network_data_policy\":\"forensic-store\"}";
    policyStatus = 200;
    when(http.send(any(HttpRequest.class), any(HttpResponse.BodyHandler.class)))
        .thenAnswer(
            invocation -> {
              HttpRequest request = invocation.getArgument(0);
              requests.add(request);
              HttpResponse<String> response = mock(HttpResponse.class);
              when(response.statusCode())
                  .thenReturn(request.method().equals("GET") ? policyStatus : 204);
              when(response.body()).thenReturn(request.method().equals("GET") ? policy : "");
              return response;
            });
    realm = LogRealm.fromName("tenant-" + TENANT + "-event-" + EVENT);
    event = new Event();
    event.setType(EventType.LOGIN);
    event.setId("synthetic-delivery");
    event.setUserId(VOTER);
    event.setTime(1778511600123L);
    event.setIpAddress("198.51.100.7");
    sender =
        new SigninNetworkSender(
            URI.create("http://harvest:8000"), tokens, http, Duration.ofSeconds(2));
  }

  @Test
  void sendsOnlyAfterExplicitPolicyApprovalWithTenantAuthentication() throws Exception {
    sender.deliver(SigninNetworkSender.snapshot(session, realm, event));
    assertEquals(2, requests.size());
    HttpRequest preflight = requests.get(0);
    assertEquals("GET", preflight.method());
    assertEquals(
        "http://harvest:8000/privacy/sign-in-network/policy?tenant_id="
            + TENANT
            + "&election_event_id="
            + EVENT,
        preflight.uri().toString());
    assertTrue(preflight.bodyPublisher().isEmpty());
    assertFalse(preflight.uri().toString().contains(VOTER));
    HttpRequest post = requests.get(1);
    assertEquals("http://harvest:8000/privacy/sign-in-network", post.uri().toString());
    assertEquals(
        "Bearer synthetic-service-token", post.headers().firstValue("Authorization").orElseThrow());
    assertEquals(Duration.ofSeconds(2), post.timeout().orElseThrow());
    assertEquals(
        json.readTree(
            """
        {"tenant_id":"90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
         "election_event_id":"6f1c3a6e-2a61-4d6b-9a52-3f0f8a3c2b10",
         "voter_id":"0b9f6c5e-7d3a-4a1e-8c2f-5e4d3c2b1a09",
         "delivery_id":"synthetic-delivery","occurred_at_ms":1778511600123,
         "ip":"198.51.100.7","country":"ES","user_agent":"Synthetic browser"}
        """),
        json.readTree(body(post)));
  }

  @Test
  void disabledMissingUnknownAndMalformedPolicyNeverTransmitNetworkMetadata() {
    for (String response :
        List.of(
            "{\"network_data_policy\":\"do-not-store\"}",
            "{}",
            "{\"network_data_policy\":\"future-policy\"}",
            "broken")) {
      requests.clear();
      policy = response;
      sender.deliver(SigninNetworkSender.snapshot(session, realm, event));
      assertEquals(1, requests.size());
      assertEquals("GET", requests.get(0).method());
    }
  }

  @Test
  void failedPolicyReadAndTokenFailureNeverSendMetadata() throws Exception {
    policyStatus = 403;
    sender.deliver(SigninNetworkSender.snapshot(session, realm, event));
    assertEquals(1, requests.size());
    requests.clear();
    when(tokens.fetchAccessToken(any())).thenThrow(new IOException("synthetic private failure"));
    assertDoesNotThrow(() -> sender.deliver(SigninNetworkSender.snapshot(session, realm, event)));
    assertTrue(requests.isEmpty());
  }

  @Test
  void signoutFailedLoginTenantRealmAndMissingIdentityDoNotContactHarvest() {
    event.setType(EventType.LOGOUT);
    sender.deliver(SigninNetworkSender.snapshot(session, realm, event));
    event.setType(EventType.LOGIN_ERROR);
    sender.deliver(SigninNetworkSender.snapshot(session, realm, event));
    event.setType(EventType.LOGIN);
    event.setError("invalid_user_credentials");
    sender.deliver(SigninNetworkSender.snapshot(session, realm, event));
    event.setError(null);
    sender.deliver(
        SigninNetworkSender.snapshot(session, LogRealm.fromName("tenant-" + TENANT), event));
    event.setUserId(null);
    sender.deliver(SigninNetworkSender.snapshot(session, realm, event));
    verifyNoInteractions(http, tokens, headers);
  }

  @Test
  void policyIsRecheckedOnEveryLoginAndOversizedMetadataIsOmitted() throws Exception {
    when(headers.getHeaderString("User-Agent")).thenReturn("é".repeat(600));
    when(headers.getHeaderString("CF-IPCountry")).thenReturn("untrusted country text");
    sender.deliver(SigninNetworkSender.snapshot(session, realm, event));
    JsonNode payload = json.readTree(body(requests.get(1)));
    assertTrue(payload.path("user_agent").isNull());
    assertTrue(payload.path("country").isNull());
    policy = "{\"network_data_policy\":\"do-not-store\"}";
    sender.deliver(SigninNetworkSender.snapshot(session, realm, event));
    assertEquals(3, requests.size());
    assertEquals("GET", requests.get(2).method());
  }

  @Test
  void transportFailureDoesNotFailLoginAndInterruptionIsPreserved() throws Exception {
    when(http.send(any(HttpRequest.class), any(HttpResponse.BodyHandler.class)))
        .thenThrow(new InterruptedException("synthetic private metadata"));
    try {
      assertDoesNotThrow(() -> sender.deliver(SigninNetworkSender.snapshot(session, realm, event)));
      assertTrue(Thread.currentThread().isInterrupted());
    } finally {
      Thread.interrupted();
    }
  }

  @Test
  void missingConfigurationDisablesTransportAndUnsafeUrlFormsAreRejected() {
    assertNull(SigninNetworkSender.harvestUri(null));
    assertEquals(URI.create("http://harvest:8000"), SigninNetworkSender.harvestUri("harvest:8000"));
    assertEquals(
        URI.create("https://harvest.internal"),
        SigninNetworkSender.harvestUri("https://harvest.internal"));
    for (String value :
        List.of(
            "file:///tmp/x",
            "http://user:password@harvest",
            "http://harvest?token=x",
            "http://harvest#fragment",
            "http://harvest/path")) {
      assertThrows(IllegalArgumentException.class, () -> SigninNetworkSender.harvestUri(value));
    }
  }

  @Test
  void failureLogsNeverContainNetworkIdentityTokensOrExceptions() throws Exception {
    java.util.logging.Logger logger =
        java.util.logging.Logger.getLogger(SigninNetworkSender.class.getName());
    List<java.util.logging.LogRecord> records = new ArrayList<>();
    java.util.logging.Handler handler =
        new java.util.logging.Handler() {
          public void publish(java.util.logging.LogRecord record) {
            records.add(record);
          }

          public void flush() {}

          public void close() {}
        };
    logger.addHandler(handler);
    try {
      when(http.send(any(HttpRequest.class), any(HttpResponse.BodyHandler.class)))
          .thenThrow(
              new IOException(
                  "198.51.100.7 " + VOTER + " Synthetic browser synthetic-service-token"));
      sender.deliver(SigninNetworkSender.snapshot(session, realm, event));
      assertFalse(records.isEmpty());
      for (java.util.logging.LogRecord record : records) {
        assertNull(record.getThrown());
        String message = record.getMessage();
        for (String privateValue :
            List.of("198.51.100.7", VOTER, "Synthetic browser", "synthetic-service-token")) {
          assertFalse(message.contains(privateValue));
        }
      }
    } finally {
      logger.removeHandler(handler);
    }
  }

  @Test
  void metadataPostIsAttemptedOnceAndNeverFollowsARefusalWithAnotherRequest() throws Exception {
    HttpResponse<String> allowed = mock(HttpResponse.class);
    when(allowed.statusCode()).thenReturn(200);
    when(allowed.body()).thenReturn(policy);
    HttpResponse<Void> refusal = mock(HttpResponse.class);
    when(refusal.statusCode()).thenReturn(503);
    when(http.send(any(HttpRequest.class), any(HttpResponse.BodyHandler.class)))
        .thenReturn(allowed)
        .thenReturn(refusal);
    assertDoesNotThrow(() -> sender.deliver(SigninNetworkSender.snapshot(session, realm, event)));
    verify(http, times(2)).send(any(HttpRequest.class), any(HttpResponse.BodyHandler.class));
  }

  @Test
  void pendingRegistrationRetriesWithSameDeliveryIdAndRechecksPolicy() throws Exception {
    SigninNetworkSender.Evidence snapshot = SigninNetworkSender.snapshot(session, realm, event);
    HttpResponse<String> enabled = mock(HttpResponse.class);
    when(enabled.statusCode()).thenReturn(200);
    when(enabled.body()).thenReturn(policy);
    HttpResponse<Void> pending = mock(HttpResponse.class);
    when(pending.statusCode()).thenReturn(503);
    HttpResponse<Void> accepted = mock(HttpResponse.class);
    when(accepted.statusCode()).thenReturn(204);
    AtomicInteger call = new AtomicInteger();
    when(http.send(any(HttpRequest.class), any(HttpResponse.BodyHandler.class)))
        .thenAnswer(
            invocation -> {
              HttpRequest request = invocation.getArgument(0);
              requests.add(request);
              int current = call.incrementAndGet();
              if (request.method().equals("GET")) return enabled;
              return current == 2 ? pending : accepted;
            });
    assertEquals(SigninNetworkSender.Delivery.RETRY, sender.deliver(snapshot));
    event.setUserId("not-the-original-voter");
    event.setIpAddress("203.0.113.9");
    when(headers.getHeaderString("User-Agent")).thenReturn("Mutated browser");
    assertEquals(SigninNetworkSender.Delivery.COMPLETE, sender.deliver(snapshot));
    assertEquals(
        List.of("GET", "POST", "GET", "POST"), requests.stream().map(HttpRequest::method).toList());
    assertEquals(body(requests.get(1)), body(requests.get(3)));
    assertEquals("198.51.100.7", json.readTree(body(requests.get(3))).path("ip").asText());
    when(enabled.body()).thenReturn("{\"network_data_policy\":\"do-not-store\"}");
    assertEquals(SigninNetworkSender.Delivery.COMPLETE, sender.deliver(snapshot));
    assertEquals(5, requests.size());
    assertEquals("GET", requests.get(4).method());
  }

  @Test
  void authorizationRefusalsAreTerminalButTransientHttpFailuresCanRetry() throws Exception {
    SigninNetworkSender.Evidence snapshot = SigninNetworkSender.snapshot(session, realm, event);
    for (int status : List.of(401, 403, 404)) {
      policyStatus = status;
      assertEquals(SigninNetworkSender.Delivery.COMPLETE, sender.deliver(snapshot));
    }
    for (int status : List.of(429, 500, 503)) {
      policyStatus = status;
      assertEquals(SigninNetworkSender.Delivery.RETRY, sender.deliver(snapshot));
    }
    assertEquals(6, requests.size());
    assertTrue(requests.stream().allMatch(request -> request.method().equals("GET")));
  }

  private String body(HttpRequest request) throws Exception {
    ByteArrayOutputStream bytes = new ByteArrayOutputStream();
    CompletableFuture<String> completed = new CompletableFuture<>();
    request
        .bodyPublisher()
        .orElseThrow()
        .subscribe(
            new Flow.Subscriber<ByteBuffer>() {
              public void onSubscribe(Flow.Subscription subscription) {
                subscription.request(Long.MAX_VALUE);
              }

              public void onNext(ByteBuffer buffer) {
                byte[] part = new byte[buffer.remaining()];
                buffer.get(part);
                bytes.writeBytes(part);
              }

              public void onError(Throwable failure) {
                completed.completeExceptionally(failure);
              }

              public void onComplete() {
                completed.complete(bytes.toString(StandardCharsets.UTF_8));
              }
            });
    return completed.get(1, java.util.concurrent.TimeUnit.SECONDS);
  }
}
