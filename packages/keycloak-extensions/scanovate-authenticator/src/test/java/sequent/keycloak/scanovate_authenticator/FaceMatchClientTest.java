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
import sequent.keycloak.scanovate_authenticator.FaceMatchClient.FaceComparison;
import sequent.keycloak.scanovate_authenticator.FaceMatchClient.FaceMatchOutcome;
import sequent.keycloak.scanovate_authenticator.FakeTransport.Call;

class FaceMatchClientTest {
  private static final byte[] OTHER_JPEG = {(byte) 0xFF, (byte) 0xD8, (byte) 0xFF, 1, 2, 3};

  private final List<Long> sleeps = new ArrayList<>();

  private FaceMatchClient client(FakeTransport transport) {
    return new FaceMatchClient(transport, URI.create("http://face-match:3000"), 3, sleeps::add);
  }

  static String response(boolean success, double similarity, int status1, int status2) {
    return """
        {"success": %s, "similarity": %s, "threshold": 0.67,
         "image_1_status": {"code": %d, "message": "m"},
         "image_2_status": {"code": %d, "message": "m"}}
        """
        .formatted(success, similarity, status1, status2);
  }

  @Test
  void comparesBothImagesAllowingSeveralFaces() throws IOException {
    FakeTransport transport = new FakeTransport().reply(200, response(true, 0.9, 0, 0));

    FaceComparison comparison = client(transport).compare(JPEG, OTHER_JPEG);

    Call call = transport.calls.get(0);
    assertEquals("http://face-match:3000/facematch11/compare_images", call.url());
    JsonNode body = json(call.body());
    assertEquals(Base64.getEncoder().encodeToString(JPEG), body.get("image_1_base64").asText());
    assertEquals(
        Base64.getEncoder().encodeToString(OTHER_JPEG), body.get("image_2_base64").asText());
    assertEquals(true, body.get("allow_multiple_faces").asBoolean());
    assertEquals(new FaceComparison(true, 0.9, 0.67, 0, 0), comparison);
  }

  @Test
  void matchesWhenTheSimilarityReachesBothThresholds() {
    assertEquals(FaceMatchOutcome.MATCH, new FaceComparison(true, 0.8, 0.67, 0, 0).outcome(0.8));
    assertEquals(
        FaceMatchOutcome.MISMATCH, new FaceComparison(true, 0.79, 0.67, 0, 0).outcome(0.8));
  }

  @Test
  void theServiceThresholdAppliesWhenItIsStricter() {
    assertEquals(FaceMatchOutcome.MISMATCH, new FaceComparison(true, 0.6, 0.67, 0, 0).outcome(0.5));
  }

  @Test
  void unsuccessfulComparisonsDoNotMatch() {
    assertEquals(
        FaceMatchOutcome.MISMATCH, new FaceComparison(false, 0.99, 0.67, 0, 0).outcome(0.5));
  }

  @Test
  void imagesWithoutAUsableFaceAreReported() {
    for (int code : List.of(1000, 1001, 1100, 1101, 1102, 1103)) {
      assertEquals(
          FaceMatchOutcome.FACE_NOT_FOUND,
          new FaceComparison(false, 0.0, 0.67, code, 0).outcome(0.5),
          String.valueOf(code));
      assertEquals(
          FaceMatchOutcome.FACE_NOT_FOUND,
          new FaceComparison(false, 0.0, 0.67, 0, code).outcome(0.5),
          String.valueOf(code));
    }
  }

  @Test
  void serverErrorsAreRetriedWithBackoff() throws IOException {
    FakeTransport transport =
        new FakeTransport().fail("down").reply(503, "").reply(200, response(true, 0.9, 0, 0));

    assertEquals(0.9, client(transport).compare(JPEG, JPEG).similarity());
    assertEquals(List.of(1_000L, 2_000L), sleeps);
  }

  @Test
  void rejectedRequestsAreNotRetried() {
    FakeTransport transport = new FakeTransport().reply(422, "{\"detail\": []}");

    assertThrows(IOException.class, () -> client(transport).compare(JPEG, JPEG));
    assertEquals(List.of(), sleeps);
  }

  @Test
  void malformedResponsesAreErrors() {
    for (String body :
        List.of(
            "not json",
            "{}",
            "{\"success\": true, \"similarity\": 0.9, \"threshold\": 0.67}",
            "{\"success\": true, \"similarity\": \"high\", \"threshold\": 0.67,"
                + " \"image_1_status\": {\"code\": 0}, \"image_2_status\": {\"code\": 0}}")) {
      FakeTransport transport = new FakeTransport().reply(200, body);
      assertThrows(IOException.class, () -> client(transport).compare(JPEG, JPEG), body);
    }
  }

  @Test
  void unreachableServiceFailsAfterTheRetries() {
    FakeTransport transport = new FakeTransport().fail("a").fail("b").fail("c");

    assertThrows(IOException.class, () -> client(transport).compare(JPEG, JPEG));
    assertEquals(3, transport.calls.size());
  }
}
