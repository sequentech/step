// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import org.junit.jupiter.api.Test;
import org.keycloak.http.FormPartValue;

class CaptureMediaTest {
  static final byte[] JPEG = {(byte) 0xFF, (byte) 0xD8, (byte) 0xFF, (byte) 0xE0, 0, 0x10};
  static final byte[] WEBM = {0x1A, 0x45, (byte) 0xDF, (byte) 0xA3, 0, 0};
  static final byte[] MP4 = {0, 0, 0, 0x18, 'f', 't', 'y', 'p', 'm', 'p', '4', '2'};
  static final byte[] PNG = {(byte) 0x89, 'P', 'N', 'G', 0x0D, 0x0A};

  /** Multipart part backed by bytes, like the file parts Keycloak hands to authenticators. */
  record BytesPart(byte[] content) implements FormPartValue {
    @Override
    public String asString() {
      throw new IllegalStateException("file part");
    }

    @Override
    public InputStream asInputStream() {
      return new ByteArrayInputStream(content);
    }
  }

  /** Part whose content cannot be read. */
  static final FormPartValue BROKEN_PART =
      new FormPartValue() {
        @Override
        public String asString() {
          throw new IllegalStateException("file part");
        }

        @Override
        public InputStream asInputStream() {
          return new InputStream() {
            @Override
            public int read() throws IOException {
              throw new IOException("gone");
            }
          };
        }
      };

  static Map<String, List<FormPartValue>> parts(
      byte[] front, byte[] back, byte[] face, byte[] video) {
    Map<String, List<FormPartValue>> parts = new HashMap<>();
    if (front != null) {
      parts.put("front", List.of(new BytesPart(front)));
    }
    if (back != null) {
      parts.put("back", List.of(new BytesPart(back)));
    }
    if (face != null) {
      parts.put("face", List.of(new BytesPart(face)));
    }
    if (video != null) {
      parts.put("video", List.of(new BytesPart(video)));
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
    CaptureMedia media = CaptureMedia.fromParts(parts(JPEG, JPEG, JPEG, MP4), BOTH_SIDES);

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
        CaptureMedia.fromParts(
            parts(JPEG, JPEG, JPEG, WEBM), settings(List.of(DocumentSide.FRONT)));

    assertEquals(
        List.of(MediaKind.FRONT_IMAGE, MediaKind.FACE_IMAGE, MediaKind.SCAN_VIDEO),
        List.copyOf(media.files().keySet()));
  }

  @Test
  void missingPartsAreRejected() {
    assertThrows(
        InvalidCaptureException.class,
        () -> CaptureMedia.fromParts(parts(JPEG, null, JPEG, WEBM), BOTH_SIDES));
    assertThrows(
        InvalidCaptureException.class,
        () -> CaptureMedia.fromParts(parts(JPEG, JPEG, null, WEBM), BOTH_SIDES));
    assertThrows(
        InvalidCaptureException.class,
        () -> CaptureMedia.fromParts(parts(JPEG, JPEG, JPEG, null), BOTH_SIDES));
    assertThrows(InvalidCaptureException.class, () -> CaptureMedia.fromParts(Map.of(), BOTH_SIDES));
  }

  @Test
  void wrongFormatsAreRejected() {
    assertThrows(
        InvalidCaptureException.class,
        () -> CaptureMedia.fromParts(parts(PNG, JPEG, JPEG, WEBM), BOTH_SIDES));
    assertThrows(
        InvalidCaptureException.class,
        () -> CaptureMedia.fromParts(parts(JPEG, JPEG, WEBM, WEBM), BOTH_SIDES));
    assertThrows(
        InvalidCaptureException.class,
        () -> CaptureMedia.fromParts(parts(JPEG, JPEG, JPEG, JPEG), BOTH_SIDES));
    assertThrows(
        InvalidCaptureException.class,
        () -> CaptureMedia.fromParts(parts(new byte[0], JPEG, JPEG, WEBM), BOTH_SIDES));
  }

  @Test
  void oversizedPartsAreRejected() {
    byte[] bigJpeg = new byte[17];
    System.arraycopy(JPEG, 0, bigJpeg, 0, JPEG.length);
    byte[] exactJpeg = new byte[16];
    System.arraycopy(JPEG, 0, exactJpeg, 0, JPEG.length);

    assertThrows(
        InvalidCaptureException.class,
        () -> CaptureMedia.fromParts(parts(bigJpeg, JPEG, JPEG, WEBM), BOTH_SIDES));
    assertThrows(
        InvalidCaptureException.class,
        () ->
            CaptureMedia.fromParts(
                parts(JPEG, JPEG, JPEG, WEBM),
                new CaptureSettings(BOTH_SIDES.sides(), 5, 16, 5, FaceCapture.PHOTO)));
    assertEquals(
        16,
        assertDoesNotThrow(
                () -> CaptureMedia.fromParts(parts(exactJpeg, JPEG, JPEG, WEBM), BOTH_SIDES))
            .files()
            .get(MediaKind.FRONT_IMAGE)
            .content()
            .length);
  }

  @Test
  void repeatedPartsAreRejected() {
    Map<String, List<FormPartValue>> parts = parts(JPEG, JPEG, JPEG, WEBM);
    parts.put("face", List.of(new BytesPart(JPEG), new BytesPart(JPEG)));

    assertThrows(InvalidCaptureException.class, () -> CaptureMedia.fromParts(parts, BOTH_SIDES));
  }

  @Test
  void unreadablePartsAreRejected() {
    Map<String, List<FormPartValue>> parts = parts(JPEG, JPEG, JPEG, WEBM);
    parts.put("video", List.of(BROKEN_PART));

    assertThrows(InvalidCaptureException.class, () -> CaptureMedia.fromParts(parts, BOTH_SIDES));
  }

  @Test
  void livenessImageIsAddedAsTheFacePhoto() throws InvalidCaptureException {
    CaptureMedia media =
        CaptureMedia.fromParts(
                parts(JPEG, JPEG, null, null),
                new CaptureSettings(BOTH_SIDES.sides(), 5, 16, 16, FaceCapture.LIVENESS))
            .withImage(MediaKind.FACE_IMAGE, JPEG, 16);

    assertEquals(
        List.of(MediaKind.FRONT_IMAGE, MediaKind.BACK_IMAGE, MediaKind.FACE_IMAGE),
        List.copyOf(media.files().keySet()));
    assertEquals(MediaFormat.JPEG, media.files().get(MediaKind.FACE_IMAGE).format());
  }

  @Test
  void invalidOrOversizedLivenessImagesAreRejected() throws InvalidCaptureException {
    CaptureMedia media =
        CaptureMedia.fromParts(
            parts(JPEG, JPEG, null, null),
            new CaptureSettings(BOTH_SIDES.sides(), 5, 16, 16, FaceCapture.LIVENESS));

    assertThrows(
        InvalidCaptureException.class, () -> media.withImage(MediaKind.FACE_IMAGE, PNG, 16));
    assertThrows(
        InvalidCaptureException.class, () -> media.withImage(MediaKind.FACE_IMAGE, JPEG, 5));
  }
}
