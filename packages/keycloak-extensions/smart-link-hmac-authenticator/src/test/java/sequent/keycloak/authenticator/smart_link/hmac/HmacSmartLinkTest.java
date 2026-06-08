// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link.hmac;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

class HmacSmartLinkTest {

  private static final String SECRET = "the cake is in the oven";
  private static final String ELECTION_ID = "150017";
  private static final String REALM_NAME = "tenant-acme-event-150017";
  private static final String USER = "example@sequentech.io";
  private static final long NOW = 1_780_869_273L;

  /** Builds a token exactly the way an external application would. */
  private static String mintToken(String secret, String userId, String electionId, long timestamp) {
    String message = userId + ":AuthEvent:" + electionId + ":vote:" + timestamp;
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
    String message = USER + ":AuthEvent:" + ELECTION_ID + ":vote:" + NOW;
    assertEquals(
        "89034fa3af76759f6edc658260afb30106c243fe86b60b652f71473fecbb8c4e",
        HmacSmartLink.computeHmacHex(SECRET, message));
  }

  // --- Happy path. ---

  @Test
  void validate_acceptsFreshToken() throws Exception {
    String token = mintToken(SECRET, USER, ELECTION_ID, NOW);
    HmacSmartLink.ValidatedSmartLink result =
        HmacSmartLink.validate(token, SECRET, ELECTION_ID, NOW, 90, 5);
    assertEquals(USER, result.userId());
    assertEquals(ELECTION_ID, result.electionId());
    assertEquals(NOW, result.timestampSeconds());
  }

  @Test
  void validate_acceptsTokenWithinClockSkewInFuture() throws Exception {
    String token = mintToken(SECRET, USER, ELECTION_ID, NOW + 3);
    // 3s in the future, within the 5s skew tolerance.
    HmacSmartLink.validate(token, SECRET, ELECTION_ID, NOW, 90, 5);
  }

  // --- Signature / integrity. ---

  @Test
  void validate_rejectsWrongSecret() {
    String token = mintToken(SECRET, USER, ELECTION_ID, NOW);
    assertEquals(
        SmartLinkError.INVALID_SIGNATURE,
        errorOf(() -> HmacSmartLink.validate(token, "wrong secret", ELECTION_ID, NOW, 90, 5)));
  }

  @Test
  void validate_rejectsTamperedMessage() {
    String token = mintToken(SECRET, USER, ELECTION_ID, NOW);
    // Flip the user id while keeping the original signature.
    String tampered = token.replace(USER, "attacker@evil.example");
    assertEquals(
        SmartLinkError.INVALID_SIGNATURE,
        errorOf(() -> HmacSmartLink.validate(tampered, SECRET, ELECTION_ID, NOW, 90, 5)));
  }

  // --- Election binding. ---

  @Test
  void validate_rejectsMismatchedElectionId() {
    String token = mintToken(SECRET, USER, ELECTION_ID, NOW);
    assertEquals(
        SmartLinkError.MISMATCHED_EVENT,
        errorOf(() -> HmacSmartLink.validate(token, SECRET, "999999", NOW, 90, 5)));
  }

  // --- Temporal: created in the past, still valid. ---

  @Test
  void validate_rejectsExpiredToken() {
    String token = mintToken(SECRET, USER, ELECTION_ID, NOW);
    // 120s old with a 90s window.
    assertEquals(
        SmartLinkError.TOKEN_EXPIRED,
        errorOf(() -> HmacSmartLink.validate(token, SECRET, ELECTION_ID, NOW + 120, 90, 5)));
  }

  @Test
  void validate_rejectsFutureDatedToken() {
    String token = mintToken(SECRET, USER, ELECTION_ID, NOW + 600);
    // 600s in the future, far beyond the 5s skew.
    assertEquals(
        SmartLinkError.TOKEN_IN_FUTURE,
        errorOf(() -> HmacSmartLink.validate(token, SECRET, ELECTION_ID, NOW, 90, 5)));
  }

  // --- Structural. ---

  @Test
  void validate_rejectsMissingSecret() {
    String token = mintToken(SECRET, USER, ELECTION_ID, NOW);
    assertEquals(
        SmartLinkError.NOT_CONFIGURED,
        errorOf(() -> HmacSmartLink.validate(token, "", ELECTION_ID, NOW, 90, 5)));
  }

  @Test
  void validate_rejectsBlankSecret() {
    String token = mintToken(SECRET, USER, ELECTION_ID, NOW);
    assertEquals(
        SmartLinkError.NOT_CONFIGURED,
        errorOf(() -> HmacSmartLink.validate(token, " ", ELECTION_ID, NOW, 90, 5)));
  }

  @Test
  void validate_rejectsBadEnvelope() {
    assertEquals(
        SmartLinkError.MALFORMED_TOKEN,
        errorOf(
            () -> HmacSmartLink.validate("not-a-khmac-token", SECRET, ELECTION_ID, NOW, 90, 5)));
  }

  @Test
  void validate_rejectsUnsupportedDigest() {
    String message = USER + ":AuthEvent:" + ELECTION_ID + ":vote:" + NOW;
    String token =
        HmacSmartLink.ENVELOPE_PREFIX
            + "sha-1;"
            + HmacSmartLink.computeHmacHex(SECRET, message)
            + "/"
            + message;
    assertEquals(
        SmartLinkError.UNSUPPORTED_DIGEST,
        errorOf(() -> HmacSmartLink.validate(token, SECRET, ELECTION_ID, NOW, 90, 5)));
  }

  @Test
  void validate_rejectsWrongPermission() {
    String message = USER + ":AuthEvent:" + ELECTION_ID + ":revoke:" + NOW;
    String token =
        HmacSmartLink.ENVELOPE_PREFIX
            + HmacSmartLink.DIGEST_LABEL
            + ";"
            + HmacSmartLink.computeHmacHex(SECRET, message)
            + "/"
            + message;
    assertEquals(
        SmartLinkError.INVALID_PERMISSION,
        errorOf(() -> HmacSmartLink.validate(token, SECRET, ELECTION_ID, NOW, 90, 5)));
  }

  @Test
  void validate_rejectsEmptyUserId() {
    String token = mintToken(SECRET, "", ELECTION_ID, NOW);
    assertEquals(
        SmartLinkError.INVALID_USER_ID,
        errorOf(() -> HmacSmartLink.validate(token, SECRET, ELECTION_ID, NOW, 90, 5)));
  }

  @Test
  void validate_rejectsUserIdContainingColon() {
    String userId = "ex:amp./le@nvot;es.com";
    String token = mintToken(SECRET, userId, ELECTION_ID, NOW);
    assertEquals(
        SmartLinkError.INVALID_USER_ID,
        errorOf(() -> HmacSmartLink.validate(token, SECRET, ELECTION_ID, NOW, 90, 5)));
  }

  @Test
  void validate_acceptsUserIdContainingOtherUrlSafeSeparators() throws Exception {
    String userId = "ex.amp/le@nvot;es.com";
    String token = mintToken(SECRET, userId, ELECTION_ID, NOW);
    HmacSmartLink.ValidatedSmartLink result =
        HmacSmartLink.validate(token, SECRET, ELECTION_ID, NOW, 90, 5);
    assertEquals(userId, result.userId());
  }

  @Test
  void validate_rejectsNonPositiveTimeoutOrSkewAsMisconfigured() {
    String token = mintToken(SECRET, USER, ELECTION_ID, NOW);
    assertEquals(
        SmartLinkError.NOT_CONFIGURED,
        errorOf(() -> HmacSmartLink.validate(token, SECRET, ELECTION_ID, NOW, 0, 5)));
    assertEquals(
        SmartLinkError.NOT_CONFIGURED,
        errorOf(() -> HmacSmartLink.validate(token, SECRET, ELECTION_ID, NOW, 90, 0)));
  }

  @Test
  void validate_rejectsMessageWithTooFewFields() {
    String message = USER + ":AuthEvent:" + ELECTION_ID + ":" + NOW;
    String token =
        HmacSmartLink.ENVELOPE_PREFIX
            + HmacSmartLink.DIGEST_LABEL
            + ";"
            + HmacSmartLink.computeHmacHex(SECRET, message)
            + "/"
            + message;
    assertEquals(
        SmartLinkError.MALFORMED_MESSAGE,
        errorOf(() -> HmacSmartLink.validate(token, SECRET, ELECTION_ID, NOW, 90, 5)));
  }

  // --- Election id selection. ---

  @Test
  void smartLinkElectionId_defaultsToRealmName() {
    assertEquals(REALM_NAME, HmacSmartLink.smartLinkElectionId(REALM_NAME, null));
  }

  @Test
  void smartLinkElectionId_usesConfiguredValue() {
    assertEquals(ELECTION_ID, HmacSmartLink.smartLinkElectionId(REALM_NAME, ELECTION_ID));
  }

  @Test
  void smartLinkElectionId_trimsConfiguredValue() {
    assertEquals(ELECTION_ID, HmacSmartLink.smartLinkElectionId(REALM_NAME, " 150017 "));
  }

  @Test
  void smartLinkElectionId_blankConfiguredValueDefaultsToRealmName() {
    assertEquals(REALM_NAME, HmacSmartLink.smartLinkElectionId(REALM_NAME, " "));
  }

  @Test
  void smartLinkElectionId_rejectsInvalidConfiguredValue() {
    assertNull(HmacSmartLink.smartLinkElectionId(REALM_NAME, "bad/value"));
    assertNull(HmacSmartLink.smartLinkElectionId(REALM_NAME, "bad:value"));
  }

  @Test
  void isValidElectionId_acceptsUrlSafeText() {
    assertTrue(HmacSmartLink.isValidElectionId("150017"));
    assertTrue(HmacSmartLink.isValidElectionId("tenant-acme-event-150017"));
    assertTrue(HmacSmartLink.isValidElectionId("spring_2026.v2"));
  }

  @Test
  void isValidElectionId_rejectsBlankOrUnsafeText() {
    assertFalse(HmacSmartLink.isValidElectionId(""));
    assertFalse(HmacSmartLink.isValidElectionId(" "));
    assertFalse(HmacSmartLink.isValidElectionId("bad/value"));
    assertFalse(HmacSmartLink.isValidElectionId("bad:value"));
  }
}
