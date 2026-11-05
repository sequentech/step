// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.authenticator.harvest;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.sun.net.httpserver.HttpServer;
import java.io.IOException;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.net.URLDecoder;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.atomic.AtomicReference;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;
import sequent.keycloak.authenticator.CapturedLogs;

class ServiceAccountTokenClientTest {
  private static final String SECRET = "synthetic-client-secret&scope=forged";
  private static final String TOKEN = "synthetic-access-token";
  private final AtomicReference<String> requestPath = new AtomicReference<>();
  private final AtomicReference<String> requestBody = new AtomicReference<>();
  private final java.util.concurrent.atomic.AtomicInteger requests =
      new java.util.concurrent.atomic.AtomicInteger();
  private HttpServer server;

  private String serve(int status, String body) throws IOException {
    server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
    server.createContext(
        "/",
        exchange -> {
          requests.incrementAndGet();
          requestPath.set(exchange.getRequestURI().getPath());
          requestBody.set(
              new String(exchange.getRequestBody().readAllBytes(), StandardCharsets.UTF_8));
          byte[] bytes = body.getBytes(StandardCharsets.UTF_8);
          exchange.sendResponseHeaders(status, bytes.length);
          try (OutputStream out = exchange.getResponseBody()) {
            out.write(bytes);
          }
        });
    server.start();
    return "http://127.0.0.1:" + server.getAddress().getPort();
  }

  @AfterEach
  void stop() {
    if (server != null) {
      server.stop(0);
    }
  }

  @Test
  void fetchesTheAccessTokenWithoutLoggingCredentials() throws Exception {
    String url = serve(200, "{\"access_token\":\"" + TOKEN + "\",\"refresh_token\":\"r\"}");
    ServiceAccountTokenClient client =
        new ServiceAccountTokenClient(url, "service-account", SECRET);

    try (CapturedLogs logs = new CapturedLogs(ServiceAccountTokenClient.class)) {
      assertEquals(TOKEN, client.fetchAccessToken("tenant-1"));
      assertFalse(logs.text().contains(SECRET));
      assertFalse(logs.text().contains(TOKEN));
    }

    assertEquals("/realms/tenant-1/protocol/openid-connect/token", requestPath.get());
    assertTrue(
        URLDecoder.decode(requestBody.get(), StandardCharsets.UTF_8)
            .contains("client_secret=" + SECRET));
    assertFalse(requestBody.get().contains("&scope=forged"));
  }

  @Test
  void rejectedRequestsFailWithoutExposingTheResponse() throws Exception {
    String url = serve(401, "{\"error\":\"unauthorized_client\",\"echo\":\"" + SECRET + "\"}");
    ServiceAccountTokenClient client =
        new ServiceAccountTokenClient(url, "service-account", SECRET);

    try (CapturedLogs logs = new CapturedLogs(ServiceAccountTokenClient.class)) {
      IOException error =
          assertThrows(IOException.class, () -> client.fetchAccessToken("tenant-1"));
      assertFalse(error.getMessage().contains(SECRET));
      assertFalse(logs.text().contains(SECRET));
    }
  }

  @Test
  void responsesWithoutATokenFail() throws Exception {
    String url = serve(200, "{\"token_type\":\"Bearer\"}");
    ServiceAccountTokenClient client =
        new ServiceAccountTokenClient(url, "service-account", SECRET);

    assertThrows(IOException.class, () -> client.fetchAccessToken("tenant-1"));
  }

  /** A clock the test moves forward. */
  private static final class MovableClock extends java.time.Clock {
    private java.time.Instant now = java.time.Instant.parse("2026-10-09T10:00:00Z");

    void advance(java.time.Duration by) {
      now = now.plus(by);
    }

    @Override
    public java.time.ZoneId getZone() {
      return java.time.ZoneOffset.UTC;
    }

    @Override
    public java.time.Clock withZone(java.time.ZoneId zone) {
      return this;
    }

    @Override
    public java.time.Instant instant() {
      return now;
    }
  }

  @Test
  void aTokenIsReusedUntilShortlyBeforeItExpires() throws Exception {
    String url = serve(200, "{\"access_token\":\"" + TOKEN + "\",\"expires_in\":300}");
    MovableClock clock = new MovableClock();
    ServiceAccountTokenClient client =
        new ServiceAccountTokenClient(url, "service-account", SECRET, clock);

    assertEquals(TOKEN, client.fetchAccessToken("tenant-1"));
    clock.advance(java.time.Duration.ofSeconds(200));
    assertEquals(TOKEN, client.fetchAccessToken("tenant-1"));
    assertEquals(1, requests.get());

    clock.advance(java.time.Duration.ofSeconds(80));
    assertEquals(TOKEN, client.fetchAccessToken("tenant-1"));
    assertEquals(2, requests.get());

    assertEquals(TOKEN, client.fetchAccessToken("tenant-2"));
    assertEquals(3, requests.get());
  }

  @Test
  void aTokenWithoutALifetimeIsNotCached() throws Exception {
    String url = serve(200, "{\"access_token\":\"" + TOKEN + "\"}");
    ServiceAccountTokenClient client =
        new ServiceAccountTokenClient(url, "service-account", SECRET);
    client.fetchAccessToken("tenant-1");
    client.fetchAccessToken("tenant-1");
    assertEquals(2, requests.get());
  }

  @Test
  void theTokenEndpointHasTimeouts() {
    assertEquals(java.time.Duration.ofSeconds(5), ServiceAccountTokenClient.CONNECT_TIMEOUT);
    assertEquals(java.time.Duration.ofSeconds(10), ServiceAccountTokenClient.READ_TIMEOUT);
  }
}
