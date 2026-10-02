// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.sun.net.httpserver.HttpExchange;
import com.sun.net.httpserver.HttpServer;
import java.io.IOException;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.net.ServerSocket;
import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.time.Duration;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import sequent.keycloak.authenticator.CapturedLogs;
import sequent.keycloak.authenticator.harvest.ServiceAccountTokenClient;

class HarvestMessageSenderProviderTest {
  private static final String TENANT = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
  private static final String TOKEN = "synthetic-access-token";
  private static final String CODE = "482619";
  private static final String DESTINATION = "+15550123456";
  private static final String TEXT = "Your synthetic verification code is " + CODE;

  private final ObjectMapper json = new ObjectMapper();
  private final Map<String, String> bodies = new ConcurrentHashMap<>();
  private final Map<String, String> authorizations = new ConcurrentHashMap<>();
  private final Map<String, Response> responses = new ConcurrentHashMap<>();
  private HttpServer server;
  private URI base;

  private record Response(int status, String body, long delayMillis) {}

  @BeforeEach
  void start() throws IOException {
    server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
    server.createContext("/", this::handle);
    server.start();
    base = URI.create("http://127.0.0.1:" + server.getAddress().getPort());
    respond(
        "/realms/tenant-" + TENANT + "/protocol/openid-connect/token",
        200,
        "{\"access_token\":\"" + TOKEN + "\"}");
  }

  @AfterEach
  void stop() {
    server.stop(0);
  }

  private void handle(HttpExchange exchange) throws IOException {
    String path = exchange.getRequestURI().getPath();
    bodies.put(path, new String(exchange.getRequestBody().readAllBytes(), StandardCharsets.UTF_8));
    String authorization = exchange.getRequestHeaders().getFirst("Authorization");
    if (authorization != null) {
      authorizations.put(path, authorization);
    }
    Response response = responses.getOrDefault(path, new Response(404, "{}", 0));
    if (response.delayMillis() > 0) {
      try {
        Thread.sleep(response.delayMillis());
      } catch (InterruptedException e) {
        Thread.currentThread().interrupt();
      }
    }
    byte[] bytes = response.body().getBytes(StandardCharsets.UTF_8);
    exchange.sendResponseHeaders(response.status(), bytes.length);
    try (OutputStream out = exchange.getResponseBody()) {
      out.write(bytes);
    }
  }

  private void respond(String path, int status, String body) {
    responses.put(path, new Response(status, body, 0));
  }

  private HarvestMessageSenderProvider sender(URI harvest) {
    return new HarvestMessageSenderProvider(
        harvest,
        Set.of(MessageChannel.WHATSAPP, MessageChannel.MESSENGER),
        new ServiceAccountTokenClient(base.toString(), "service-account", "synthetic-secret"),
        Duration.ofMillis(500));
  }

  private SendMessageRequest otp() {
    return new SendMessageRequest(
        TENANT,
        "11111111-2222-3333-4444-555555555555",
        "synthetic-voter",
        MessageChannel.WHATSAPP,
        MessagePurpose.OTP,
        DESTINATION,
        "en",
        new MessageContent(null, TEXT, null, List.of(CODE, "5"), CODE),
        "otp:synthetic-code-id",
        "2026-10-02T10:05:00Z");
  }

  @Test
  void sendPostsTheContractWithAServiceAccountTokenAndLogsNoSecrets() throws Exception {
    respond("/messages/send", 200, "{\"message_id\":\"m-1\",\"state\":\"ACCEPTED\"}");

    SendMessageResponse response;
    try (CapturedLogs logs = new CapturedLogs(HarvestMessageSenderProvider.class)) {
      response = sender(base).send(otp());
      String text = logs.text();
      assertFalse(text.contains(CODE));
      assertFalse(text.contains(TOKEN));
      assertFalse(text.contains(DESTINATION));
      assertFalse(text.contains(TEXT));
    }

    assertEquals(MessageAttemptState.ACCEPTED, response.attemptState());
    assertEquals("m-1", response.messageId());
    assertEquals("Bearer " + TOKEN, authorizations.get("/messages/send"));
    JsonNode body = json.readTree(bodies.get("/messages/send"));
    assertEquals(TENANT, body.get("tenant_id").asText());
    assertEquals("11111111-2222-3333-4444-555555555555", body.get("election_event_id").asText());
    assertEquals("synthetic-voter", body.get("voter_id").asText());
    assertEquals("WHATSAPP", body.get("channel").asText());
    assertEquals("OTP", body.get("purpose").asText());
    assertEquals(DESTINATION, body.get("destination").asText());
    assertEquals("en", body.get("language").asText());
    assertEquals(TEXT, body.get("content").get("text").asText());
    assertEquals(CODE, body.get("content").get("code").asText());
    assertEquals(CODE, body.get("content").get("template_parameters").get(0).asText());
    assertEquals("otp:synthetic-code-id", body.get("logical_key").asText());
    assertEquals("2026-10-02T10:05:00Z", body.get("expires_at").asText());
  }

  @Test
  void anUnknownOutcomeStaysUnknown() {
    respond("/messages/send", 200, "{\"message_id\":\"m-2\",\"state\":\"UNKNOWN\"}");
    assertEquals(MessageAttemptState.UNKNOWN, sender(base).send(otp()).attemptState());
  }

  @Test
  void anUnrecognisedStateIsUnknown() {
    respond("/messages/send", 200, "{\"message_id\":\"m-3\",\"state\":\"SENT_PROBABLY\"}");
    assertEquals(MessageAttemptState.UNKNOWN, sender(base).send(otp()).attemptState());
  }

  @Test
  void aTimeoutAfterDispatchIsUnknownNotFailed() {
    responses.put(
        "/messages/send", new Response(200, "{\"message_id\":\"m\",\"state\":\"ACCEPTED\"}", 2000));
    assertEquals(MessageAttemptState.UNKNOWN, sender(base).send(otp()).attemptState());
  }

  @Test
  void aRejectedRequestFails() {
    respond("/messages/send", 422, "{\"message\":\"echo " + CODE + "\"}");
    SendMessageResponse response = sender(base).send(otp());
    assertEquals(MessageAttemptState.FAILED, response.attemptState());
    assertFalse(String.valueOf(response.reason()).contains(CODE));
  }

  @Test
  void aServerErrorMayHaveBeenSentSoItIsUnknown() {
    respond("/messages/send", 503, "{}");
    assertEquals(MessageAttemptState.UNKNOWN, sender(base).send(otp()).attemptState());
  }

  @Test
  void withoutATokenNothingIsSent() {
    respond("/realms/tenant-" + TENANT + "/protocol/openid-connect/token", 401, "{}");
    respond("/messages/send", 200, "{\"message_id\":\"m\",\"state\":\"ACCEPTED\"}");
    assertEquals(MessageAttemptState.FAILED, sender(base).send(otp()).attemptState());
    assertNull(bodies.get("/messages/send"));
  }

  @Test
  void anUnreachableHarvestFails() throws IOException {
    int closedPort;
    try (ServerSocket socket = new ServerSocket(0)) {
      closedPort = socket.getLocalPort();
    }
    SendMessageResponse response = sender(URI.create("http://127.0.0.1:" + closedPort)).send(otp());
    assertEquals(MessageAttemptState.FAILED, response.attemptState());
  }

  @Test
  void messengerLinksAreCreatedPolledAndConfirmed() throws Exception {
    respond(
        "/messages/link",
        200,
        "{\"reference\":\"ref-1\",\"link\":\"https://m.me/synthetic?ref=ref-1\","
            + "\"link_word\":\"MAPLE\",\"expires_at\":\"2026-10-02T10:05:00Z\"}");
    respond("/messages/link/status", 200, "{\"state\":\"CODE_SENT\"}");
    respond(
        "/messages/link/confirm",
        200,
        "{\"state\":\"CONFIRMED\",\"page_scoped_id\":\"psid-1\",\"page_id\":\"page-1\"}");
    HarvestMessageSenderProvider sender = sender(base);

    CreateMessengerLinkResponse link =
        sender.createMessengerLink(
            new CreateMessengerLinkRequest(
                TENANT,
                null,
                "session-digest",
                "challenge-digest",
                CODE,
                "en",
                MessageContent.text(TEXT),
                "2026-10-02T10:05:00Z"));
    MessengerLinkRequest request =
        new MessengerLinkRequest(TENANT, link.reference(), "session-digest", "challenge-digest");
    MessengerLinkStatus status = sender.messengerLinkStatus(request);
    MessengerLinkStatus confirmed = sender.confirmMessengerLink(request);

    assertEquals("https://m.me/synthetic?ref=ref-1", link.link());
    assertEquals("MAPLE", link.linkWord());
    assertEquals(MessengerLinkState.CODE_SENT, status.linkState());
    assertEquals(MessengerLinkState.CONFIRMED, confirmed.linkState());
    assertEquals("psid-1", confirmed.pageScopedId());
    assertEquals("page-1", confirmed.pageId());
    JsonNode created = json.readTree(bodies.get("/messages/link"));
    assertEquals("session-digest", created.get("auth_session").asText());
    assertEquals("challenge-digest", created.get("challenge").asText());
    assertEquals(CODE, created.get("code").asText());
    assertFalse(created.has("election_event_id"));
    JsonNode confirm = json.readTree(bodies.get("/messages/link/confirm"));
    assertEquals("ref-1", confirm.get("reference").asText());
    assertEquals(TENANT, confirm.get("tenant_id").asText());
    assertEquals("Bearer " + TOKEN, authorizations.get("/messages/link/confirm"));
  }

  @Test
  void aRefusedConfirmationThrowsWithoutEchoingTheResponse() {
    respond("/messages/link/confirm", 409, "{\"message\":\"" + CODE + "\"}");
    IOException error =
        assertThrows(
            IOException.class,
            () ->
                sender(base)
                    .confirmMessengerLink(
                        new MessengerLinkRequest(TENANT, "ref-1", "session", "challenge")));
    assertFalse(error.getMessage().contains(CODE));
  }

  @Test
  void channelsComeFromConfigurationAndIgnoreUnknownNames() {
    assertEquals(
        Set.of(MessageChannel.WHATSAPP, MessageChannel.VIBER),
        HarvestMessageSenderProviderFactory.parseChannels(" whatsapp,VIBER,carrier-pigeon "));
    assertEquals(
        Set.of(MessageChannel.WHATSAPP, MessageChannel.VIBER, MessageChannel.MESSENGER),
        HarvestMessageSenderProviderFactory.parseChannels(null));
  }

  @Test
  void harvestDomainsWithoutSchemeUseHttp() {
    assertEquals(
        java.util.Optional.of(URI.create("http://harvest:8400")),
        HarvestMessageSenderProviderFactory.harvestUri("harvest:8400"));
    assertEquals(
        java.util.Optional.of(URI.create("https://harvest.example")),
        HarvestMessageSenderProviderFactory.harvestUri("https://harvest.example/"));
  }

  @Test
  void withoutAHarvestUrlTheFactoryOffersNoChannelsInsteadOfFailingToStart() {
    assertEquals(java.util.Optional.empty(), HarvestMessageSenderProviderFactory.harvestUri(null));
    assertEquals(java.util.Optional.empty(), HarvestMessageSenderProviderFactory.harvestUri(" "));
    HarvestMessageSenderProviderFactory factory = new HarvestMessageSenderProviderFactory();
    factory.init(org.mockito.Mockito.mock(org.keycloak.Config.Scope.class));
    MessageSenderProvider sender = factory.create(null);
    if (System.getenv("HARVEST_DOMAIN") == null) {
      assertTrue(sender.getChannels().isEmpty());
      assertEquals(MessageAttemptState.FAILED, sender.send(otp()).attemptState());
    }
  }

  @Test
  void theDefaultSenderDeliversNoMessagingAppChannels() {
    DefaultMessageSenderProvider sender = new DefaultMessageSenderProvider();
    assertTrue(sender.getChannels().isEmpty());
    assertFalse(sender.delivers(MessageChannel.WHATSAPP));
    assertEquals(MessageAttemptState.FAILED, sender.send(otp()).attemptState());
    assertThrows(
        IOException.class,
        () -> sender.messengerLinkStatus(new MessengerLinkRequest(TENANT, "r", "s", "c")));
  }
}
