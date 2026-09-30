// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.security.SecureRandom;
import java.util.Base64;
import java.util.HashMap;
import java.util.Map;
import java.util.Optional;

/**
 * Files the capture page uploads before submitting the capture, see {@link
 * ScanovateReturnResource#uploadCapture}.
 *
 * <p>Keycloak can't receive them with the capture form: its authentication flow reads every login
 * form as text, which fails on file parts, and it limits text fields to 128 KiB. So the page
 * uploads each file on its own, with a one-time token Keycloak issues when it renders the capture
 * page, and then submits the form without files. Keycloak reads the files back by the token kept in
 * the authentication session, never by anything the browser posts with the form.
 *
 * <p>Each part is its own entry, so that uploads of different parts can't overwrite each other.
 * Only the format and a size cap are checked here; the capture applies the configured limits.
 */
public class CaptureUploads {
  static final String KEY_PREFIX = "scanovate-capture:";
  static final long LIFESPAN_SECONDS = 30 * 60;

  /** Hard cap of any uploaded file, above the configured limits of the photos and the video. */
  static final int MAX_UPLOAD_BYTES = 8 * 1024 * 1024;

  private static final String CONTENT = "content";
  private static final Map<String, String> ISSUED = Map.of("issued", "true");
  private static final SecureRandom RANDOM = new SecureRandom();

  /** What was done with an uploaded file. */
  public enum UploadOutcome {
    STORED,
    UNAUTHORIZED,
    UNKNOWN_PART,
    EMPTY,
    TOO_LARGE,
    INVALID_FORMAT
  }

  private final LivenessStore store;

  public CaptureUploads(LivenessStore store) {
    this.store = store;
  }

  /** Issues a token for a rendered capture page, with no files yet. */
  public String create() {
    byte[] bytes = new byte[32];
    RANDOM.nextBytes(bytes);
    String token = Base64.getUrlEncoder().withoutPadding().encodeToString(bytes);
    store.put(key(token), LIFESPAN_SECONDS, ISSUED);
    return token;
  }

  /**
   * Stores (or replaces) a file of the token.
   *
   * @param part name of the part, one of the {@link MediaKind#formPart()} names
   * @param content the file, possibly read up to one byte past {@link #MAX_UPLOAD_BYTES}
   */
  public UploadOutcome store(String token, String part, byte[] content) {
    if (!issued(token)) {
      return UploadOutcome.UNAUTHORIZED;
    }
    Optional<MediaKind> kind = MediaKind.fromFormPart(part);
    if (kind.isEmpty()) {
      return UploadOutcome.UNKNOWN_PART;
    }
    if (content == null || content.length == 0) {
      return UploadOutcome.EMPTY;
    }
    if (content.length > MAX_UPLOAD_BYTES) {
      return UploadOutcome.TOO_LARGE;
    }
    if (MediaFormat.detect(kind.get().category(), content).isEmpty()) {
      return UploadOutcome.INVALID_FORMAT;
    }
    store.put(
        key(token, kind.get()),
        LIFESPAN_SECONDS,
        Map.of(CONTENT, Base64.getEncoder().encodeToString(content)));
    return UploadOutcome.STORED;
  }

  /** The files uploaded with the token, by part name, or empty if the token is unknown. */
  public Optional<Map<String, byte[]>> parts(String token) {
    if (!issued(token)) {
      return Optional.empty();
    }
    Map<String, byte[]> parts = new HashMap<>();
    for (MediaKind kind : MediaKind.values()) {
      Map<String, String> entry = store.get(key(token, kind));
      if (entry != null && entry.get(CONTENT) != null) {
        parts.put(kind.formPart(), Base64.getDecoder().decode(entry.get(CONTENT)));
      }
    }
    return Optional.of(parts);
  }

  /** Forgets the token and its files. */
  public void discard(String token) {
    if (token == null) {
      return;
    }
    for (MediaKind kind : MediaKind.values()) {
      store.remove(key(token, kind));
    }
    store.remove(key(token));
  }

  private boolean issued(String token) {
    return token != null && store.get(key(token)) != null;
  }

  private static String key(String token) {
    return KEY_PREFIX + token;
  }

  private static String key(String token, MediaKind kind) {
    return KEY_PREFIX + token + ":" + kind.formPart();
  }
}
