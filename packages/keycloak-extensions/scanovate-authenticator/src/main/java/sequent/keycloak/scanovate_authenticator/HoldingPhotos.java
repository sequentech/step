// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.security.SecureRandom;
import java.util.Arrays;
import java.util.Base64;
import java.util.Map;
import java.util.Optional;

/**
 * The photo of the voter holding their document, which the capture page uploads before submitting
 * the capture, see {@link IdentityVerificationResource#uploadHoldingPhoto}.
 *
 * <p>Keycloak can't receive it with the capture form: its authentication flow reads every login
 * form as text, and limits text fields to 128 KiB. So the page uploads the photo on its own, with a
 * one-time token Keycloak issues when it renders the capture page, and then submits the form
 * without it. Keycloak reads the photo back by the token kept in the authentication session, never
 * by anything the browser posts with the form.
 */
public class HoldingPhotos {
  static final String KEY_PREFIX = "scanovate-holding:";
  static final long LIFESPAN_SECONDS = 30 * 60;

  /** Hard cap of an uploaded photo. */
  static final int MAX_UPLOAD_BYTES = 8 * 1024 * 1024;

  private static final byte[] JPEG_SIGNATURE = {(byte) 0xFF, (byte) 0xD8, (byte) 0xFF};
  private static final String CONTENT = "content";
  private static final Map<String, String> ISSUED = Map.of("issued", "true");
  private static final SecureRandom RANDOM = new SecureRandom();

  /** What was done with an uploaded photo. */
  public enum UploadOutcome {
    STORED,
    UNAUTHORIZED,
    EMPTY,
    TOO_LARGE,
    INVALID_FORMAT
  }

  private final SessionStore store;

  public HoldingPhotos(SessionStore store) {
    this.store = store;
  }

  /** Issues a token for a rendered capture page, with no photo yet. */
  public String create() {
    byte[] bytes = new byte[32];
    RANDOM.nextBytes(bytes);
    String token = Base64.getUrlEncoder().withoutPadding().encodeToString(bytes);
    store.put(key(token), LIFESPAN_SECONDS, ISSUED);
    return token;
  }

  /**
   * Stores (or replaces) the photo of the token: a JPEG, recognised by its signature rather than by
   * what the browser claims.
   *
   * @param content the photo, possibly read up to one byte past {@link #MAX_UPLOAD_BYTES}
   */
  public UploadOutcome store(String token, byte[] content) {
    if (!issued(token)) {
      return UploadOutcome.UNAUTHORIZED;
    }
    if (content == null || content.length == 0) {
      return UploadOutcome.EMPTY;
    }
    if (content.length > MAX_UPLOAD_BYTES) {
      return UploadOutcome.TOO_LARGE;
    }
    if (content.length < JPEG_SIGNATURE.length
        || !Arrays.equals(
            content, 0, JPEG_SIGNATURE.length, JPEG_SIGNATURE, 0, JPEG_SIGNATURE.length)) {
      return UploadOutcome.INVALID_FORMAT;
    }
    store.put(
        photoKey(token),
        LIFESPAN_SECONDS,
        Map.of(CONTENT, Base64.getEncoder().encodeToString(content)));
    return UploadOutcome.STORED;
  }

  /** The photo uploaded with the token, or empty if there is none or the token is unknown. */
  public Optional<byte[]> photo(String token) {
    if (!issued(token)) {
      return Optional.empty();
    }
    Map<String, String> entry = store.get(photoKey(token));
    return entry == null || entry.get(CONTENT) == null
        ? Optional.empty()
        : Optional.of(Base64.getDecoder().decode(entry.get(CONTENT)));
  }

  /** Forgets the token and its photo. */
  public void discard(String token) {
    if (token == null) {
      return;
    }
    store.remove(photoKey(token));
    store.remove(key(token));
  }

  private boolean issued(String token) {
    return token != null && store.get(key(token)) != null;
  }

  private static String key(String token) {
    return KEY_PREFIX + token;
  }

  private static String photoKey(String token) {
    return KEY_PREFIX + token + ":photo";
  }
}
