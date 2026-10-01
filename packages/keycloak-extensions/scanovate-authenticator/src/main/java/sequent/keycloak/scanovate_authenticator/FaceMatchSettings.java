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
 * How faces are compared with the on-premise Scanovate Face Match service, for the {@link
 * FaceCapture#LIVENESS} face capture.
 *
 * @param url internal base URL of the Face Match service, only reached by Keycloak
 * @param minSimilarity minimum similarity, from 0 to 1, for the faces of the document type to match
 */
public record FaceMatchSettings(URI url, double minSimilarity) {
  static final double DEFAULT_MIN_SIMILARITY = 0.67;

  private static final ObjectMapper MAPPER = new ObjectMapper();

  /**
   * Reads the settings for the given document type from the authenticator configuration.
   *
   * <p>The minimum similarity is a JSON object keyed by document type, with a {@code default} key
   * for any other document type. Without a matching entry, {@link #DEFAULT_MIN_SIMILARITY} applies.
   *
   * @throws ScanovateException if the URL is missing, or a setting is malformed
   */
  public static FaceMatchSettings fromConfig(Map<String, String> config, String docType)
      throws ScanovateException {
    return new FaceMatchSettings(
        ServiceUrls.parse(config, ScanovateAuthenticatorFactory.FACE_MATCH_URL),
        minSimilarity(
            config.get(ScanovateAuthenticatorFactory.FACE_MATCH_MIN_SIMILARITY), docType));
  }

  private static double minSimilarity(String configuration, String docType)
      throws ScanovateException {
    if (configuration == null || configuration.isBlank()) {
      return DEFAULT_MIN_SIMILARITY;
    }
    JsonNode root;
    try {
      root = MAPPER.readTree(configuration);
    } catch (IOException e) {
      throw new ScanovateException("Invalid face match minimum similarity configuration", e);
    }
    if (root == null || !root.isObject()) {
      throw new ScanovateException("Face match minimum similarity configuration must be an object");
    }
    for (Map.Entry<String, JsonNode> entry : root.properties()) {
      JsonNode value = entry.getValue();
      if (!value.isNumber() || value.asDouble() <= 0 || value.asDouble() > 1) {
        throw new ScanovateException(
            "Face match minimum similarity for " + entry.getKey() + " must be in (0, 1]");
      }
    }
    JsonNode entry =
        docType != null && root.has(docType)
            ? root.get(docType)
            : root.get(AttributeRules.DEFAULT_DOC_TYPE);
    return entry == null ? DEFAULT_MIN_SIMILARITY : entry.asDouble();
  }
}
