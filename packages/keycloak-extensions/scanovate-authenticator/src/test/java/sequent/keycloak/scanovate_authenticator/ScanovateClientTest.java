// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static sequent.keycloak.scanovate_authenticator.ScanovateResultsTest.json;

import com.fasterxml.jackson.databind.JsonNode;
import java.io.IOException;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Deque;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

class ScanovateClientTest {
  private static final String BASE_URL = "https://btrust.example.com/";

  record Call(String method, String url, Map<String, String> headers, String body) {}

  static class FakeTransport implements HttpTransport {
    final List<Call> calls = new ArrayList<>();
    final Deque<Object> replies = new ArrayDeque<>();

    FakeTransport reply(int status, String body) {
      replies.add(new HttpResult(status, body));
      return this;
    }

    FakeTransport fail(String message) {
      replies.add(new IOException(message));
      return this;
    }

    @Override
    public HttpResult get(String url, Map<String, String> headers) throws IOException {
      return next(new Call("GET", url, headers, null));
    }

    @Override
    public HttpResult postJson(String url, Map<String, String> headers, String body)
        throws IOException {
      return next(new Call("POST", url, headers, body));
    }

    private HttpResult next(Call call) throws IOException {
      calls.add(call);
      Object reply = replies.poll();
      if (reply instanceof IOException exception) {
        throw exception;
      }
      return (HttpResult) reply;
    }
  }

  private final List<Long> sleeps = new ArrayList<>();

  private ScanovateClient client(FakeTransport transport) {
    return new ScanovateClient(transport, BASE_URL, "client", "secret", 3, sleeps::add);
  }

  @Test
  void fetchAccessTokenPostsClientCredentials() throws IOException {
    FakeTransport transport =
        new FakeTransport().reply(200, "{\"access_token\": \"jwt\", \"token_type\": \"Bearer\"}");

    assertEquals("jwt", client(transport).fetchAccessToken());

    Call call = transport.calls.get(0);
    assertEquals("POST", call.method());
    assertEquals("https://btrust.example.com/auth/token", call.url());
    assertEquals(
        json("{\"client_id\": \"client\", \"client_secret\": \"secret\"}"), json(call.body()));
  }

  @Test
  void fetchAccessTokenWithoutTokenFails() {
    FakeTransport transport = new FakeTransport().reply(200, "{\"token_type\": \"Bearer\"}");
    assertThrows(IOException.class, () -> client(transport).fetchAccessToken());
  }

  @Test
  void unauthorizedIsNotRetried() {
    FakeTransport transport =
        new FakeTransport().reply(401, "{\"code\": \"401\", \"message\": \"Unauthorized\"}");
    assertThrows(IOException.class, () -> client(transport).fetchAccessToken());
    assertEquals(1, transport.calls.size());
    assertEquals(List.of(), sleeps);
  }

  @Test
  void transportErrorsAreRetriedWithBackoff() throws IOException {
    FakeTransport transport =
        new FakeTransport()
            .fail("connection reset")
            .reply(503, "unavailable")
            .reply(200, "{\"access_token\": \"jwt\"}");

    assertEquals("jwt", client(transport).fetchAccessToken());
    assertEquals(3, transport.calls.size());
    assertEquals(List.of(1_000L, 2_000L), sleeps);
  }

  @Test
  void retriesAreBounded() {
    FakeTransport transport =
        new FakeTransport().fail("down").fail("down").fail("down").fail("never used");
    assertThrows(IOException.class, () -> client(transport).fetchAccessToken());
    assertEquals(3, transport.calls.size());
  }

  @Test
  void createSessionLinkSendsFlowRequest() throws IOException {
    FakeTransport transport =
        new FakeTransport()
            .reply(
                200,
                "{\"success\": true, \"errorCode\": 0, \"data\": \"https://flow.example.com?urlId=u&cid=c&process_id=proc-1\"}");

    SessionLink link =
        client(transport)
            .createSessionLink(
                "jwt",
                new LinkRequest(
                    3659,
                    "ident-1",
                    null,
                    "https://kc/realms/r/login-actions/authenticate?session_code=a&execution=b",
                    Map.of("country", "Spain"),
                    SaveOption.DO_NOT_SAVE));

    assertEquals(
        new SessionLink("https://flow.example.com?urlId=u&cid=c&process_id=proc-1", "proc-1"),
        link);
    Call call = transport.calls.get(0);
    assertEquals("https://btrust.example.com/flow/v3/link", call.url());
    assertEquals("Bearer jwt", call.headers().get("Authorization"));
    JsonNode body = json(call.body());
    assertEquals(3659, body.get("flow_id").asInt());
    assertEquals("ident-1", body.get("identifier_id").asText());
    assertEquals(
        "https://kc/realms/r/login-actions/authenticate?session_code=a&execution=b",
        body.get("redirect_url").asText());
    assertEquals("Spain", body.at("/params/country").asText());
    assertEquals("do_not_save", body.get("save_option").asText());
    assertFalse(body.has("id_number"));
  }

  @Test
  void createSessionLinkOmitsDefaultSaveOption() throws IOException {
    FakeTransport transport =
        new FakeTransport()
            .reply(200, "{\"success\": true, \"errorCode\": 0, \"data\": \"https://f\"}");

    SessionLink link =
        client(transport)
            .createSessionLink(
                "jwt",
                new LinkRequest(1, "ident-1", "ID-9", "https://r", Map.of(), SaveOption.DEFAULT));

    assertEquals(new SessionLink("https://f", "ident-1"), link);
    JsonNode body = json(transport.calls.get(0).body());
    assertFalse(body.has("save_option"));
    assertEquals("ID-9", body.get("id_number").asText());
  }

  @Test
  void createSessionLinkFailureIsReported() {
    FakeTransport transport =
        new FakeTransport()
            .reply(
                200,
                "{\"success\": false, \"errorCode\": 1, \"data\": \"Flow id: 1 is not active\"}");
    assertThrows(
        IOException.class,
        () ->
            client(transport)
                .createSessionLink(
                    "jwt",
                    new LinkRequest(
                        1, "ident-1", null, "https://r", Map.of(), SaveOption.DEFAULT)));
  }

  @Test
  void fetchSessionTokenUsesProcessId() throws IOException {
    FakeTransport transport =
        new FakeTransport().reply(200, "{\"token\": \"session-jwt\", \"processId\": \"proc-1\"}");

    assertEquals("session-jwt", client(transport).fetchSessionToken("jwt", "proc-1"));
    Call call = transport.calls.get(0);
    assertEquals("GET", call.method());
    assertEquals("https://btrust.example.com/api/v3/mobile_interaction/proc-1/token", call.url());
    assertEquals("Bearer jwt", call.headers().get("Authorization"));
  }

  @Test
  void fetchResultsUsesFastResultsEndpoint() throws IOException {
    FakeTransport transport =
        new FakeTransport().reply(200, "{\"success\": true, \"errorCode\": 0, \"data\": {}}");

    JsonNode results = client(transport).fetchResults("session-jwt");

    assertEquals(0, results.get("errorCode").asInt());
    Call call = transport.calls.get(0);
    assertEquals(
        "https://btrust.example.com/api/v3/mobile_interaction/v2/session-jwt/results_with_image_names",
        call.url());
    assertEquals("Bearer session-jwt", call.headers().get("Authorization"));
  }

  @Test
  void fetchResultsForProcessChainsTheWholeExchange() throws IOException {
    FakeTransport transport =
        new FakeTransport()
            .reply(200, "{\"access_token\": \"jwt\"}")
            .reply(200, "{\"token\": \"session-jwt\", \"processId\": \"proc-1\"}")
            .reply(200, "{\"success\": true, \"errorCode\": 0, \"data\": {\"success\": true}}");

    JsonNode results = client(transport).fetchResultsForProcess("proc-1");

    assertEquals(true, results.at("/data/success").asBoolean());
    assertEquals(3, transport.calls.size());
  }

  @Test
  void malformedJsonIsReportedAsIoError() {
    FakeTransport transport = new FakeTransport().reply(200, "<html>oops</html>");
    assertThrows(IOException.class, () -> client(transport).fetchResults("session-jwt"));
  }
}
