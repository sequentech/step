// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.voter_enrollment;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.Locale;
import java.util.Optional;

/**
 * The reference a voter gives support while their enrollment is reviewed ("Q5KW-8RFX"). It comes
 * from the application record's id only, never from the session or a contact, so support can find
 * the application by it:
 *
 * <p>Crockford base32 (0-9, A-Z without I, L, O, U) of the first 40 bits of SHA-256 of {@code
 * "application:" + id}, the id in lower case, shown as two groups of four.
 */
public final class SupportReference {
  private static final char[] ALPHABET = "0123456789ABCDEFGHJKMNPQRSTVWXYZ".toCharArray();

  private SupportReference() {}

  public static Optional<String> of(String applicationId) {
    if (applicationId == null || applicationId.isBlank()) {
      return Optional.empty();
    }
    byte[] digest;
    try {
      digest =
          MessageDigest.getInstance("SHA-256")
              .digest(
                  ("application:" + applicationId.trim().toLowerCase(Locale.ROOT))
                      .getBytes(StandardCharsets.UTF_8));
    } catch (NoSuchAlgorithmException e) {
      throw new IllegalStateException(e);
    }
    long bits = 0;
    for (int i = 0; i < 5; i++) {
      bits = (bits << 8) | (digest[i] & 0xFF);
    }
    StringBuilder reference = new StringBuilder(9);
    for (int i = 7; i >= 0; i--) {
      reference.append(ALPHABET[(int) ((bits >> (i * 5)) & 0x1F)]);
      if (i == 4) {
        reference.append('-');
      }
    }
    return Optional.of(reference.toString());
  }
}
