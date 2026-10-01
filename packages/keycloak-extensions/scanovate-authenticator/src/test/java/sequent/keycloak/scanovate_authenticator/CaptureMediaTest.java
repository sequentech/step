// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import org.junit.jupiter.api.Test;

class CaptureMediaTest {
  static final byte[] JPEG = {(byte) 0xFF, (byte) 0xD8, (byte) 0xFF, (byte) 0xE0, 0, 0x10};
  static final byte[] WEBM = {0x1A, 0x45, (byte) 0xDF, (byte) 0xA3, 0, 0};
  static final byte[] PNG = {(byte) 0x89, 'P', 'N', 'G', 0x0D, 0x0A};

  /** Parts uploaded by the capture page, by name. */
  static Map<String, byte[]> parts(byte[] front, byte[] back, byte[] holding) {
    Map<String, byte[]> parts = new HashMap<>();
    if (front != null) {
      parts.put("front", front);
    }
    if (back != null) {
      parts.put("back", back);
    }
    if (holding != null) {
      parts.put("holding", holding);
    }
    return parts;
  }

  private static CaptureSettings settings(List<DocumentSide> sides) {
    return new CaptureSettings(sides, 5, 16);
  }

  private static final CaptureSettings BOTH_SIDES =
      settings(List.of(DocumentSide.FRONT, DocumentSide.BACK));

  @Test
  void jpegIsDetectedFromItsMagicBytes() {
    assertEquals(Optional.of(MediaFormat.JPEG), MediaFormat.detect(JPEG));
    assertEquals(Optional.empty(), MediaFormat.detect(PNG));
    assertEquals(Optional.empty(), MediaFormat.detect(WEBM));
    assertEquals(Optional.empty(), MediaFormat.detect(new byte[] {-1, -40}));
    assertEquals(Optional.empty(), MediaFormat.detect(new byte[0]));
  }

  @Test
  void validCaptureIsAccepted() throws InvalidCaptureException {
    byte[] holding = {(byte) 0xFF, (byte) 0xD8, (byte) 0xFF, 9};
    CaptureMedia media = CaptureMedia.fromUploads(parts(JPEG, JPEG, holding), BOTH_SIDES);

    assertEquals(
        List.of(MediaKind.FRONT_IMAGE, MediaKind.BACK_IMAGE, MediaKind.HOLDING_IMAGE),
        List.copyOf(media.files().keySet()));
    assertArrayEquals(holding, media.files().get(MediaKind.HOLDING_IMAGE));
  }

  @Test
  void backIsOnlyTakenWhenTheDocumentHasOne() throws InvalidCaptureException {
    CaptureMedia media =
        CaptureMedia.fromUploads(parts(JPEG, JPEG, JPEG), settings(List.of(DocumentSide.FRONT)));

    assertEquals(
        List.of(MediaKind.FRONT_IMAGE, MediaKind.HOLDING_IMAGE),
        List.copyOf(media.files().keySet()));
  }

  @Test
  void missingPartsAreRejected() {
    assertThrows(
        InvalidCaptureException.class,
        () -> CaptureMedia.fromUploads(parts(JPEG, null, JPEG), BOTH_SIDES));
    assertThrows(
        InvalidCaptureException.class,
        () -> CaptureMedia.fromUploads(parts(JPEG, JPEG, null), BOTH_SIDES));
    assertThrows(
        InvalidCaptureException.class, () -> CaptureMedia.fromUploads(Map.of(), BOTH_SIDES));
  }

  @Test
  void wrongFormatsAreRejected() {
    assertThrows(
        InvalidCaptureException.class,
        () -> CaptureMedia.fromUploads(parts(PNG, JPEG, JPEG), BOTH_SIDES));
    assertThrows(
        InvalidCaptureException.class,
        () -> CaptureMedia.fromUploads(parts(JPEG, JPEG, WEBM), BOTH_SIDES));
    assertThrows(
        InvalidCaptureException.class,
        () -> CaptureMedia.fromUploads(parts(new byte[0], JPEG, JPEG), BOTH_SIDES));
  }

  @Test
  void oversizedPartsAreRejected() {
    byte[] bigJpeg = new byte[17];
    System.arraycopy(JPEG, 0, bigJpeg, 0, JPEG.length);
    byte[] exactJpeg = new byte[16];
    System.arraycopy(JPEG, 0, exactJpeg, 0, JPEG.length);

    assertThrows(
        InvalidCaptureException.class,
        () -> CaptureMedia.fromUploads(parts(bigJpeg, JPEG, JPEG), BOTH_SIDES));
    assertThrows(
        InvalidCaptureException.class,
        () -> CaptureMedia.fromUploads(parts(JPEG, JPEG, bigJpeg), BOTH_SIDES));
    assertEquals(
        16,
        assertDoesNotThrow(() -> CaptureMedia.fromUploads(parts(exactJpeg, JPEG, JPEG), BOTH_SIDES))
            .files()
            .get(MediaKind.FRONT_IMAGE)
            .length);
  }

  @Test
  void partsThatAreNotCapturedAreIgnored() throws InvalidCaptureException {
    Map<String, byte[]> parts = parts(JPEG, JPEG, JPEG);
    parts.put("face", JPEG);
    parts.put("video", WEBM);

    assertEquals(
        List.of(MediaKind.FRONT_IMAGE, MediaKind.BACK_IMAGE, MediaKind.HOLDING_IMAGE),
        List.copyOf(CaptureMedia.fromUploads(parts, BOTH_SIDES).files().keySet()));
  }
}
