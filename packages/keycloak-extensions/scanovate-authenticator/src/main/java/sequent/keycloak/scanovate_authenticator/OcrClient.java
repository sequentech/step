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

/**
 * Client for the on-premise Scanovate OCR service, which reads the fields of a photo of an identity
 * document and checks them, without storing the photo.
 */
public class OcrClient {
  static final String SINGLE_IMAGE_PATH = "/single_image_ocr";

  private static final ObjectMapper MAPPER = new ObjectMapper();

  private final HttpTransport transport;
  private final URI baseUrl;
  private final RetryingRequests requests;

  public OcrClient(HttpTransport transport, URI baseUrl, int maxRetries, Sleeper sleeper) {
    this.transport = transport;
    this.baseUrl = baseUrl;
    this.requests = new RetryingRequests(maxRetries, sleeper);
  }

  /**
   * Reads a JPEG photo of a document side, see {@link OcrResults} for its response.
   *
   * @param ocrType how the service reads the document, e.g. {@code passport}
   * @param requestId identifies the request in the service's logs
   * @throws IOException if the service is unreachable or answers with an error or a malformed body
   */
  public JsonNode recognize(String ocrType, byte[] image, String requestId) throws IOException {
    ObjectNode body = MAPPER.createObjectNode();
    body.put("ocr_type", ocrType);
    body.put("image_base64", Base64.getEncoder().encodeToString(image));
    body.put("request_id", requestId);
    String payload = MAPPER.writeValueAsString(body);
    JsonNode response =
        requests.execute(
            SINGLE_IMAGE_PATH,
            () -> transport.postJson(baseUrl + SINGLE_IMAGE_PATH, Map.of(), payload));
    if (response == null || !response.isObject()) {
      throw new IOException(SINGLE_IMAGE_PATH + " response is not an object");
    }
    return response;
  }
}
