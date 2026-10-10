// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static sequent.keycloak.scanovate_authenticator.CaptureMediaTest.JPEG;
import static sequent.keycloak.scanovate_authenticator.TestJson.json;

import com.fasterxml.jackson.databind.JsonNode;
import java.io.IOException;
import java.net.URI;
import java.util.ArrayList;
import java.util.Base64;
import java.util.List;
import org.junit.jupiter.api.Test;
import sequent.keycloak.scanovate_authenticator.FakeTransport.Call;

class OcrClientTest {
  private final List<Long> sleeps = new ArrayList<>();

  private OcrClient client(FakeTransport transport) {
    return new OcrClient(transport, URI.create("http://ocr:5040"), 3, sleeps::add);
  }

  @Test
  void sendsTheImageWithItsOcrTypeAndRequestId() throws IOException {
    FakeTransport transport = new FakeTransport().reply(200, "{\"status\": \"completed\"}");

    JsonNode response = client(transport).recognize("passport", JPEG, "case-1-front");

    Call call = transport.calls.get(0);
    assertEquals("http://ocr:5040/single_image_ocr", call.url());
    JsonNode body = json(call.body());
    assertEquals("passport", body.get("ocr_type").asText());
    assertEquals(Base64.getEncoder().encodeToString(JPEG), body.get("image_base64").asText());
    assertEquals("case-1-front", body.get("request_id").asText());
    assertEquals("completed", response.get("status").asText());
  }

  @Test
  void serverErrorsAreRetriedWithBackoff() throws IOException {
    FakeTransport transport =
        new FakeTransport().fail("down").reply(503, "").reply(200, "{\"status\": \"completed\"}");

    client(transport).recognize("passport", JPEG, "r");
    assertEquals(List.of(1_000L, 2_000L), sleeps);
  }

  @Test
  void rejectedRequestsAreNotRetried() {
    FakeTransport transport = new FakeTransport().reply(422, "{\"detail\": []}");

    assertThrows(IOException.class, () -> client(transport).recognize("passport", JPEG, "r"));
    assertEquals(List.of(), sleeps);
  }

  @Test
  void responsesThatAreNotAnObjectAreErrors() {
    for (String body : List.of("not json", "[]", "\"completed\"")) {
      FakeTransport transport = new FakeTransport().reply(200, body);
      assertThrows(
          IOException.class, () -> client(transport).recognize("passport", JPEG, "r"), body);
    }
  }

  @Test
  void unreachableServiceFailsAfterTheRetries() {
    FakeTransport transport = new FakeTransport().fail("a").fail("b").fail("c");

    assertThrows(IOException.class, () -> client(transport).recognize("passport", JPEG, "r"));
    assertEquals(3, transport.calls.size());
  }
}
