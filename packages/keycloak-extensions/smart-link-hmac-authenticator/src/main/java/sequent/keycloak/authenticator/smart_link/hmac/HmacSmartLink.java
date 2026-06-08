// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link.hmac;

import java.nio.charset.StandardCharsets;
import java.security.GeneralSecurityException;
import java.security.MessageDigest;
import java.util.HexFormat;
import javax.crypto.Mac;
import javax.crypto.spec.SecretKeySpec;

/**
 * Pure, side-effect-free validation of an externally generated "Smart Link" auth-token.
 *
 * <p>This is the second-generation equivalent of the first-generation IAM {@code m_smart_link.py}
 * check. The token is symmetric: an external application that knows the per-event shared secret
 * mints it offline, and Keycloak verifies it here. The wire format is byte-for-byte identical to
 * the first generation so existing token generators keep working:
 *
 * <pre>
 *   message = &lt;user_id&gt;:AuthEvent:&lt;election_id&gt;:vote:&lt;unix_timestamp&gt;
 *   code    = HMAC_SHA256(shared_secret, message)   // lowercase hex
 *   token   = khmac:///sha-256;&lt;code&gt;/&lt;message&gt;
 * </pre>
 *
 * <p>All methods are static and deterministic, so this class is exhaustively unit-testable without
 * a running Keycloak.
 */
public final class HmacSmartLink {

  private HmacSmartLink() {}

  public static final String ENVELOPE_PREFIX = "khmac:///";
  public static final String DIGEST_LABEL = "sha-256";
  public static final String HMAC_ALGORITHM = "HmacSHA256";
  public static final String PERMISSION_OBJECT = "AuthEvent";
  public static final String PERMISSION_ACTION = "vote";

  /** Minimum fields in {@code user_id : AuthEvent : election_id : vote : timestamp}. */
  public static final int MIN_MESSAGE_FIELD_COUNT = 5;

  /** Hex length of a SHA-256 HMAC. */
  public static final int HASH_HEX_LENGTH = 64;

  /** Default validity window, matching the first generation {@code SMARTLINK_TIMEOUT}. */
  public static final long DEFAULT_TIMEOUT_SECONDS = 90L;

  /** Default tolerance for clock differences between the external app and Keycloak. */
  public static final long DEFAULT_CLOCK_SKEW_SECONDS = 5L;

  /** Maximum accepted length of the public Smart Link election id. */
  public static final int ELECTION_ID_MAX_LENGTH = 255;

  // Realm attribute names. These MUST stay in sync with the constants in
  // sequent-core: packages/sequent-core/src/types/keycloak.rs
  public static final String ATTR_ENABLED = "smart-link-enabled";
  public static final String ATTR_SHARED_SECRET = "smart-link-shared-secret";
  public static final String ATTR_TIMEOUT_SECONDS = "smart-link-timeout-secs";
  public static final String ATTR_CLOCK_SKEW_SECONDS = "smart-link-clock-skew-secs";
  public static final String ATTR_CLIENT_ID = "smart-link-client-id";
  public static final String ATTR_ELECTION_ID = "smart-link-election-id";
  public static final String ATTR_REQUIRED_ATTRIBUTES = "smart-link-required-attributes";

  /** Successful result of {@link #validate}. */
  public record ValidatedSmartLink(String userId, String electionId, long timestampSeconds) {}

  /**
   * Computes the lowercase-hex HMAC-SHA256 of {@code message} keyed with {@code sharedSecret}.
   *
   * <p>Uses UTF-8 for both the key and the message, producing exactly the same value as the
   * first-generation generators (Python {@code hmac}, Scala {@code Crypto.hmac}, Go, Rust).
   */
  public static String computeHmacHex(String sharedSecret, String message) {
    try {
      Mac mac = Mac.getInstance(HMAC_ALGORITHM);
      mac.init(new SecretKeySpec(sharedSecret.getBytes(StandardCharsets.UTF_8), HMAC_ALGORITHM));
      byte[] raw = mac.doFinal(message.getBytes(StandardCharsets.UTF_8));
      return HexFormat.of().formatHex(raw);
    } catch (GeneralSecurityException e) {
      // HmacSHA256 is mandated by the JCA spec; if it is missing the JVM is unusable.
      throw new IllegalStateException("HMAC-SHA256 is unavailable in this JVM", e);
    }
  }

  /**
   * Validates an auth-token end to end and returns the authenticated identity.
   *
   * <p>Checks, in order: envelope shape, digest, message structure, permission, election id
   * binding, the HMAC itself (constant time), and finally the temporal window. The expected
   * election id is the configured {@code smart-link-election-id}, or the realm name when the
   * attribute is unset. The signature is verified before the temporal checks so that timing
   * decisions are only made about authentic messages.
   *
   * @param authToken the raw {@code khmac:///...} token (already URL-decoded)
   * @param sharedSecret the per-event shared secret configured on the realm
   * @param expectedElectionId the Smart Link election id expected for the realm being accessed
   * @param nowEpochSeconds the current time, in seconds since the Unix epoch
   * @param timeoutSeconds how long after creation a token stays valid
   * @param clockSkewSeconds tolerance for the token being slightly ahead of this server's clock
   * @throws SmartLinkValidationException if any check fails
   */
  public static ValidatedSmartLink validate(
      String authToken,
      String sharedSecret,
      String expectedElectionId,
      long nowEpochSeconds,
      long timeoutSeconds,
      long clockSkewSeconds)
      throws SmartLinkValidationException {

    if (sharedSecret == null || sharedSecret.isBlank()) {
      throw new SmartLinkValidationException(
          SmartLinkError.NOT_CONFIGURED, "no shared secret configured for realm");
    }
    if (authToken == null || !authToken.startsWith(ENVELOPE_PREFIX)) {
      throw new SmartLinkValidationException(
          SmartLinkError.MALFORMED_TOKEN, "missing or malformed khmac envelope");
    }

    // khmac:///<digest>;<hash>/<message>
    String tail = authToken.substring(ENVELOPE_PREFIX.length());
    int semicolon = tail.indexOf(';');
    if (semicolon < 0) {
      throw new SmartLinkValidationException(
          SmartLinkError.MALFORMED_TOKEN, "missing ';' separator");
    }
    String digest = tail.substring(0, semicolon);
    if (!DIGEST_LABEL.equals(digest)) {
      throw new SmartLinkValidationException(
          SmartLinkError.UNSUPPORTED_DIGEST, "unsupported digest: " + digest);
    }
    String afterDigest = tail.substring(semicolon + 1);
    int slash = afterDigest.indexOf('/');
    if (slash < 0) {
      throw new SmartLinkValidationException(
          SmartLinkError.MALFORMED_TOKEN, "missing '/' separator");
    }
    String hash = afterDigest.substring(0, slash);
    String message = afterDigest.substring(slash + 1);
    if (hash.length() != HASH_HEX_LENGTH || message.isEmpty()) {
      throw new SmartLinkValidationException(
          SmartLinkError.MALFORMED_TOKEN, "bad hash length or empty message");
    }

    // message = <user_id>:AuthEvent:<election_id>:vote:<timestamp>
    // First-generation SmartLink expected exactly five fields; ':' is not valid in user_id.
    String[] fields = message.split(":", -1);
    if (fields.length != MIN_MESSAGE_FIELD_COUNT) {
      if (fields.length > MIN_MESSAGE_FIELD_COUNT) {
        throw new SmartLinkValidationException(
            SmartLinkError.INVALID_USER_ID, "user id must not contain ':'");
      }
      throw new SmartLinkValidationException(
          SmartLinkError.MALFORMED_MESSAGE, "unexpected field count: " + fields.length);
    }
    String userId = fields[0];
    String permissionObject = fields[1];
    String electionId = fields[2];
    String permissionAction = fields[3];
    String timestampField = fields[4];

    if (userId.isEmpty()) {
      throw new SmartLinkValidationException(SmartLinkError.INVALID_USER_ID, "empty user id");
    }
    if (!PERMISSION_OBJECT.equals(permissionObject)
        || !PERMISSION_ACTION.equals(permissionAction)) {
      throw new SmartLinkValidationException(
          SmartLinkError.INVALID_PERMISSION, "permission is not AuthEvent/vote");
    }
    if (expectedElectionId == null || !expectedElectionId.equals(electionId)) {
      throw new SmartLinkValidationException(
          SmartLinkError.MISMATCHED_EVENT,
          "token election " + electionId + " != expected election " + expectedElectionId);
    }

    long timestampSeconds;
    try {
      timestampSeconds = Long.parseLong(timestampField);
    } catch (NumberFormatException e) {
      throw new SmartLinkValidationException(
          SmartLinkError.MALFORMED_MESSAGE, "timestamp is not an integer");
    }

    // --- cryptographic gate: verify the HMAC before trusting any timing in the message ---
    String expectedHash = computeHmacHex(sharedSecret, message);
    if (!constantTimeEquals(expectedHash, hash)) {
      throw new SmartLinkValidationException(
          SmartLinkError.INVALID_SIGNATURE, "HMAC mismatch (wrong secret or tampered message)");
    }

    if (timeoutSeconds <= 0L || clockSkewSeconds <= 0L) {
      throw new SmartLinkValidationException(
          SmartLinkError.NOT_CONFIGURED, "timeout and clock skew must be positive");
    }

    // --- temporal validation: must have been created in the past AND still be valid ---
    // Reject future-dated tokens: a valid token can only be minted "now or earlier"
    // (within a small clock-skew tolerance). This blocks pre-minted, long-lived tokens.
    if (timestampSeconds > nowEpochSeconds + clockSkewSeconds) {
      throw new SmartLinkValidationException(
          SmartLinkError.TOKEN_IN_FUTURE, "token timestamp is in the future");
    }
    // Reject expired tokens: valid iff timestamp + timeout > now (same rule as the first gen).
    if (timestampSeconds <= nowEpochSeconds - timeoutSeconds) {
      throw new SmartLinkValidationException(SmartLinkError.TOKEN_EXPIRED, "token has expired");
    }

    return new ValidatedSmartLink(userId, electionId, timestampSeconds);
  }

  /** Constant-time comparison of two ASCII/hex strings. */
  private static boolean constantTimeEquals(String a, String b) {
    return MessageDigest.isEqual(
        a.getBytes(StandardCharsets.UTF_8), b.getBytes(StandardCharsets.UTF_8));
  }

  /**
   * Returns the Smart Link election id for a realm.
   *
   * <p>If {@code smart-link-election-id} is configured, that text value is used. Otherwise the
   * realm name itself is the default election id. This keeps the URL and token independent from the
   * internal realm event id while still letting deployments expose first-generation-style numeric
   * ids when needed.
   */
  public static String smartLinkElectionId(String realmName, String configuredElectionId) {
    String electionId = realmName;
    if (configuredElectionId != null && !configuredElectionId.isBlank()) {
      electionId = configuredElectionId.trim();
    }
    return isValidElectionId(electionId) ? electionId : null;
  }

  /** Returns true when {@code electionId} is safe for both the URL path and token message. */
  public static boolean isValidElectionId(String electionId) {
    if (electionId == null
        || electionId.isBlank()
        || electionId.length() > ELECTION_ID_MAX_LENGTH) {
      return false;
    }
    for (int i = 0; i < electionId.length(); i++) {
      char c = electionId.charAt(i);
      if (!isAsciiAlphaNumeric(c) && c != '.' && c != '_' && c != '-') {
        return false;
      }
    }
    return true;
  }

  private static boolean isAsciiAlphaNumeric(char c) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9');
  }
}
