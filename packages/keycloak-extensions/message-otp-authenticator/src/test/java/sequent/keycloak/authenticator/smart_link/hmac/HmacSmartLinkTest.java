// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link.hmac;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;

import org.junit.jupiter.api.Test;

class HmacSmartLinkTest {

  private static final String SECRET = "the cake is in the oven";
  private static final String EVENT_ID = "150017";
  private static final String USER = "example@sequentech.io";
  private static final long NOW = 1_780_869_273L;

  /** Builds a token exactly the way an external application would. */
  private static String mintToken(String secret, String userId, String eventId, long timestamp) {
    String message = userId + ":AuthEvent:" + eventId + ":vote:" + timestamp;
    return HmacSmartLink.ENVELOPE_PREFIX
        + HmacSmartLink.DIGEST_LABEL
        + ";"
        + HmacSmartLink.computeHmacHex(secret, message)
        + "/"
        + message;
  }

  private static SmartLinkError errorOf(Executable call) {
    SmartLinkValidationException ex = assertThrows(SmartLinkValidationException.class, call::run);
    return ex.getError();
  }

  @FunctionalInterface
  private interface Executable {
    void run() throws Exception;
  }

  // --- Cross-generation compatibility: same value as the Python/Scala/Go generators. ---

  @Test
  void computeHmacHex_matchesKnownVector() {
    String message = USER + ":AuthEvent:" + EVENT_ID + ":vote:" + NOW;
    assertEquals(
        "89034fa3af76759f6edc658260afb30106c243fe86b60b652f71473fecbb8c4e",
        HmacSmartLink.computeHmacHex(SECRET, message));
  }

  // --- Happy path. ---

  @Test
  void validate_acceptsFreshToken() throws Exception {
    String token = mintToken(SECRET, USER, EVENT_ID, NOW);
    HmacSmartLink.ValidatedSmartLink result =
        HmacSmartLink.validate(token, SECRET, EVENT_ID, NOW, 90, 5);
    assertEquals(USER, result.userId());
    assertEquals(EVENT_ID, result.electionEventId());
    assertEquals(NOW, result.timestampSeconds());
  }

  @Test
  void validate_acceptsTokenWithinClockSkewInFuture() throws Exception {
    String token = mintToken(SECRET, USER, EVENT_ID, NOW + 3);
    // 3s in the future, within the 5s skew tolerance.
    HmacSmartLink.validate(token, SECRET, EVENT_ID, NOW, 90, 5);
  }

  // --- Signature / integrity. ---

  @Test
  void validate_rejectsWrongSecret() {
    String token = mintToken(SECRET, USER, EVENT_ID, NOW);
    assertEquals(
        SmartLinkError.INVALID_SIGNATURE,
        errorOf(() -> HmacSmartLink.validate(token, "wrong secret", EVENT_ID, NOW, 90, 5)));
  }

  @Test
  void validate_rejectsTamperedMessage() {
    String token = mintToken(SECRET, USER, EVENT_ID, NOW);
    // Flip the user id while keeping the original signature.
    String tampered = token.replace(USER, "attacker@evil.example");
    assertEquals(
        SmartLinkError.INVALID_SIGNATURE,
        errorOf(() -> HmacSmartLink.validate(tampered, SECRET, EVENT_ID, NOW, 90, 5)));
  }

  // --- Event binding. ---

  @Test
  void validate_rejectsMismatchedEvent() {
    String token = mintToken(SECRET, USER, EVENT_ID, NOW);
    assertEquals(
        SmartLinkError.MISMATCHED_EVENT,
        errorOf(() -> HmacSmartLink.validate(token, SECRET, "999999", NOW, 90, 5)));
  }

  // --- Temporal: created in the past, still valid. ---

  @Test
  void validate_rejectsExpiredToken() {
    String token = mintToken(SECRET, USER, EVENT_ID, NOW);
    // 120s old with a 90s window.
    assertEquals(
        SmartLinkError.TOKEN_EXPIRED,
        errorOf(() -> HmacSmartLink.validate(token, SECRET, EVENT_ID, NOW + 120, 90, 5)));
  }

  @Test
  void validate_rejectsFutureDatedToken() {
    String token = mintToken(SECRET, USER, EVENT_ID, NOW + 600);
    // 600s in the future, far beyond the 5s skew.
    assertEquals(
        SmartLinkError.TOKEN_IN_FUTURE,
        errorOf(() -> HmacSmartLink.validate(token, SECRET, EVENT_ID, NOW, 90, 5)));
  }

  // --- Structural. ---

  @Test
  void validate_rejectsMissingSecret() {
    String token = mintToken(SECRET, USER, EVENT_ID, NOW);
    assertEquals(
        SmartLinkError.NOT_CONFIGURED,
        errorOf(() -> HmacSmartLink.validate(token, "", EVENT_ID, NOW, 90, 5)));
  }

  @Test
  void validate_rejectsBadEnvelope() {
    assertEquals(
        SmartLinkError.MALFORMED_TOKEN,
        errorOf(
            () ->
                HmacSmartLink.validate(
                    "not-a-khmac-token", SECRET, EVENT_ID, NOW, 90, 5)));
  }

  @Test
  void validate_rejectsUnsupportedDigest() {
    String message = USER + ":AuthEvent:" + EVENT_ID + ":vote:" + NOW;
    String token =
        HmacSmartLink.ENVELOPE_PREFIX
            + "sha-1;"
            + HmacSmartLink.computeHmacHex(SECRET, message)
            + "/"
            + message;
    assertEquals(
        SmartLinkError.UNSUPPORTED_DIGEST,
        errorOf(() -> HmacSmartLink.validate(token, SECRET, EVENT_ID, NOW, 90, 5)));
  }

  @Test
  void validate_rejectsWrongPermission() {
    String message = USER + ":AuthEvent:" + EVENT_ID + ":revoke:" + NOW;
    String token =
        HmacSmartLink.ENVELOPE_PREFIX
            + HmacSmartLink.DIGEST_LABEL
            + ";"
            + HmacSmartLink.computeHmacHex(SECRET, message)
            + "/"
            + message;
    assertEquals(
        SmartLinkError.INVALID_PERMISSION,
        errorOf(() -> HmacSmartLink.validate(token, SECRET, EVENT_ID, NOW, 90, 5)));
  }

  @Test
  void validate_rejectsEmptyUserId() {
    String token = mintToken(SECRET, "", EVENT_ID, NOW);
    assertEquals(
        SmartLinkError.INVALID_USER_ID,
        errorOf(() -> HmacSmartLink.validate(token, SECRET, EVENT_ID, NOW, 90, 5)));
  }

  @Test
  void validate_rejectsUserIdContainingColon() {
    // A ':' in the user id yields 6 fields, so the message no longer parses.
    String token = mintToken(SECRET, "a:b", EVENT_ID, NOW);
    assertEquals(
        SmartLinkError.MALFORMED_MESSAGE,
        errorOf(() -> HmacSmartLink.validate(token, SECRET, EVENT_ID, NOW, 90, 5)));
  }

  // --- Realm name parsing. ---

  @Test
  void electionEventIdFromRealm_parsesEventRealm() {
    assertEquals("150017", HmacSmartLink.electionEventIdFromRealm("tenant-acme-event-150017"));
  }

  @Test
  void electionEventIdFromRealm_handlesIdsWithDashes() {
    assertEquals(
        "2025-spring-election",
        HmacSmartLink.electionEventIdFromRealm("tenant-acme-event-2025-spring-election"));
  }

  @Test
  void electionEventIdFromRealm_returnsNullForTenantRealm() {
    assertNull(HmacSmartLink.electionEventIdFromRealm("tenant-acme"));
  }

  @Test
  void electionEventIdFromRealm_returnsNullForMaster() {
    assertNull(HmacSmartLink.electionEventIdFromRealm("master"));
  }
}
