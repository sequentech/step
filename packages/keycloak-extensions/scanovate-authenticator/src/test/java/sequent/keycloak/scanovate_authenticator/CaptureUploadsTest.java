// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static sequent.keycloak.scanovate_authenticator.CaptureMediaTest.JPEG;
import static sequent.keycloak.scanovate_authenticator.CaptureMediaTest.PNG;
import static sequent.keycloak.scanovate_authenticator.CaptureMediaTest.WEBM;

import java.util.Map;
import java.util.Optional;
import org.junit.jupiter.api.Test;
import sequent.keycloak.scanovate_authenticator.CaptureUploads.UploadOutcome;

class CaptureUploadsTest {
  private final LivenessSessionsTest.MemoryStore store = new LivenessSessionsTest.MemoryStore();
  private final CaptureUploads uploads = new CaptureUploads(store);

  @Test
  void storesThePartsOfAToken() {
    String token = uploads.create();

    assertEquals(UploadOutcome.STORED, uploads.store(token, "front", JPEG));
    assertEquals(UploadOutcome.STORED, uploads.store(token, "holding", JPEG));

    Map<String, byte[]> parts = uploads.parts(token).orElseThrow();
    assertEquals(2, parts.size());
    assertArrayEquals(JPEG, parts.get("front"));
    assertArrayEquals(JPEG, parts.get("holding"));
  }

  @Test
  void newTokensStartEmptyAndAreUnique() {
    String token = uploads.create();

    assertEquals(Optional.of(Map.of()), uploads.parts(token));
    assertNotEquals(token, uploads.create());
  }

  @Test
  void uploadingAPartAgainReplacesIt() {
    String token = uploads.create();
    byte[] other = JPEG.clone();
    other[5] = 0x42;

    uploads.store(token, "holding", JPEG);
    uploads.store(token, "holding", other);

    assertArrayEquals(other, uploads.parts(token).orElseThrow().get("holding"));
  }

  @Test
  void onlyIssuedTokensAcceptUploads() {
    assertEquals(UploadOutcome.UNAUTHORIZED, uploads.store("forged", "front", JPEG));
    assertEquals(UploadOutcome.UNAUTHORIZED, uploads.store(null, "front", JPEG));
    assertEquals(Optional.empty(), uploads.parts("forged"));
    assertEquals(Optional.empty(), uploads.parts(null));
    assertTrue(store.entries.isEmpty());
  }

  @Test
  void rejectsUnknownPartsEmptyOversizedAndUnrecognisedFiles() {
    String token = uploads.create();
    byte[] oversized = new byte[CaptureUploads.MAX_UPLOAD_BYTES + 1];
    System.arraycopy(JPEG, 0, oversized, 0, JPEG.length);

    assertEquals(UploadOutcome.UNKNOWN_PART, uploads.store(token, "selfie", JPEG));
    assertEquals(UploadOutcome.UNKNOWN_PART, uploads.store(token, "face", JPEG));
    assertEquals(UploadOutcome.UNKNOWN_PART, uploads.store(token, "video", WEBM));
    assertEquals(UploadOutcome.UNKNOWN_PART, uploads.store(token, null, JPEG));
    assertEquals(UploadOutcome.EMPTY, uploads.store(token, "front", new byte[0]));
    assertEquals(UploadOutcome.EMPTY, uploads.store(token, "front", null));
    assertEquals(UploadOutcome.TOO_LARGE, uploads.store(token, "front", oversized));
    assertEquals(UploadOutcome.INVALID_FORMAT, uploads.store(token, "front", PNG));
    assertEquals(UploadOutcome.INVALID_FORMAT, uploads.store(token, "front", WEBM));
    assertEquals(Optional.of(Map.of()), uploads.parts(token));
  }

  @Test
  void discardingATokenForgetsItsParts() {
    String token = uploads.create();
    uploads.store(token, "front", JPEG);
    uploads.store(token, "back", JPEG);

    uploads.discard(token);
    uploads.discard(null);

    assertEquals(Optional.empty(), uploads.parts(token));
    assertEquals(UploadOutcome.UNAUTHORIZED, uploads.store(token, "front", JPEG));
    assertTrue(store.entries.isEmpty());
  }
}
