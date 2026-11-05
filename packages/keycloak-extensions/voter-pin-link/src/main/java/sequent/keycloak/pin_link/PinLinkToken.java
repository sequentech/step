// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.pin_link;

import java.nio.charset.StandardCharsets;
import java.security.GeneralSecurityException;
import java.security.MessageDigest;
import java.util.Arrays;
import java.util.HexFormat;
import java.util.Optional;
import java.util.regex.Pattern;
import javax.crypto.Mac;
import javax.crypto.spec.SecretKeySpec;

/**
 * The link token: an HMAC-SHA-256 of the PIN, keyed for one voter and truncated to 130 bits of
 * Crockford base32. A plain hash would let anyone who saw a link recover a 12-digit PIN by brute
 * force. Windmill generates it; {@code voter_pin_link.rs} must stay in step with this class, and a
 * shared fixture pins both.
 */
final class PinLinkToken {
  static final int LENGTH = 26;
  private static final String ALPHABET = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
  private static final Pattern CANONICAL = Pattern.compile("[0-9A-HJKMNP-TV-Z]{" + LENGTH + "}");
  private static final byte[] KEY_DOMAIN =
      "sequent-voter-pin-link-v1".getBytes(StandardCharsets.UTF_8);
  private static final String HMAC = "HmacSHA256";

  private PinLinkToken() {}

  /**
   * The canonical form of a typed token: case-insensitive, separators ignored, and the characters
   * Crockford base32 treats as look-alikes mapped to their digit.
   */
  static Optional<String> canonicalize(String input) {
    if (input == null || input.length() > LENGTH * 2) {
      return Optional.empty();
    }
    String token =
        input
            .replace("-", "")
            .replace(" ", "")
            .toUpperCase(java.util.Locale.ROOT)
            .replace('O', '0')
            .replace('I', '1')
            .replace('L', '1');
    return CANONICAL.matcher(token).matches() ? Optional.of(token) : Optional.empty();
  }

  /** The lookup value: the token has 130 bits of entropy, so a plain hash is safe here. */
  static String lookupHash(String canonicalToken) {
    try {
      return HexFormat.of()
          .formatHex(
              MessageDigest.getInstance("SHA-256")
                  .digest(canonicalToken.getBytes(StandardCharsets.UTF_8)));
    } catch (GeneralSecurityException error) {
      throw new IllegalStateException("SHA-256 is not available");
    }
  }

  /** Derives a voter's token from the PIN, without leaving the key or PIN in any string. */
  static String derive(String masterHex, String tenant, String event, String user, byte[] pin)
      throws GeneralSecurityException {
    byte[] master = HexFormat.of().parseHex(masterHex);
    byte[] prk = null;
    byte[] key = null;
    byte[] tag = null;
    try {
      Mac mac = Mac.getInstance(HMAC);
      mac.init(new SecretKeySpec(KEY_DOMAIN, HMAC));
      prk = mac.doFinal(master);
      mac.init(new SecretKeySpec(prk, HMAC));
      mac.update(
          ("tenant=" + tenant + ";event=" + event + ";user=" + user)
              .getBytes(StandardCharsets.UTF_8));
      key = mac.doFinal(new byte[] {1});
      mac.init(new SecretKeySpec(key, HMAC));
      tag = mac.doFinal(pin);
      return encode(tag);
    } finally {
      wipe(master);
      wipe(prk);
      wipe(key);
      wipe(tag);
    }
  }

  /** Constant-time comparison of two canonical tokens. */
  static boolean sameToken(String left, String right) {
    return MessageDigest.isEqual(
        left.getBytes(StandardCharsets.UTF_8), right.getBytes(StandardCharsets.UTF_8));
  }

  private static String encode(byte[] tag) {
    StringBuilder token = new StringBuilder(LENGTH);
    int buffer = 0;
    int bits = 0;
    int next = 0;
    while (token.length() < LENGTH) {
      if (bits < 5) {
        buffer = (buffer << 8) | (tag[next++] & 0xff);
        bits += 8;
      }
      bits -= 5;
      token.append(ALPHABET.charAt((buffer >> bits) & 0x1f));
      buffer &= (1 << bits) - 1;
    }
    return token.toString();
  }

  private static void wipe(byte[] bytes) {
    if (bytes != null) Arrays.fill(bytes, (byte) 0);
  }
}
