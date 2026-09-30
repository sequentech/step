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
  static final byte[] MP4 = {0, 0, 0, 0x18, 'f', 't', 'y', 'p', 'm', 'p', '4', '2'};
  static final byte[] PNG = {(byte) 0x89, 'P', 'N', 'G', 0x0D, 0x0A};

  /** Parts uploaded by the capture page with the liveness face capture. */
  static Map<String, byte[]> livenessParts(byte[] front, byte[] back, byte[] holding) {
    Map<String, byte[]> parts = parts(front, back, null, null);
    if (holding != null) {
      parts.put("holding", holding);
    }
    return parts;
  }

  /** Parts uploaded by the capture page, by name. */
  static Map<String, byte[]> parts(byte[] front, byte[] back, byte[] face, byte[] video) {
    Map<String, byte[]> parts = new HashMap<>();
    if (front != null) {
      parts.put("front", front);
    }
    if (back != null) {
      parts.put("back", back);
    }
    if (face != null) {
      parts.put("face", face);
    }
    if (video != null) {
      parts.put("video", video);
    }
    return parts;
  }

  private static CaptureSettings settings(List<DocumentSide> sides) {
    return new CaptureSettings(sides, 5, 16, 16, FaceCapture.PHOTO);
  }

  private static final CaptureSettings BOTH_SIDES =
      settings(List.of(DocumentSide.FRONT, DocumentSide.BACK));

  @Test
  void formatsAreDetectedFromMagicBytes() {
    assertEquals(Optional.of(MediaFormat.JPEG), MediaFormat.detect(MediaCategory.IMAGE, JPEG));
    assertEquals(Optional.of(MediaFormat.WEBM), MediaFormat.detect(MediaCategory.VIDEO, WEBM));
    assertEquals(Optional.of(MediaFormat.MP4), MediaFormat.detect(MediaCategory.VIDEO, MP4));
    assertEquals(Optional.empty(), MediaFormat.detect(MediaCategory.IMAGE, PNG));
    assertEquals(Optional.empty(), MediaFormat.detect(MediaCategory.IMAGE, WEBM));
    assertEquals(Optional.empty(), MediaFormat.detect(MediaCategory.VIDEO, JPEG));
    assertEquals(Optional.empty(), MediaFormat.detect(MediaCategory.IMAGE, new byte[] {-1, -40}));
    assertEquals(
        Optional.empty(), MediaFormat.detect(MediaCategory.VIDEO, new byte[] {0, 0, 0, 0, 'f'}));
    assertEquals(Optional.empty(), MediaFormat.detect(MediaCategory.VIDEO, new byte[0]));
  }

  @Test
  void validCaptureIsAccepted() throws InvalidCaptureException {
    CaptureMedia media = CaptureMedia.fromUploads(parts(JPEG, JPEG, JPEG, MP4), BOTH_SIDES);

    assertEquals(
        List.of(
            MediaKind.FRONT_IMAGE,
            MediaKind.BACK_IMAGE,
            MediaKind.FACE_IMAGE,
            MediaKind.SCAN_VIDEO),
        List.copyOf(media.files().keySet()));
    assertEquals(MediaFormat.MP4, media.files().get(MediaKind.SCAN_VIDEO).format());
    assertArrayEquals(MP4, media.files().get(MediaKind.SCAN_VIDEO).content());
  }

  @Test
  void backIsOnlyTakenWhenTheDocumentHasOne() throws InvalidCaptureException {
    CaptureMedia media =
        CaptureMedia.fromUploads(
            parts(JPEG, JPEG, JPEG, WEBM), settings(List.of(DocumentSide.FRONT)));

    assertEquals(
        List.of(MediaKind.FRONT_IMAGE, MediaKind.FACE_IMAGE, MediaKind.SCAN_VIDEO),
        List.copyOf(media.files().keySet()));
  }

  @Test
  void missingPartsAreRejected() {
    assertThrows(
        InvalidCaptureException.class,
        () -> CaptureMedia.fromUploads(parts(JPEG, null, JPEG, WEBM), BOTH_SIDES));
    assertThrows(
        InvalidCaptureException.class,
        () -> CaptureMedia.fromUploads(parts(JPEG, JPEG, null, WEBM), BOTH_SIDES));
    assertThrows(
        InvalidCaptureException.class,
        () -> CaptureMedia.fromUploads(parts(JPEG, JPEG, JPEG, null), BOTH_SIDES));
    assertThrows(
        InvalidCaptureException.class, () -> CaptureMedia.fromUploads(Map.of(), BOTH_SIDES));
  }

  @Test
  void wrongFormatsAreRejected() {
    assertThrows(
        InvalidCaptureException.class,
        () -> CaptureMedia.fromUploads(parts(PNG, JPEG, JPEG, WEBM), BOTH_SIDES));
    assertThrows(
        InvalidCaptureException.class,
        () -> CaptureMedia.fromUploads(parts(JPEG, JPEG, WEBM, WEBM), BOTH_SIDES));
    assertThrows(
        InvalidCaptureException.class,
        () -> CaptureMedia.fromUploads(parts(JPEG, JPEG, JPEG, JPEG), BOTH_SIDES));
    assertThrows(
        InvalidCaptureException.class,
        () -> CaptureMedia.fromUploads(parts(new byte[0], JPEG, JPEG, WEBM), BOTH_SIDES));
  }

  @Test
  void oversizedPartsAreRejected() {
    byte[] bigJpeg = new byte[17];
    System.arraycopy(JPEG, 0, bigJpeg, 0, JPEG.length);
    byte[] exactJpeg = new byte[16];
    System.arraycopy(JPEG, 0, exactJpeg, 0, JPEG.length);

    assertThrows(
        InvalidCaptureException.class,
        () -> CaptureMedia.fromUploads(parts(bigJpeg, JPEG, JPEG, WEBM), BOTH_SIDES));
    assertThrows(
        InvalidCaptureException.class,
        () ->
            CaptureMedia.fromUploads(
                parts(JPEG, JPEG, JPEG, WEBM),
                new CaptureSettings(BOTH_SIDES.sides(), 5, 16, 5, FaceCapture.PHOTO)));
    assertEquals(
        16,
        assertDoesNotThrow(
                () -> CaptureMedia.fromUploads(parts(exactJpeg, JPEG, JPEG, WEBM), BOTH_SIDES))
            .files()
            .get(MediaKind.FRONT_IMAGE)
            .content()
            .length);
  }

  private static final CaptureSettings LIVENESS =
      new CaptureSettings(BOTH_SIDES.sides(), 5, 16, 16, FaceCapture.LIVENESS);

  @Test
  void livenessCaptureHasTheSidesAndThePhotoHoldingTheDocument() throws InvalidCaptureException {
    CaptureMedia media = CaptureMedia.fromUploads(livenessParts(JPEG, JPEG, JPEG), LIVENESS);

    assertEquals(
        List.of(MediaKind.FRONT_IMAGE, MediaKind.BACK_IMAGE, MediaKind.HOLDING_IMAGE),
        List.copyOf(media.files().keySet()));
  }

  @Test
  void livenessCaptureWithoutThePhotoHoldingTheDocumentIsRejected() {
    assertThrows(
        InvalidCaptureException.class,
        () -> CaptureMedia.fromUploads(livenessParts(JPEG, JPEG, null), LIVENESS));
    assertThrows(
        InvalidCaptureException.class,
        () -> CaptureMedia.fromUploads(livenessParts(JPEG, JPEG, PNG), LIVENESS));
  }

  @Test
  void onlyKeepsTheGivenFiles() throws InvalidCaptureException {
    CaptureMedia media =
        CaptureMedia.fromUploads(livenessParts(JPEG, JPEG, JPEG), LIVENESS)
            .only(LIVENESS.uploadedMedia());

    assertEquals(
        List.of(MediaKind.FRONT_IMAGE, MediaKind.BACK_IMAGE), List.copyOf(media.files().keySet()));
  }
}
