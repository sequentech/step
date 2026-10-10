// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.inetum_authenticator;

import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.RETURNS_DEEP_STUBS;
import static org.mockito.Mockito.doReturn;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import com.sun.net.httpserver.HttpExchange;
import com.sun.net.httpserver.HttpServer;
import java.io.IOException;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.concurrent.atomic.AtomicReference;
import java.util.logging.Handler;
import java.util.logging.Level;
import java.util.logging.LogRecord;
import java.util.logging.Logger;
import java.util.logging.SimpleFormatter;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.credential.hash.PasswordHashProvider;
import org.keycloak.credential.hash.Pbkdf2PasswordHashProvider;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.representations.userprofile.config.UPConfig;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.userprofile.UserProfileProvider;
import sequent.keycloak.authenticator.harvest.ServiceAccountTokenClient;

class LookupAndUpdateUserServiceTokenTest {
  private static final String SECRET = "synthetic-client-secret";
  private static final String TOKEN = "synthetic-access-token";
  private static final String TENANT_ID = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
  private static final String ELECTION_EVENT_ID = "4f6c1d1e-5b0a-4a43-9a4f-0e7f8a2b9c11";
  private static final String REALM_ID = "realm-id";
  private static final String TOKEN_PATH =
      "/realms/tenant-" + TENANT_ID + "/protocol/openid-connect/token";
  private static final String VERIFY_PATH = "/verify-application";

  private final AtomicReference<String> harvestAuthorization = new AtomicReference<>();
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
    for (Class<?> type : List.of(LookupAndUpdateUser.class, ServiceAccountTokenClient.class)) {
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

  private String serve(int tokenStatus, String tokenBody) throws IOException {
    server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
    server.createContext(TOKEN_PATH, exchange -> respond(exchange, tokenStatus, tokenBody));
    server.createContext(
        VERIFY_PATH,
        exchange -> {
          harvestAuthorization.set(exchange.getRequestHeaders().getFirst("Authorization"));
          respond(exchange, 500, "{\"message\":\"stub\"}");
        });
    server.start();
    return "127.0.0.1:" + server.getAddress().getPort();
  }

  private static void respond(HttpExchange exchange, int status, String body) throws IOException {
    exchange.getRequestBody().readAllBytes();
    byte[] bytes = body.getBytes(StandardCharsets.UTF_8);
    exchange.sendResponseHeaders(status, bytes.length);
    try (OutputStream out = exchange.getResponseBody()) {
      out.write(bytes);
    }
  }

  private static LookupAndUpdateUser newAuthenticator(String host) {
    return new LookupAndUpdateUser(
        new ServiceAccountTokenClient("http://" + host, "service-account", SECRET), host);
  }

  private static AuthenticationFlowContext enrollmentContext() {
    AuthenticationFlowContext context = mock(AuthenticationFlowContext.class, RETURNS_DEEP_STUBS);
    AuthenticationSessionModel authSession =
        mock(AuthenticationSessionModel.class, RETURNS_DEEP_STUBS);
    KeycloakSession session = mock(KeycloakSession.class, RETURNS_DEEP_STUBS);
    RealmModel realm = mock(RealmModel.class, RETURNS_DEEP_STUBS);
    UserProfileProvider userProfile = mock(UserProfileProvider.class);
    UPConfig userProfileConfig = new UPConfig();
    userProfileConfig.setAttributes(new ArrayList<>());
    AuthenticatorConfigModel config = new AuthenticatorConfigModel();
    config.setConfig(new HashMap<>(Map.of("messageCourierAttribute", "NONE")));

    when(context.getAuthenticationSession()).thenReturn(authSession);
    when(context.getSession()).thenReturn(session);
    when(context.getRealm()).thenReturn(realm);
    when(context.getAuthenticatorConfig()).thenReturn(config);
    when(realm.getId()).thenReturn(REALM_ID);
    when(session.realms().getRealm(REALM_ID).getName())
        .thenReturn("tenant-" + TENANT_ID + "-event-" + ELECTION_EVENT_ID);
    doReturn(mock(Pbkdf2PasswordHashProvider.class))
        .when(session)
        .getProvider(PasswordHashProvider.class, "pbkdf2-sha256");
    doReturn(userProfile).when(session).getProvider(UserProfileProvider.class);
    when(userProfile.getConfiguration()).thenReturn(userProfileConfig);
    when(authSession.getParentSession().getId()).thenReturn("auth-session-id");
    return context;
  }

  private String capturedLogs() {
    return String.join("\n", logMessages);
  }

  @Test
  void enrollmentSendsTheServiceTokenWithoutLoggingCredentials() throws Exception {
    String host = serve(200, "{\"access_token\":\"" + TOKEN + "\",\"refresh_token\":\"r\"}");
    AuthenticationFlowContext context = enrollmentContext();

    newAuthenticator(host).authenticate(context);

    assertEquals("Bearer " + TOKEN, harvestAuthorization.get());
    assertFalse(capturedLogs().contains(SECRET));
    assertFalse(capturedLogs().contains("client_secret"));
    assertFalse(capturedLogs().contains(TOKEN));
  }

  @Test
  void enrollmentStopsWhenTheServiceTokenIsRefused() throws Exception {
    String host = serve(401, "{\"error\":\"unauthorized_client\",\"echo\":\"" + SECRET + "\"}");
    AuthenticationFlowContext context = enrollmentContext();

    assertDoesNotThrow(() -> newAuthenticator(host).authenticate(context));

    assertNull(harvestAuthorization.get());
    verify(context).failureChallenge(eq(AuthenticationFlowError.INTERNAL_ERROR), any());
    assertFalse(capturedLogs().contains(SECRET));
  }
}
