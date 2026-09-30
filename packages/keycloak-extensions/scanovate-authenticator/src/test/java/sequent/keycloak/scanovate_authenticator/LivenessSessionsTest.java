// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static sequent.keycloak.scanovate_authenticator.CaptureMediaTest.JPEG;

import com.fasterxml.jackson.databind.JsonNode;
import java.util.ArrayList;
import java.util.Base64;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import org.junit.jupiter.api.Test;
import sequent.keycloak.scanovate_authenticator.LivenessSessions.LivenessResult;
import sequent.keycloak.scanovate_authenticator.LivenessSessions.RecordOutcome;

class LivenessSessionsTest {
  private static final String SECRET = "callback-secret";
  private static final String CASE_ID = "proc-1";

  /** In-memory stand-in for Keycloak's single-use object store. */
  static class MemoryStore implements LivenessStore {
    final Map<String, Map<String, String>> entries = new HashMap<>();

    @Override
    public void put(String key, long lifespanSeconds, Map<String, String> value) {
      entries.put(key, Map.copyOf(value));
    }

    @Override
    public Map<String, String> get(String key) {
      return entries.get(key);
    }

    @Override
    public void remove(String key) {
      entries.remove(key);
    }
  }

  private final MemoryStore store = new MemoryStore();
  private final List<Long> sleeps = new ArrayList<>();
  private final LivenessSessions sessions = new LivenessSessions(store, sleeps::add);

  /**
   * Result callback of a session run in Liveness Plus PRESENTATION mode, where the injection check
   * does not run and is reported as null.
   */
  static String result(String status, boolean passed, byte[] image) {
    return result(status, passed, passed, "null", image);
  }

  static String result(
      String status,
      boolean livenessPassed,
      boolean presentationPassed,
      String injectionPassed,
      byte[] image) {
    String imageField =
        image == null
            ? ""
            : ", \"image\": \"%s\"".formatted(Base64.getEncoder().encodeToString(image));
    String processingResult =
        ("{\"liveness_check_passed\": %s, \"presentation_attack_check_passed\": %s,"
                + " \"injection_attack_check_passed\": %s%s}")
            .formatted(livenessPassed, presentationPassed, injectionPassed, imageField);
    return """
        {"message_type": "result", "status": "%s",
         "scan": {"processing_result": %s},
         "onprem_params": {"case_id": "proc-1"}}
        """
        .formatted(status, processingResult);
  }

  static final String START =
      """
      {"message_type": "start", "status": "session_started",
       "onprem_params": {"case_id": "proc-1"}}
      """;

  @Test
  void tokensAreRandomAndUrlSafe() {
    String first = sessions.create(CASE_ID, SECRET);
    String second = sessions.create(CASE_ID, SECRET);

    assertNotEquals(first, second);
    assertTrue(first.matches("[A-Za-z0-9_-]{43}"), first);
  }

  @Test
  void verifiesTheTokenOfTheCase() {
    String token = sessions.create(CASE_ID, SECRET);

    assertTrue(sessions.verify(token, CASE_ID, SECRET));
  }

  @Test
  void rejectsUnknownTokensWrongCasesAndWrongSecrets() {
    String token = sessions.create(CASE_ID, SECRET);

    assertFalse(sessions.verify("unknown", CASE_ID, SECRET));
    assertFalse(sessions.verify(token, "proc-2", SECRET));
    assertFalse(sessions.verify(token, CASE_ID, "wrong"));
    assertFalse(sessions.verify(token, CASE_ID, null));
    assertFalse(sessions.verify(null, CASE_ID, SECRET));
  }

  @Test
  void caseIdIsOptionalWhenVerifying() {
    String token = sessions.create(CASE_ID, SECRET);

    assertTrue(sessions.verify(token, null, SECRET));
  }

  @Test
  void limitsTheLivenessSessionsOfAToken() {
    String token = sessions.create(CASE_ID, SECRET);

    for (int i = 0; i < LivenessSessions.MAX_SESSIONS_PER_TOKEN; i++) {
      assertTrue(sessions.verify(token, CASE_ID, SECRET));
    }
    assertFalse(sessions.verify(token, CASE_ID, SECRET));
  }

  @Test
  void recordsAPassedResultWithItsImage() {
    String token = sessions.create(CASE_ID, SECRET);

    assertEquals(
        RecordOutcome.RECORDED, sessions.record(token, SECRET, ScanovateResultsTest.json(START)));
    assertEquals(
        RecordOutcome.RECORDED,
        sessions.record(token, SECRET, ScanovateResultsTest.json(result("completed", true, JPEG))));

    LivenessResult result = sessions.awaitResult(token, 5).orElseThrow();
    assertTrue(result.passed());
    assertEquals("completed", result.status());
    assertArrayEquals(JPEG, result.image().orElseThrow());
    assertTrue(sleeps.isEmpty());
  }

  @Test
  void completedSessionsThatFailTheCheckDoNotPass() {
    String token = sessions.create(CASE_ID, SECRET);
    sessions.record(token, SECRET, ScanovateResultsTest.json(result("completed", false, JPEG)));

    LivenessResult result = sessions.awaitResult(token, 5).orElseThrow();
    assertFalse(result.passed());
    assertTrue(result.image().isEmpty());
  }

  @Test
  void failedPresentationAttackCheckDoesNotPass() {
    String token = sessions.create(CASE_ID, SECRET);
    sessions.record(
        token, SECRET, ScanovateResultsTest.json(result("completed", true, false, "null", JPEG)));

    assertFalse(sessions.awaitResult(token, 5).orElseThrow().passed());
  }

  @Test
  void failedInjectionAttackCheckDoesNotPass() {
    String token = sessions.create(CASE_ID, SECRET);
    sessions.record(
        token, SECRET, ScanovateResultsTest.json(result("completed", true, true, "false", JPEG)));

    assertFalse(sessions.awaitResult(token, 5).orElseThrow().passed());
  }

  @Test
  void passedInjectionAttackCheckPasses() {
    String token = sessions.create(CASE_ID, SECRET);
    sessions.record(
        token, SECRET, ScanovateResultsTest.json(result("completed", true, true, "true", JPEG)));

    assertTrue(sessions.awaitResult(token, 5).orElseThrow().passed());
  }

  @Test
  void missingPresentationAttackCheckDoesNotPass() {
    String token = sessions.create(CASE_ID, SECRET);
    JsonNode body = ScanovateResultsTest.json(result("completed", true, JPEG));
    body.withObject("/scan/processing_result").remove("presentation_attack_check_passed");
    sessions.record(token, SECRET, body);

    assertFalse(sessions.awaitResult(token, 5).orElseThrow().passed());
  }

  @Test
  void abortedSessionsDoNotPassEvenIfTheCheckDid() {
    String token = sessions.create(CASE_ID, SECRET);
    sessions.record(token, SECRET, ScanovateResultsTest.json(result("aborted", true, JPEG)));

    LivenessResult result = sessions.awaitResult(token, 5).orElseThrow();
    assertFalse(result.passed());
    assertEquals("aborted", result.status());
  }

  @Test
  void passedResultsWithoutAnImageDoNotPass() {
    String token = sessions.create(CASE_ID, SECRET);
    sessions.record(token, SECRET, ScanovateResultsTest.json(result("completed", true, null)));

    assertFalse(sessions.awaitResult(token, 5).orElseThrow().passed());
  }

  @Test
  void aNewSessionReplacesAnEarlierFailure() {
    String token = sessions.create(CASE_ID, SECRET);
    sessions.record(token, SECRET, ScanovateResultsTest.json(result("aborted", false, null)));
    sessions.record(token, SECRET, ScanovateResultsTest.json(START));
    sessions.record(token, SECRET, ScanovateResultsTest.json(result("completed", true, JPEG)));

    assertTrue(sessions.awaitResult(token, 5).orElseThrow().passed());
  }

  @Test
  void rejectsCallbacksWithAWrongSecretOrToken() {
    String token = sessions.create(CASE_ID, SECRET);
    String body = result("completed", true, JPEG);

    assertEquals(
        RecordOutcome.UNAUTHORIZED,
        sessions.record(token, "wrong", ScanovateResultsTest.json(body)));
    assertEquals(
        RecordOutcome.UNAUTHORIZED,
        sessions.record("unknown", SECRET, ScanovateResultsTest.json(body)));
    assertEquals(Optional.empty(), sessions.awaitResult(token, 0));
  }

  @Test
  void rejectsCallbacksForAnotherCase() {
    String token = sessions.create(CASE_ID, SECRET);
    String body = result("completed", true, JPEG).replace("proc-1", "proc-2");

    assertEquals(
        RecordOutcome.UNAUTHORIZED,
        sessions.record(token, SECRET, ScanovateResultsTest.json(body)));
  }

  @Test
  void ignoresDebugResults() {
    String token = sessions.create(CASE_ID, SECRET);

    assertEquals(
        RecordOutcome.IGNORED,
        sessions.record(
            token,
            SECRET,
            ScanovateResultsTest.json(
                "{\"message_type\": \"result_debug\", \"status\": \"completed\"}")));
    assertEquals(Optional.empty(), sessions.awaitResult(token, 0));
  }

  @Test
  void rejectsMalformedCallbacks() {
    String token = sessions.create(CASE_ID, SECRET);

    assertEquals(RecordOutcome.INVALID, sessions.record(token, SECRET, null));
    assertEquals(
        RecordOutcome.INVALID,
        sessions.record(
            token,
            SECRET,
            ScanovateResultsTest.json("{\"message_type\": \"result\", \"status\": 1}")));
    JsonNode badImage = ScanovateResultsTest.json(result("completed", true, JPEG));
    badImage.withObject("/scan/processing_result").put("image", "not base64!");
    assertEquals(RecordOutcome.INVALID, sessions.record(token, SECRET, badImage));
  }

  @Test
  void waitsForTheResultUntilTheTimeout() {
    String token = sessions.create(CASE_ID, SECRET);
    sessions.record(token, SECRET, ScanovateResultsTest.json(START));

    assertEquals(Optional.empty(), sessions.awaitResult(token, 2));
    assertEquals(
        2000L / LivenessSessions.POLL_INTERVAL_MS,
        sleeps.size(),
        "polls every " + LivenessSessions.POLL_INTERVAL_MS + " ms");
  }

  @Test
  void stopsWaitingOnceTheResultArrives() {
    String token = sessions.create(CASE_ID, SECRET);
    LivenessSessions arriving =
        new LivenessSessions(
            store,
            millis -> {
              sleeps.add(millis);
              if (sleeps.size() == 2) {
                new LivenessSessions(store, ignored -> {})
                    .record(
                        token, SECRET, ScanovateResultsTest.json(result("completed", true, JPEG)));
              }
            });

    assertTrue(arriving.awaitResult(token, 10).orElseThrow().passed());
    assertEquals(2, sleeps.size());
  }

  @Test
  void discardedTokensCanNoLongerBeUsed() {
    String token = sessions.create(CASE_ID, SECRET);
    sessions.discard(token);

    assertFalse(sessions.verify(token, CASE_ID, SECRET));
    assertEquals(
        RecordOutcome.UNAUTHORIZED,
        sessions.record(token, SECRET, ScanovateResultsTest.json(START)));
    sessions.discard(null);
  }
}
