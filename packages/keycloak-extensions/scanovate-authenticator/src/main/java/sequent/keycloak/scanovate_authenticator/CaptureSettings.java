// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.IOException;
import java.util.ArrayList;
import java.util.EnumSet;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;

/**
 * What the voter captures in the embedded mode for a document type, and the limits applied to it.
 *
 * @param sides document sides to capture, front first
 * @param videoSeconds length of the video holding the document
 * @param maxImageBytes size limit of each image
 * @param maxVideoBytes size limit of the video
 * @param faceCapture how the voter's face is captured
 */
public record CaptureSettings(
    List<DocumentSide> sides,
    int videoSeconds,
    int maxImageBytes,
    int maxVideoBytes,
    FaceCapture faceCapture) {
  static final List<DocumentSide> DEFAULT_SIDES = List.of(DocumentSide.FRONT, DocumentSide.BACK);

  private static final ObjectMapper MAPPER = new ObjectMapper();

  public CaptureSettings {
    sides = List.copyOf(sides);
  }

  /**
   * Reads the settings for the given document type from the authenticator configuration.
   *
   * @throws ScanovateException if a setting is malformed
   */
  public static CaptureSettings fromConfig(Map<String, String> config, String docType)
      throws ScanovateException {
    return new CaptureSettings(
        parseSides(config.get(ScanovateAuthenticatorFactory.CAPTURE_SIDES), docType),
        positiveInt(
            config,
            ScanovateAuthenticatorFactory.VIDEO_SECONDS,
            ScanovateAuthenticatorFactory.DEFAULT_VIDEO_SECONDS),
        positiveInt(
            config,
            ScanovateAuthenticatorFactory.MAX_IMAGE_BYTES,
            ScanovateAuthenticatorFactory.DEFAULT_MAX_IMAGE_BYTES),
        positiveInt(
            config,
            ScanovateAuthenticatorFactory.MAX_VIDEO_BYTES,
            ScanovateAuthenticatorFactory.DEFAULT_MAX_VIDEO_BYTES),
        faceCapture(config));
  }

  /**
   * Reads the face capture from the authenticator configuration, photo when unset.
   *
   * @throws ScanovateException if the face capture is unknown
   */
  public static FaceCapture faceCapture(Map<String, String> config) throws ScanovateException {
    String value = config.get(ScanovateAuthenticatorFactory.FACE_CAPTURE);
    if (value == null || value.isBlank()) {
      return FaceCapture.PHOTO;
    }
    return FaceCapture.fromValue(value.trim())
        .orElseThrow(() -> new ScanovateException("Invalid face capture: " + value));
  }

  /**
   * Returns the sides to capture for the given document type.
   *
   * <p>The configuration is a JSON object keyed by document type, with a {@code default} key for
   * any other document type. Without a matching entry, both sides are captured.
   *
   * @throws ScanovateException if the configuration is malformed
   */
  public static List<DocumentSide> parseSides(String configuration, String docType)
      throws ScanovateException {
    if (configuration == null || configuration.isBlank()) {
      return DEFAULT_SIDES;
    }
    JsonNode root;
    try {
      root = MAPPER.readTree(configuration);
    } catch (IOException e) {
      throw new ScanovateException("Invalid capture sides configuration", e);
    }
    if (root == null || !root.isObject()) {
      throw new ScanovateException("Capture sides configuration must be an object");
    }
    JsonNode entry =
        docType != null && root.has(docType)
            ? root.get(docType)
            : root.get(AttributeRules.DEFAULT_DOC_TYPE);
    if (entry == null) {
      return DEFAULT_SIDES;
    }
    if (!entry.isArray() || entry.isEmpty()) {
      throw new ScanovateException("Capture sides for " + docType + " must be a non-empty list");
    }
    Set<DocumentSide> sides = EnumSet.noneOf(DocumentSide.class);
    for (JsonNode value : entry) {
      Optional<DocumentSide> side =
          value.isTextual() ? DocumentSide.fromValue(value.asText()) : Optional.empty();
      if (side.isEmpty() || !sides.add(side.get())) {
        throw new ScanovateException("Invalid or repeated capture side " + value);
      }
    }
    if (!sides.contains(DocumentSide.FRONT)) {
      throw new ScanovateException("Capture sides for " + docType + " must include the front");
    }
    return List.copyOf(sides);
  }

  /**
   * Files that the capture page must post, in upload order. With the liveness face capture, the
   * face photo comes from Liveness Plus instead.
   */
  public List<MediaKind> requiredMedia() {
    List<MediaKind> media = new ArrayList<>();
    sides.forEach(side -> media.add(side.mediaKind()));
    if (faceCapture == FaceCapture.PHOTO) {
      media.add(MediaKind.FACE_IMAGE);
      media.add(MediaKind.SCAN_VIDEO);
    }
    return media;
  }

  public int maxBytes(MediaCategory category) {
    return switch (category) {
      case IMAGE -> maxImageBytes;
      case VIDEO -> maxVideoBytes;
    };
  }

  static int positiveInt(Map<String, String> config, String key, int defaultValue)
      throws ScanovateException {
    String value = config.get(key);
    if (value == null || value.isBlank()) {
      return defaultValue;
    }
    int parsed;
    try {
      parsed = Integer.parseInt(value.trim());
    } catch (NumberFormatException e) {
      throw new ScanovateException("Invalid " + key + ": " + value, e);
    }
    // The byte limits are read with one extra byte to detect oversized files
    if (parsed <= 0 || parsed == Integer.MAX_VALUE) {
      throw new ScanovateException("Invalid " + key + ": " + value);
    }
    return parsed;
  }
}
