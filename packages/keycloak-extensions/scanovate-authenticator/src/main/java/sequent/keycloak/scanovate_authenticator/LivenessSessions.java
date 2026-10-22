// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import com.fasterxml.jackson.databind.JsonNode;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.SecureRandom;
import java.util.Base64;
import java.util.HashMap;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import lombok.extern.jbosslog.JBossLog;

/**
 * One-time tokens that let a voter's browser open a Liveness Plus session, and the results that
 * Liveness Plus posts back for them.
 *
 * <p>Keycloak gives the token and the case id to the capture page, which starts a Liveness Plus
 * session with them ({@code POST /create_session}) and sends it the frames of the voter's face.
 * Liveness Plus checks the token with Keycloak before starting the session ({@link #verify}) and
 * posts the start and the result of the session to Keycloak ({@link #record}), server to server.
 * Both calls carry a shared secret that the browser never sees, as the token and the case id go
 * through the browser. The verdict is only ever taken from these callbacks, never from the browser.
 */
@JBossLog
public class LivenessSessions {
  static final String KEY_PREFIX = "scanovate-liveness:";
  static final long LIFESPAN_SECONDS = 30 * 60;
  static final int MAX_SESSIONS_PER_TOKEN = 3;
  static final long POLL_INTERVAL_MS = 500;
  static final String STATUS_COMPLETED = "completed";

  static final String MESSAGE_START = "start";
  static final String MESSAGE_RESULT = "result";

  private static final String CASE_ID = "case-id";
  private static final String SECRET = "secret";
  private static final String SESSIONS = "sessions";
  private static final String STATUS = "status";
  private static final String PASSED = "passed";
  private static final String IMAGE = "image";
  private static final String STATUS_PENDING = "pending";
  private static final Set<String> RUNNING = Set.of(STATUS_PENDING, "session_started");
  private static final SecureRandom RANDOM = new SecureRandom();

  /** What was done with a callback of Liveness Plus. */
  public enum RecordOutcome {
    RECORDED,
    IGNORED,
    UNAUTHORIZED,
    INVALID
  }

  /**
   * Final result of the last Liveness Plus session of a token.
   *
   * @param status final session status: completed, expired, aborted or client_error
   * @param passed whether the session completed and passed the presentation attack check, and the
   *     injection attack check unless Liveness Plus runs without it (PRESENTATION mode)
   * @param image the voter's picture taken during the session, only when passed
   */
  public record LivenessResult(String status, boolean passed, Optional<byte[]> image) {}

  private final LivenessStore store;
  private final Sleeper sleeper;

  public LivenessSessions(LivenessStore store, Sleeper sleeper) {
    this.store = store;
    this.sleeper = sleeper;
  }

  /** Creates a token for the given case, whose callbacks must carry the given secret. */
  public String create(String caseId, String secret) {
    byte[] bytes = new byte[32];
    RANDOM.nextBytes(bytes);
    String token = Base64.getUrlEncoder().withoutPadding().encodeToString(bytes);
    store.put(
        key(token),
        LIFESPAN_SECONDS,
        Map.of(CASE_ID, caseId, SECRET, secret, SESSIONS, "0", STATUS, STATUS_PENDING));
    return token;
  }

  /**
   * Whether Liveness Plus may start a session for the token. Each token allows a few sessions, so
   * the voter can try again after a camera problem, but not indefinitely.
   */
  public boolean verify(String token, String caseId, String secret) {
    Optional<Map<String, String>> entry = authorized(token, secret);
    if (entry.isEmpty() || (caseId != null && !caseId.equals(entry.get().get(CASE_ID)))) {
      return false;
    }
    int sessions = ScanovateAuthenticator.parseInt(entry.get().get(SESSIONS), 0);
    if (sessions >= MAX_SESSIONS_PER_TOKEN) {
      log.warnv("verify: too many liveness sessions for case {0}", entry.get().get(CASE_ID));
      return false;
    }
    Map<String, String> updated = new HashMap<>(entry.get());
    updated.put(SESSIONS, String.valueOf(sessions + 1));
    store.put(key(token), LIFESPAN_SECONDS, updated);
    return true;
  }

  /** Records a start or result callback of Liveness Plus. */
  public RecordOutcome record(String token, String secret, JsonNode body) {
    Optional<Map<String, String>> entry = authorized(token, secret);
    if (entry.isEmpty()) {
      return RecordOutcome.UNAUTHORIZED;
    }
    if (body == null || !body.isObject()) {
      return RecordOutcome.INVALID;
    }
    JsonNode caseId = body.at("/onprem_params/case_id");
    if (!caseId.isMissingNode() && !caseId.asText().equals(entry.get().get(CASE_ID))) {
      return RecordOutcome.UNAUTHORIZED;
    }

    String messageType = body.path("message_type").asText();
    if (!MESSAGE_START.equals(messageType) && !MESSAGE_RESULT.equals(messageType)) {
      return RecordOutcome.IGNORED;
    }
    JsonNode status = body.get(STATUS);
    if (status == null || !status.isTextual() || status.asText().isBlank()) {
      return RecordOutcome.INVALID;
    }

    Map<String, String> updated = new HashMap<>(entry.get());
    updated.remove(PASSED);
    updated.remove(IMAGE);
    updated.put(STATUS, MESSAGE_START.equals(messageType) ? "session_started" : status.asText());
    if (MESSAGE_RESULT.equals(messageType)) {
      JsonNode processing = body.at("/scan/processing_result");
      Optional<String> image = text(processing, "image").or(() -> text(processing, "face_image"));
      if (image.isPresent() && decode(image.get()).isEmpty()) {
        return RecordOutcome.INVALID;
      }
      boolean passed =
          STATUS_COMPLETED.equals(status.asText())
              && isTrue(processing, "liveness_check_passed")
              && isTrue(processing, "presentation_attack_check_passed")
              && !isFalse(processing, "injection_attack_check_passed")
              && image.isPresent();
      updated.put(PASSED, String.valueOf(passed));
      if (passed) {
        updated.put(IMAGE, image.get());
      }
      log.infov(
          "record: liveness session of case {0} ended with {1}, passed={2}",
          entry.get().get(CASE_ID), status.asText(), passed);
    }
    store.put(key(token), LIFESPAN_SECONDS, updated);
    return RecordOutcome.RECORDED;
  }

  /**
   * Waits up to the given time for the final result of the token's last session, as the browser may
   * learn that the session ended before Keycloak gets the result callback.
   */
  public Optional<LivenessResult> awaitResult(String token, int timeoutSeconds) {
    long polls = timeoutSeconds * 1000L / POLL_INTERVAL_MS;
    for (long poll = 0; ; poll++) {
      Optional<LivenessResult> result = finalResult(token);
      if (result.isPresent() || poll >= polls) {
        return result;
      }
      try {
        sleeper.sleep(POLL_INTERVAL_MS);
      } catch (InterruptedException e) {
        Thread.currentThread().interrupt();
        return Optional.empty();
      }
    }
  }

  /** Invalidates the token. */
  public void discard(String token) {
    if (token != null) {
      store.remove(key(token));
    }
  }

  private Optional<LivenessResult> finalResult(String token) {
    Map<String, String> entry = token == null ? null : store.get(key(token));
    if (entry == null || RUNNING.contains(entry.get(STATUS))) {
      return Optional.empty();
    }
    Optional<byte[]> image =
        Optional.ofNullable(entry.get(IMAGE)).flatMap(LivenessSessions::decode);
    boolean passed = Boolean.parseBoolean(entry.get(PASSED)) && image.isPresent();
    return Optional.of(new LivenessResult(entry.get(STATUS), passed, image));
  }

  private Optional<Map<String, String>> authorized(String token, String secret) {
    if (token == null || secret == null) {
      return Optional.empty();
    }
    Map<String, String> entry = store.get(key(token));
    if (entry == null || !constantTimeEquals(secret, entry.get(SECRET))) {
      return Optional.empty();
    }
    return Optional.of(entry);
  }

  private static boolean constantTimeEquals(String given, String expected) {
    return expected != null
        && MessageDigest.isEqual(
            given.getBytes(StandardCharsets.UTF_8), expected.getBytes(StandardCharsets.UTF_8));
  }

  private static boolean isTrue(JsonNode node, String field) {
    JsonNode value = node.get(field);
    return value != null && value.isBoolean() && value.asBoolean();
  }

  /** Whether the check ran and failed: Liveness Plus reports checks it did not run as null. */
  private static boolean isFalse(JsonNode node, String field) {
    JsonNode value = node.get(field);
    return value != null && !value.isNull() && !(value.isBoolean() && value.asBoolean());
  }

  private static Optional<String> text(JsonNode node, String field) {
    JsonNode value = node.get(field);
    return value != null && value.isTextual() && !value.asText().isBlank()
        ? Optional.of(value.asText())
        : Optional.empty();
  }

  private static Optional<byte[]> decode(String base64) {
    try {
      return Optional.of(Base64.getDecoder().decode(base64.replaceAll("\\s", "")));
    } catch (IllegalArgumentException e) {
      return Optional.empty();
    }
  }

  private static String key(String token) {
    return KEY_PREFIX + token;
  }
}
