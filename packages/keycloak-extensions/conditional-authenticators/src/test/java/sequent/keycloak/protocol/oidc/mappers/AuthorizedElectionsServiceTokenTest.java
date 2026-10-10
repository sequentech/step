// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.protocol.oidc.mappers;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;

import com.sun.net.httpserver.HttpServer;
import java.io.IOException;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import java.util.logging.Handler;
import java.util.logging.Level;
import java.util.logging.LogRecord;
import java.util.logging.Logger;
import java.util.logging.SimpleFormatter;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import sequent.keycloak.authenticator.harvest.ServiceAccountTokenClient;

class AuthorizedElectionsServiceTokenTest {
  private static final String SECRET = "synthetic-client-secret";
  private static final String TOKEN = "synthetic-access-token";
  private static final String TENANT_ID = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
  private static final String TOKEN_PATH =
      "/realms/tenant-" + TENANT_ID + "/protocol/openid-connect/token";

  private final List<String> logMessages = new ArrayList<>();
  private final List<Logger> capturedLoggers = new ArrayList<>();
  private final Handler logHandler =
      new Handler() {
        private final SimpleFormatter formatter = new SimpleFormatter();

        @Override
        public void publish(LogRecord record) {
          logMessages.add(formatter.format(record));
        }

        @Override
        public void flush() {}

        @Override
        public void close() {}
      };
  private HttpServer server;

  @BeforeEach
  void captureLogs() {
    for (Class<?> type :
        List.of(AuthorizedElectionsUserAttributeMapper.class, ServiceAccountTokenClient.class)) {
      Logger logger = Logger.getLogger(type.getName());
      logger.setLevel(Level.ALL);
      logger.addHandler(logHandler);
      capturedLoggers.add(logger);
    }
  }

  @AfterEach
  void stop() {
    for (Logger logger : capturedLoggers) {
      logger.removeHandler(logHandler);
      logger.setLevel(null);
    }
    if (server != null) {
      server.stop(0);
    }
  }

  private String serve(int status, String body) throws IOException {
    server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
    server.createContext(
        TOKEN_PATH,
        exchange -> {
          exchange.getRequestBody().readAllBytes();
          byte[] bytes = body.getBytes(StandardCharsets.UTF_8);
          exchange.sendResponseHeaders(status, bytes.length);
          try (OutputStream out = exchange.getResponseBody()) {
            out.write(bytes);
          }
        });
    server.start();
    return "http://127.0.0.1:" + server.getAddress().getPort();
  }

  private static AuthorizedElectionsUserAttributeMapper newMapper(String keycloakUrl) {
    return new AuthorizedElectionsUserAttributeMapper(
        new ServiceAccountTokenClient(keycloakUrl, "service-account", SECRET));
  }

  private String capturedLogs() {
    return String.join("\n", logMessages);
  }

  @Test
  void serviceTokenIsFetchedWithoutLoggingCredentials() throws Exception {
    String url = serve(200, "{\"access_token\":\"" + TOKEN + "\",\"refresh_token\":\"r\"}");

    assertEquals(TOKEN, newMapper(url).authenticate(TENANT_ID));

    assertFalse(capturedLogs().contains(SECRET));
    assertFalse(capturedLogs().contains("client_secret"));
    assertFalse(capturedLogs().contains(TOKEN));
  }

  @Test
  void refusedServiceTokenRequestFails() throws Exception {
    String url = serve(401, "{\"error\":\"unauthorized_client\",\"echo\":\"" + SECRET + "\"}");
    AuthorizedElectionsUserAttributeMapper mapper = newMapper(url);

    assertThrows(IOException.class, () -> mapper.authenticate(TENANT_ID));
    assertFalse(capturedLogs().contains(SECRET));
  }
}
