// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.io.IOException;
import java.net.URI;
import java.util.Base64;
import java.util.Map;
import sequent.keycloak.scanovate_authenticator.ScanovateClient.Sleeper;

/**
 * Client for the 1:1 API of the on-premise Scanovate Face Match service, which compares the faces
 * of two images without storing them.
 */
public class FaceMatchClient {
  static final String COMPARE_PATH = "/facematch11/compare_images";
  static final int STATUS_OK = 0;

  /** Whether the faces of a comparison belong to the same person. */
  public enum FaceMatchOutcome {
    MATCH,
    MISMATCH,
    /** An image has no usable face: unreadable, too small, no face, or it could not be aligned. */
    FACE_NOT_FOUND
  }

  /**
   * Response of {@code POST /facematch11/compare_images}.
   *
   * @param success whether the comparison ran to completion
   * @param similarity from 0 to 1
   * @param threshold the service's own cutoff for a match
   * @param image1Status status code of the first image, 0 when its face was used
   * @param image2Status status code of the second image, 0 when its face was used
   */
  public record FaceComparison(
      boolean success, double similarity, double threshold, int image1Status, int image2Status) {
    /**
     * The faces match only if the comparison succeeded on a face of each image, and the similarity
     * reaches both the given minimum and the service's threshold, whichever is stricter.
     */
    public FaceMatchOutcome outcome(double minSimilarity) {
      if (image1Status != STATUS_OK || image2Status != STATUS_OK) {
        return FaceMatchOutcome.FACE_NOT_FOUND;
      }
      return success && similarity >= Math.max(minSimilarity, threshold)
          ? FaceMatchOutcome.MATCH
          : FaceMatchOutcome.MISMATCH;
    }
  }

  private static final ObjectMapper MAPPER = new ObjectMapper();

  private final HttpTransport transport;
  private final URI baseUrl;
  private final RetryingRequests requests;

  public FaceMatchClient(HttpTransport transport, URI baseUrl, int maxRetries, Sleeper sleeper) {
    this.transport = transport;
    this.baseUrl = baseUrl;
    this.requests = new RetryingRequests(maxRetries, sleeper);
  }

  /**
   * Compares the faces of two JPEG images. Images may show several faces, such as the ghost
   * portrait of an ID or the ID held next to the voter's face: the best matching faces are
   * compared.
   *
   * @throws IOException if the service is unreachable or answers with an error or a malformed body
   */
  public FaceComparison compare(byte[] image1, byte[] image2) throws IOException {
    ObjectNode body = MAPPER.createObjectNode();
    body.put("image_1_base64", Base64.getEncoder().encodeToString(image1));
    body.put("image_2_base64", Base64.getEncoder().encodeToString(image2));
    body.put("allow_multiple_faces", true);
    String payload = MAPPER.writeValueAsString(body);
    JsonNode response =
        requests.execute(
            COMPARE_PATH, () -> transport.postJson(baseUrl + COMPARE_PATH, Map.of(), payload));
    return new FaceComparison(
        requiredBoolean(response, "success"),
        requiredNumber(response.get("similarity"), "similarity"),
        requiredNumber(response.get("threshold"), "threshold"),
        statusCode(response, "image_1_status"),
        statusCode(response, "image_2_status"));
  }

  private static boolean requiredBoolean(JsonNode response, String field) throws IOException {
    JsonNode value = response.get(field);
    if (value == null || !value.isBoolean()) {
      throw new IOException(COMPARE_PATH + " response is missing " + field);
    }
    return value.asBoolean();
  }

  private static double requiredNumber(JsonNode value, String field) throws IOException {
    if (value == null || !value.isNumber()) {
      throw new IOException(COMPARE_PATH + " response is missing " + field);
    }
    return value.asDouble();
  }

  private static int statusCode(JsonNode response, String field) throws IOException {
    return (int) requiredNumber(response.path(field).get("code"), field + ".code");
  }
}
