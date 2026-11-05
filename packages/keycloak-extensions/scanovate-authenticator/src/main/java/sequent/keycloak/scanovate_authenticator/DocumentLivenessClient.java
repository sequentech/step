// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.io.IOException;
import java.net.URI;
import java.util.ArrayList;
import java.util.Base64;
import java.util.List;
import java.util.Map;
import sequent.keycloak.authenticator.diagnostics.ScanovateDiagnostics.ServiceError;

/**
 * Client for the on-premise Scanovate Document Liveness service, which checks that a photo shows a
 * physical document: not a screen (screen replay), a printed copy, a substituted portrait or a
 * digitally manipulated image, each check a pipeline of the service. It keeps nothing.
 */
public class DocumentLivenessClient {
  static final String CHECK_PATH = "/check_liveness";
  static final String STATUS_OK = "OK";

  /** Whether the photo shows a physical document. */
  public enum DocumentLivenessOutcome {
    GENUINE,
    NOT_GENUINE,
    /**
     * A pipeline could not check the photo: no document, cropped, too small, too close, several
     * documents, or no portrait.
     */
    NOT_CHECKED
  }

  /**
   * The result of one pipeline.
   *
   * @param pipeline the pipeline name, as requested
   * @param status {@value #STATUS_OK} when the pipeline checked the photo, otherwise why not
   * @param probability from 0 to 1, that the document is physical
   * @param warnings image quality warnings, which make the result less reliable
   */
  public record PipelineCheck(
      String pipeline, String status, double probability, List<String> warnings) {}

  /**
   * Response of {@code POST /check_liveness}.
   *
   * @param checks the result of each requested pipeline
   */
  public record DocumentLivenessResult(List<PipelineCheck> checks) {
    /**
     * The document is genuine only if every pipeline checked the photo and reached the minimum
     * probability.
     */
    public DocumentLivenessOutcome outcome(double minProbability) {
      if (checks.stream().anyMatch(check -> !STATUS_OK.equals(check.status()))) {
        return DocumentLivenessOutcome.NOT_CHECKED;
      }
      return checks.stream().allMatch(check -> check.probability() >= minProbability)
          ? DocumentLivenessOutcome.GENUINE
          : DocumentLivenessOutcome.NOT_GENUINE;
    }

    /** The lowest probability of the pipelines, which decides the outcome. */
    public double lowestProbability() {
      return checks.stream().mapToDouble(PipelineCheck::probability).min().orElse(0);
    }
  }

  private static final ObjectMapper MAPPER = new ObjectMapper();

  private final HttpTransport transport;
  private final URI baseUrl;
  private final RetryingRequests requests;

  public DocumentLivenessClient(
      HttpTransport transport, URI baseUrl, int maxRetries, Sleeper sleeper) {
    this.transport = transport;
    this.baseUrl = baseUrl;
    this.requests = new RetryingRequests(maxRetries, sleeper);
  }

  /**
   * Checks a JPEG photo of the document with the given pipelines, which the service must have
   * enabled ({@code IDLIVEDOC_SERVER_AVAILABLE_PIPELINES}): it rejects any other. No validation
   * error is ignored.
   *
   * @param calibration REGULAR, SOFT (fewer false rejections) or HARD (fewer missed attacks)
   * @throws IOException if the service is unreachable, answers with an error, or its body is
   *     malformed or lacks a requested pipeline
   */
  public DocumentLivenessResult check(byte[] image, List<String> pipelines, String calibration)
      throws IOException {
    ObjectNode body = MAPPER.createObjectNode();
    body.put("image", Base64.getEncoder().encodeToString(image));
    ArrayNode entries = body.putArray("pipelines");
    pipelines.forEach(
        pipeline ->
            entries.addObject().put("pipeline_name", pipeline).put("calibration", calibration));
    body.putArray("errors_to_ignore");
    String payload = MAPPER.writeValueAsString(body);
    JsonNode response =
        requests.execute(
            CHECK_PATH, () -> transport.postJson(baseUrl + CHECK_PATH, Map.of(), payload));
    JsonNode results = response.get("pipeline_results");
    if (results == null || !results.isArray()) {
      throw new ServiceFailure(ServiceError.MALFORMED_RESPONSE);
    }
    List<PipelineCheck> checks = new ArrayList<>();
    for (String pipeline : pipelines) {
      checks.add(pipelineCheck(results, pipeline));
    }
    return new DocumentLivenessResult(List.copyOf(checks));
  }

  private static PipelineCheck pipelineCheck(JsonNode results, String pipeline) throws IOException {
    for (JsonNode result : results) {
      if (!pipeline.equals(result.path("pipeline_name").asText())) {
        continue;
      }
      JsonNode status = result.path("result").get("status_code");
      JsonNode probability = result.path("result").get("liveness_probability");
      if (status == null || !status.isTextual()) {
        throw new ServiceFailure(ServiceError.MALFORMED_RESPONSE);
      }
      List<String> warnings = new ArrayList<>();
      result.path("result").path("image_quality_warnings").forEach(w -> warnings.add(w.asText()));
      if (!STATUS_OK.equals(status.asText())) {
        return new PipelineCheck(pipeline, status.asText(), 0, List.copyOf(warnings));
      }
      if (probability == null || !probability.isNumber()) {
        throw new ServiceFailure(ServiceError.MALFORMED_RESPONSE);
      }
      return new PipelineCheck(pipeline, STATUS_OK, probability.asDouble(), List.copyOf(warnings));
    }
    throw new ServiceFailure(ServiceError.MALFORMED_RESPONSE);
  }
}
