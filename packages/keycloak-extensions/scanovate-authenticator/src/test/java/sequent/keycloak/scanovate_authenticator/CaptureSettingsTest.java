// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.HashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

class CaptureSettingsTest {
  private static final String SIDES =
      "{\"philSysID\": [\"front\"], \"default\": [\"front\", \"back\"]}";

  @Test
  void unsetSidesRequireBothSides() throws ScanovateException {
    assertEquals(
        List.of(DocumentSide.FRONT, DocumentSide.BACK),
        CaptureSettings.parseSides(null, "philSysID"));
    assertEquals(
        List.of(DocumentSide.FRONT, DocumentSide.BACK),
        CaptureSettings.parseSides("  ", "philSysID"));
  }

  @Test
  void sidesArePickedByDocumentType() throws ScanovateException {
    assertEquals(List.of(DocumentSide.FRONT), CaptureSettings.parseSides(SIDES, "philSysID"));
    assertEquals(
        List.of(DocumentSide.FRONT, DocumentSide.BACK),
        CaptureSettings.parseSides(SIDES, "philippinePassport"));
    assertEquals(
        List.of(DocumentSide.FRONT, DocumentSide.BACK), CaptureSettings.parseSides(SIDES, null));
  }

  @Test
  void sidesFallBackToBothSidesWithoutDefaultEntry() throws ScanovateException {
    assertEquals(
        List.of(DocumentSide.FRONT, DocumentSide.BACK),
        CaptureSettings.parseSides("{\"philSysID\": [\"front\"]}", "driversLicense"));
  }

  @Test
  void sidesAreOrderedFrontFirst() throws ScanovateException {
    assertEquals(
        List.of(DocumentSide.FRONT, DocumentSide.BACK),
        CaptureSettings.parseSides("{\"default\": [\"back\", \"front\"]}", "x"));
  }

  @Test
  void malformedSidesAreRejected() {
    for (String configuration :
        List.of(
            "not json",
            "[\"front\"]",
            "{\"default\": \"front\"}",
            "{\"default\": []}",
            "{\"default\": [\"front\", \"side\"]}",
            "{\"default\": [\"front\", \"front\"]}",
            "{\"default\": [\"back\"]}",
            "{\"default\": [1]}")) {
      assertThrows(
          ScanovateException.class,
          () -> CaptureSettings.parseSides(configuration, "x"),
          configuration);
    }
  }

  @Test
  void defaultsApplyWhenUnset() throws ScanovateException {
    CaptureSettings settings = CaptureSettings.fromConfig(Map.of(), "philSysID");

    assertEquals(List.of(DocumentSide.FRONT, DocumentSide.BACK), settings.sides());
    assertEquals(ScanovateAuthenticatorFactory.DEFAULT_VIDEO_SECONDS, settings.videoSeconds());
    assertEquals(2 * 1024 * 1024, settings.maxBytes(MediaCategory.IMAGE));
    assertEquals(3 * 1024 * 1024, settings.maxBytes(MediaCategory.VIDEO));
    assertEquals(
        List.of(
            MediaKind.FRONT_IMAGE,
            MediaKind.BACK_IMAGE,
            MediaKind.FACE_IMAGE,
            MediaKind.SCAN_VIDEO),
        settings.requiredMedia());
  }

  @Test
  void configuredValuesAreRead() throws ScanovateException {
    Map<String, String> config = new HashMap<>();
    config.put(ScanovateAuthenticatorFactory.CAPTURE_SIDES, SIDES);
    config.put(ScanovateAuthenticatorFactory.VIDEO_SECONDS, " 8 ");
    config.put(ScanovateAuthenticatorFactory.MAX_IMAGE_BYTES, "1000");
    config.put(ScanovateAuthenticatorFactory.MAX_VIDEO_BYTES, "2000");

    CaptureSettings settings = CaptureSettings.fromConfig(config, "philSysID");

    assertEquals(List.of(DocumentSide.FRONT), settings.sides());
    assertEquals(8, settings.videoSeconds());
    assertEquals(1000, settings.maxBytes(MediaCategory.IMAGE));
    assertEquals(2000, settings.maxBytes(MediaCategory.VIDEO));
    assertEquals(
        List.of(MediaKind.FRONT_IMAGE, MediaKind.FACE_IMAGE, MediaKind.SCAN_VIDEO),
        settings.requiredMedia());
  }

  @Test
  void facePhotoIsCapturedByDefault() throws ScanovateException {
    assertEquals(FaceCapture.PHOTO, CaptureSettings.fromConfig(Map.of(), "x").faceCapture());
  }

  @Test
  void livenessNeedsTheDocumentSidesAndThePhotoHoldingTheDocument() throws ScanovateException {
    Map<String, String> config = new HashMap<>();
    config.put(ScanovateAuthenticatorFactory.CAPTURE_SIDES, SIDES);
    config.put(ScanovateAuthenticatorFactory.FACE_CAPTURE, FaceCapture.LIVENESS.value());

    assertEquals(
        List.of(MediaKind.FRONT_IMAGE, MediaKind.HOLDING_IMAGE),
        CaptureSettings.fromConfig(config, "philSysID").requiredMedia());
    assertEquals(
        List.of(MediaKind.FRONT_IMAGE, MediaKind.BACK_IMAGE, MediaKind.HOLDING_IMAGE),
        CaptureSettings.fromConfig(config, "driversLicense").requiredMedia());
  }

  @Test
  void livenessOnlyUploadsTheDocumentSides() throws ScanovateException {
    Map<String, String> config = new HashMap<>();
    config.put(ScanovateAuthenticatorFactory.CAPTURE_SIDES, SIDES);
    config.put(ScanovateAuthenticatorFactory.FACE_CAPTURE, FaceCapture.LIVENESS.value());

    assertEquals(
        List.of(MediaKind.FRONT_IMAGE),
        CaptureSettings.fromConfig(config, "philSysID").uploadedMedia());
    assertEquals(
        List.of(MediaKind.FRONT_IMAGE, MediaKind.BACK_IMAGE),
        CaptureSettings.fromConfig(config, "driversLicense").uploadedMedia());
  }

  @Test
  void photoUploadsEverythingItCaptures() throws ScanovateException {
    CaptureSettings settings = CaptureSettings.fromConfig(Map.of(), "x");

    assertEquals(settings.requiredMedia(), settings.uploadedMedia());
  }

  @Test
  void unknownFaceCaptureIsRejected() {
    assertThrows(
        ScanovateException.class,
        () ->
            CaptureSettings.fromConfig(
                Map.of(ScanovateAuthenticatorFactory.FACE_CAPTURE, "selfie"), "x"));
  }

  @Test
  void invalidNumbersAreRejected() {
    for (String key :
        List.of(
            ScanovateAuthenticatorFactory.VIDEO_SECONDS,
            ScanovateAuthenticatorFactory.MAX_IMAGE_BYTES,
            ScanovateAuthenticatorFactory.MAX_VIDEO_BYTES)) {
      for (String value : List.of("five", "0", "-1", "2147483647")) {
        assertThrows(
            ScanovateException.class,
            () -> CaptureSettings.fromConfig(Map.of(key, value), "x"),
            key + "=" + value);
      }
    }
  }

  @Test
  void defaultCaptureFitsTheDefaultKeycloakBodyLimit() throws ScanovateException {
    CaptureSettings settings = CaptureSettings.fromConfig(Map.of(), "x");
    long worstCase =
        settings.requiredMedia().stream()
            .mapToLong(kind -> settings.maxBytes(kind.category()))
            .sum();
    // quarkus.http.limits.max-body-size defaults to 10240K
    assertTrue(worstCase < 10240L * 1024, String.valueOf(worstCase));
  }
}
