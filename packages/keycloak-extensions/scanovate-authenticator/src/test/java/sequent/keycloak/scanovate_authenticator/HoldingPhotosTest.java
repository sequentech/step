// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static sequent.keycloak.scanovate_authenticator.OnPremSessionsTest.JPEG;

import java.util.Arrays;
import org.junit.jupiter.api.Test;
import sequent.keycloak.scanovate_authenticator.HoldingPhotos.UploadOutcome;

class HoldingPhotosTest {
  private final OnPremSessionsTest.MemoryStore store = new OnPremSessionsTest.MemoryStore();
  private final HoldingPhotos photos = new HoldingPhotos(store);

  @Test
  void anIssuedTokenStoresAJpegAndReadsItBack() {
    String token = photos.create();
    assertTrue(photos.photo(token).isEmpty());

    assertEquals(UploadOutcome.STORED, photos.store(token, JPEG));

    assertArrayEquals(JPEG, photos.photo(token).orElseThrow());
  }

  @Test
  void everyPageGetsItsOwnToken() {
    assertNotEquals(photos.create(), photos.create());
  }

  @Test
  void onlyAnIssuedTokenCanUpload() {
    assertEquals(UploadOutcome.UNAUTHORIZED, photos.store(null, JPEG));
    assertEquals(UploadOutcome.UNAUTHORIZED, photos.store("forged", JPEG));
    assertTrue(store.entries.isEmpty());
  }

  @Test
  void onlyANonEmptyJpegWithinTheCapIsKept() {
    String token = photos.create();
    assertEquals(UploadOutcome.EMPTY, photos.store(token, null));
    assertEquals(UploadOutcome.EMPTY, photos.store(token, new byte[0]));
    assertEquals(UploadOutcome.INVALID_FORMAT, photos.store(token, "GIF89a".getBytes()));
    assertEquals(UploadOutcome.INVALID_FORMAT, photos.store(token, new byte[] {(byte) 0xFF}));
    byte[] tooLarge = Arrays.copyOf(JPEG, HoldingPhotos.MAX_UPLOAD_BYTES + 1);
    assertEquals(UploadOutcome.TOO_LARGE, photos.store(token, tooLarge));
    assertTrue(photos.photo(token).isEmpty());
  }

  @Test
  void aNewUploadReplacesTheEarlierPhoto() {
    String token = photos.create();
    photos.store(token, JPEG);
    byte[] retaken = Arrays.copyOf(JPEG, JPEG.length + 1);

    photos.store(token, retaken);

    assertArrayEquals(retaken, photos.photo(token).orElseThrow());
  }

  @Test
  void aDiscardedTokenForgetsItsPhotoAndRefusesUploads() {
    String token = photos.create();
    photos.store(token, JPEG);

    photos.discard(token);
    photos.discard(null);

    assertTrue(photos.photo(token).isEmpty());
    assertEquals(UploadOutcome.UNAUTHORIZED, photos.store(token, JPEG));
    assertTrue(store.entries.isEmpty());
  }
}
