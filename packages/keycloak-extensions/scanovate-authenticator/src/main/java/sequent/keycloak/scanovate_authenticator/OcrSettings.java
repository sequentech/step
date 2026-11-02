// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.IOException;
import java.net.URI;
import java.util.Map;

/**
 * How a document type is read by the on-premise Scanovate OCR service.
 *
 * @param url internal base URL of the OCR service, only reached by Keycloak
 * @param ocrType how the service reads the document type, e.g. {@code passport}
 */
public record OcrSettings(URI url, String ocrType) {
  static final String DEFAULT_OCR_TYPE = "passport";

  private static final ObjectMapper MAPPER = new ObjectMapper();

  /**
   * Reads the settings for the given document type from the authenticator configuration.
   *
   * <p>The OCR types are a JSON object keyed by document type, with a {@code default} key for any
   * other document type. Without configuration, every document is read as a passport.
   *
   * @throws ScanovateException if the URL is missing, the document type has no OCR type, or a
   *     setting is malformed
   */
  public static OcrSettings fromConfig(Map<String, String> config, String docType)
      throws ScanovateException {
    return new OcrSettings(
        ServiceUrls.parse(config, ScanovateAuthenticatorFactory.OCR_URL),
        ocrType(config.get(ScanovateAuthenticatorFactory.OCR_TYPES), docType));
  }

  /** The physical format of the documents this OCR type reads. */
  public DocumentFormat format() {
    return DocumentFormat.ofOcrType(ocrType);
  }

  private static String ocrType(String configuration, String docType) throws ScanovateException {
    if (configuration == null || configuration.isBlank()) {
      return DEFAULT_OCR_TYPE;
    }
    JsonNode root;
    try {
      root = MAPPER.readTree(configuration);
    } catch (IOException e) {
      throw new ScanovateException("Invalid OCR types configuration", e);
    }
    if (root == null || !root.isObject()) {
      throw new ScanovateException("OCR types configuration must be an object");
    }
    for (Map.Entry<String, JsonNode> entry : root.properties()) {
      if (!entry.getValue().isTextual() || entry.getValue().asText().isBlank()) {
        throw new ScanovateException("OCR type for " + entry.getKey() + " must be a name");
      }
    }
    JsonNode entry =
        docType != null && root.has(docType)
            ? root.get(docType)
            : root.get(AttributeRules.DEFAULT_DOC_TYPE);
    if (entry == null) {
      throw new ScanovateException("No OCR type for document type " + docType);
    }
    return entry.asText().trim();
  }
}
