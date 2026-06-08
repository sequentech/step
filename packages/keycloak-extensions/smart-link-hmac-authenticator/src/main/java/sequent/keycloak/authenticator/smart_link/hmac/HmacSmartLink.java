// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link.hmac;

import java.nio.charset.StandardCharsets;
import java.security.GeneralSecurityException;
import java.security.MessageDigest;
import java.util.Arrays;
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
 *   message = &lt;user_id&gt;:AuthEvent:&lt;election_event_id&gt;:vote:&lt;unix_timestamp&gt;
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

  /** Minimum fields in {@code user_id : AuthEvent : election_event_id : vote : timestamp}. */
  public static final int MIN_MESSAGE_FIELD_COUNT = 5;

  /** Hex length of a SHA-256 HMAC. */
  public static final int HASH_HEX_LENGTH = 64;

  /** Default validity window, matching the first generation {@code SMARTLINK_TIMEOUT}. */
  public static final long DEFAULT_TIMEOUT_SECONDS = 90L;

  /** Default tolerance for clock differences between the external app and Keycloak. */
  public static final long DEFAULT_CLOCK_SKEW_SECONDS = 5L;

  // Realm attribute names. These MUST stay in sync with the constants in
  // sequent-core: packages/sequent-core/src/types/keycloak.rs
  public static final String ATTR_ENABLED = "smart-link-enabled";
  public static final String ATTR_SHARED_SECRET = "smart-link-shared-secret";
  public static final String ATTR_TIMEOUT_SECONDS = "smart-link-timeout-secs";
  public static final String ATTR_CLOCK_SKEW_SECONDS = "smart-link-clock-skew-secs";
  public static final String ATTR_CLIENT_ID = "smart-link-client-id";
  public static final String ATTR_FORCE_CREATE = "smart-link-force-create";
  public static final String ATTR_REQUIRED_ATTRIBUTES = "smart-link-required-attributes";

  /** Successful result of {@link #validate}. */
  public record ValidatedSmartLink(String userId, String electionEventId, long timestampSeconds) {}

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
   * <p>Checks, in order: envelope shape, digest, message structure, permission, election-event
   * binding, the HMAC itself (constant time), and finally the temporal window. The signature is
   * verified before the temporal checks so that timing decisions are only made about authentic
   * messages.
   *
   * @param authToken the raw {@code khmac:///...} token (already URL-decoded)
   * @param sharedSecret the per-event shared secret configured on the realm
   * @param expectedElectionEventId the election-event id derived from the realm being accessed
   * @param nowEpochSeconds the current time, in seconds since the Unix epoch
   * @param timeoutSeconds how long after creation a token stays valid
   * @param clockSkewSeconds tolerance for the token being slightly ahead of this server's clock
   * @throws SmartLinkValidationException if any check fails
   */
  public static ValidatedSmartLink validate(
      String authToken,
      String sharedSecret,
      String expectedElectionEventId,
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

    // message = <user_id>:AuthEvent:<election_event_id>:vote:<timestamp>
    // Parse from the right so ':' remains valid inside user_id, matching the first generation.
    String[] fields = message.split(":", -1);
    if (fields.length < MIN_MESSAGE_FIELD_COUNT) {
      throw new SmartLinkValidationException(
          SmartLinkError.MALFORMED_MESSAGE, "unexpected field count: " + fields.length);
    }
    int tailIndex = fields.length;
    String timestampField = fields[tailIndex - 1];
    String permissionAction = fields[tailIndex - 2];
    String electionEventId = fields[tailIndex - 3];
    String permissionObject = fields[tailIndex - 4];
    String userId = String.join(":", Arrays.copyOfRange(fields, 0, tailIndex - 4));

    if (userId.isEmpty()) {
      throw new SmartLinkValidationException(SmartLinkError.INVALID_USER_ID, "empty user id");
    }
    if (!PERMISSION_OBJECT.equals(permissionObject)
        || !PERMISSION_ACTION.equals(permissionAction)) {
      throw new SmartLinkValidationException(
          SmartLinkError.INVALID_PERMISSION, "permission is not AuthEvent/vote");
    }
    if (expectedElectionEventId == null || !expectedElectionEventId.equals(electionEventId)) {
      throw new SmartLinkValidationException(
          SmartLinkError.MISMATCHED_EVENT,
          "token event " + electionEventId + " != realm event " + expectedElectionEventId);
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

    // --- temporal validation: must have been created in the past AND still be valid ---
    long skew = Math.max(0L, clockSkewSeconds);
    long timeout = Math.max(0L, timeoutSeconds);
    // Reject future-dated tokens: a valid token can only be minted "now or earlier"
    // (within a small clock-skew tolerance). This blocks pre-minted, long-lived tokens.
    if (timestampSeconds > nowEpochSeconds + skew) {
      throw new SmartLinkValidationException(
          SmartLinkError.TOKEN_IN_FUTURE, "token timestamp is in the future");
    }
    // Reject expired tokens: valid iff timestamp + timeout > now (same rule as the first gen).
    if (timestampSeconds <= nowEpochSeconds - timeout) {
      throw new SmartLinkValidationException(SmartLinkError.TOKEN_EXPIRED, "token has expired");
    }

    return new ValidatedSmartLink(userId, electionEventId, timestampSeconds);
  }

  /** Constant-time comparison of two ASCII/hex strings. */
  private static boolean constantTimeEquals(String a, String b) {
    return MessageDigest.isEqual(
        a.getBytes(StandardCharsets.UTF_8), b.getBytes(StandardCharsets.UTF_8));
  }

  /**
   * Extracts the election-event id from an event realm name.
   *
   * <p>Mirrors {@code parse_realm} in sequent-core: event realms are named {@code
   * tenant-<tenant_id>-event-<election_event_id>}. Returns {@code null} for tenant realms, the
   * master realm, or any name that does not match.
   */
  public static String electionEventIdFromRealm(String realmName) {
    if (realmName == null) {
      return null;
    }
    String[] parts = realmName.split("-");
    if (parts.length < 2 || !"tenant".equals(parts[0])) {
      return null;
    }
    int eventIndex = -1;
    for (int i = 0; i < parts.length; i++) {
      if ("event".equals(parts[i])) {
        eventIndex = i;
        break;
      }
    }
    if (eventIndex > 1 && eventIndex < parts.length - 1) {
      return String.join("-", Arrays.copyOfRange(parts, eventIndex + 1, parts.length));
    }
    return null;
  }
}
