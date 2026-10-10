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
  private HttpServer server;

  private String serve(int status, String body) throws IOException {
    server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
    server.createContext(
        "/",
        exchange -> {
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
}
