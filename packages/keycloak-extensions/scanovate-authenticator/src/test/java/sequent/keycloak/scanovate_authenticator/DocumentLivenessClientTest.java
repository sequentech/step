// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static sequent.keycloak.scanovate_authenticator.OnPremSessionsTest.JPEG;
import static sequent.keycloak.scanovate_authenticator.TestJson.json;

import com.fasterxml.jackson.databind.JsonNode;
import java.io.IOException;
import java.net.URI;
import java.util.ArrayList;
import java.util.Base64;
import java.util.List;
import org.junit.jupiter.api.Test;
import sequent.keycloak.scanovate_authenticator.DocumentLivenessClient.DocumentLivenessOutcome;
import sequent.keycloak.scanovate_authenticator.DocumentLivenessClient.DocumentLivenessResult;
import sequent.keycloak.scanovate_authenticator.DocumentLivenessClient.PipelineCheck;
import sequent.keycloak.scanovate_authenticator.FakeTransport.Call;

class DocumentLivenessClientTest {
  private static final List<String> PIPELINES = List.of("default-sr", "default-pc");

  private final List<Long> sleeps = new ArrayList<>();

  private DocumentLivenessClient client(FakeTransport transport) {
    return new DocumentLivenessClient(
        transport, URI.create("http://document-liveness:8080"), 3, sleeps::add);
  }

  /** A response of the service, as IDLive Doc Server 3.5.5 answers it. */
  static String response(String srStatus, double sr, double pc) {
    return """
        {"aggregate_image_quality_warnings": [], "aggregate_liveness_probability": %s,
         "pipeline_results": [
           {"pipeline_name": "default-pc", "calibration": "REGULAR",
            "result": {"image_quality_warnings": ["GLARE_ON_IMAGE"],
                       "liveness_probability": %s, "liveness_score": 1.2, "status_code": "OK"}},
           {"pipeline_name": "default-sr", "calibration": "REGULAR",
            "result": {"image_quality_warnings": [], "liveness_probability": %s,
                       "liveness_score": 6.1, "status_code": "%s"}}]}
        """
        .formatted(Math.min(sr, pc), pc, sr, srStatus);
  }

  @Test
  void sendsThePhotoWithEveryPipelineAndItsCalibration() throws IOException {
    FakeTransport transport = new FakeTransport().reply(200, response("OK", 0.99, 0.93));

    DocumentLivenessResult result = client(transport).check(JPEG, PIPELINES, "SOFT");

    Call call = transport.calls.get(0);
    assertEquals("http://document-liveness:8080/check_liveness", call.url());
    JsonNode body = json(call.body());
    assertEquals(Base64.getEncoder().encodeToString(JPEG), body.get("image").asText());
    assertEquals(
        json(
            "[{\"pipeline_name\":\"default-sr\",\"calibration\":\"SOFT\"},"
                + "{\"pipeline_name\":\"default-pc\",\"calibration\":\"SOFT\"}]"),
        body.get("pipelines"));
    assertEquals(json("[]"), body.get("errors_to_ignore"));
    assertEquals(
        List.of(
            new PipelineCheck("default-sr", "OK", 0.99, List.of()),
            new PipelineCheck("default-pc", "OK", 0.93, List.of("GLARE_ON_IMAGE"))),
        result.checks());
    assertEquals(DocumentLivenessOutcome.GENUINE, result.outcome(0.5));
    assertEquals(0.93, result.lowestProbability());
  }

  @Test
  void everyPipelineMustReachTheMinimum() throws IOException {
    FakeTransport transport = new FakeTransport().reply(200, response("OK", 0.99, 0.49));

    assertEquals(
        DocumentLivenessOutcome.NOT_GENUINE,
        client(transport).check(JPEG, PIPELINES, "REGULAR").outcome(0.5));
  }

  @Test
  void aValidationErrorIsNeverGenuine() throws IOException {
    FakeTransport transport =
        new FakeTransport().reply(200, response("DOCUMENT_TOO_CLOSE_TO_BORDER", 0.99, 0.99));

    assertEquals(
        DocumentLivenessOutcome.NOT_CHECKED,
        client(transport).check(JPEG, PIPELINES, "REGULAR").outcome(0.5));
  }

  @Test
  void serverErrorsAreRetriedWithBackoff() throws IOException {
    FakeTransport transport =
        new FakeTransport().fail("down").reply(503, "").reply(200, response("OK", 0.9, 0.9));

    client(transport).check(JPEG, PIPELINES, "REGULAR");
    assertEquals(List.of(1_000L, 2_000L), sleeps);
  }

  @Test
  void malformedResponsesAndMissingPipelinesAreErrors() {
    for (String body :
        List.of(
            "not json",
            "{}",
            "{\"pipeline_results\": {}}",
            "{\"pipeline_results\": [{\"pipeline_name\": \"default-sr\","
                + " \"result\": {\"status_code\": \"OK\", \"liveness_probability\": 0.9}}]}",
            "{\"pipeline_results\": [{\"pipeline_name\": \"default-sr\","
                + " \"result\": {\"status_code\": \"OK\"}},"
                + " {\"pipeline_name\": \"default-pc\","
                + " \"result\": {\"status_code\": \"OK\", \"liveness_probability\": 0.9}}]}")) {
      FakeTransport transport = new FakeTransport().reply(200, body);
      assertThrows(
          IOException.class, () -> client(transport).check(JPEG, PIPELINES, "REGULAR"), body);
    }
  }

  @Test
  void rejectedRequestsAreNotRetried() {
    FakeTransport transport = new FakeTransport().reply(400, "{}");

    assertThrows(IOException.class, () -> client(transport).check(JPEG, PIPELINES, "REGULAR"));
    assertEquals(List.of(), sleeps);
  }
}
