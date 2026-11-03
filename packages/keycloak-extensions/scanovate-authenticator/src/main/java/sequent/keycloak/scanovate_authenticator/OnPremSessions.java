// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.SecureRandom;
import java.time.LocalDate;
import java.util.Base64;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import lombok.extern.jbosslog.JBossLog;
import sequent.keycloak.authenticator.diagnostics.ScanovateDiagnostics;

/**
 * One-time tokens that let a voter's browser open a session of an on-premise Scanovate service, in
 * its own capture page, and the results that the service posts back for them.
 *
 * <p>Keycloak puts the token and the case id in the URL of the service's page, which the capture
 * page embeds. The service checks the token with Keycloak before starting a session ({@link
 * #verify}) and posts the start and the result of the session to Keycloak ({@link #record}), server
 * to server. Both calls carry a shared secret that the browser never sees, as the token and the
 * case id go through the browser. The verdict is only ever taken from these callbacks, never from
 * the browser.
 */
@JBossLog
public class OnPremSessions {
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
  private static final String FACE = "face";
  private static final String READING = "reading";
  private static final String SIDE_PREFIX = "side.";
  private static final String STATUS_PENDING = "pending";
  private static final Set<String> RUNNING = Set.of(STATUS_PENDING, "session_started");
  private static final SecureRandom RANDOM = new SecureRandom();
  private static final ObjectMapper MAPPER = new ObjectMapper();

  /** What was done with a callback of the service. */
  public enum RecordOutcome {
    RECORDED,
    IGNORED,
    UNAUTHORIZED,
    INVALID
  }

  /**
   * Final result of the last session of a token.
   *
   * @param status final session status: completed, expired, aborted or client_error
   * @param passed whether the session completed and passed the checks of the service
   * @param face the voter's face: the live picture taken by Liveness Plus, or the portrait of the
   *     document read by OCR; only when passed
   * @param reading what OCR read from the document, see {@link OcrResults#combine}; only when
   *     passed
   * @param sides the status of each side of the document OCR processed, for diagnostics
   */
  public record SessionResult(
      String status,
      boolean passed,
      Optional<byte[]> face,
      Optional<JsonNode> reading,
      Map<String, String> sides) {}

  /** What a result callback says. */
  private record Reading(
      boolean passed, Optional<String> face, Optional<String> reading, Map<String, String> sides) {}

  private final OnPremService service;
  private final SessionStore store;
  private final Sleeper sleeper;

  public OnPremSessions(OnPremService service, SessionStore store, Sleeper sleeper) {
    this.service = service;
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
   * Whether the service may start a session for the token. Each token allows a few sessions, so the
   * voter can try again after a camera problem, but not indefinitely.
   */
  public boolean verify(String token, String caseId, String secret) {
    Optional<Map<String, String>> entry = authorized(token, secret);
    if (entry.isEmpty() || (caseId != null && !caseId.equals(entry.get().get(CASE_ID)))) {
      return false;
    }
    int sessions = ScanovateAuthenticator.parseInt(entry.get().get(SESSIONS), 0);
    if (sessions >= MAX_SESSIONS_PER_TOKEN) {
      log.warnv(
          "verify: too many {0} sessions for case {1}", service.path(), entry.get().get(CASE_ID));
      return false;
    }
    Map<String, String> updated = new HashMap<>(entry.get());
    updated.put(SESSIONS, String.valueOf(sessions + 1));
    store.put(key(token), LIFESPAN_SECONDS, updated);
    return true;
  }

  /** Records a start or result callback of the service. */
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
    updated
        .keySet()
        .removeIf(
            key ->
                key.equals(PASSED)
                    || key.equals(FACE)
                    || key.equals(READING)
                    || key.startsWith(SIDE_PREFIX));
    updated.put(STATUS, MESSAGE_START.equals(messageType) ? "session_started" : status.asText());
    if (MESSAGE_RESULT.equals(messageType)) {
      Optional<Reading> reading =
          switch (service) {
            case LIVENESS -> readLiveness(status.asText(), body);
            case OCR -> readOcr(status.asText(), body);
          };
      if (reading.isEmpty()) {
        return RecordOutcome.INVALID;
      }
      updated.put(PASSED, String.valueOf(reading.get().passed()));
      reading.get().face().ifPresent(face -> updated.put(FACE, face));
      reading.get().reading().ifPresent(text -> updated.put(READING, text));
      reading.get().sides().forEach((side, result) -> updated.put(SIDE_PREFIX + side, result));
      log.infov(
          "record: {0} session of case {1} ended with {2}, passed={3}",
          service.path(),
          ScanovateDiagnostics.caseId(entry.get().get(CASE_ID)),
          ScanovateDiagnostics.status(status.asText()),
          reading.get().passed());
    }
    store.put(key(token), LIFESPAN_SECONDS, updated);
    return RecordOutcome.RECORDED;
  }

  /**
   * Reads the result of a Liveness Plus session. It passes if the session completed and passed the
   * liveness, presentation attack and injection attack checks, and has the picture of the voter.
   *
   * @return empty if the picture is not valid base64
   */
  private static Optional<Reading> readLiveness(String status, JsonNode body) {
    JsonNode processing = body.at("/scan/processing_result");
    Optional<String> image = text(processing, "image").or(() -> text(processing, "face_image"));
    if (image.isPresent() && decode(image.get()).isEmpty()) {
      return Optional.empty();
    }
    boolean passed =
        STATUS_COMPLETED.equals(status)
            && isTrue(processing, "liveness_check_passed")
            && isTrue(processing, "presentation_attack_check_passed")
            && isTrue(processing, "injection_attack_check_passed")
            && image.isPresent();
    return Optional.of(
        new Reading(passed, passed ? image : Optional.empty(), Optional.empty(), Map.of()));
  }

  /**
   * Reads the result of an OCR session. It passes if the session completed, every side was read,
   * and the portrait of the document is there.
   *
   * @return empty if the portrait is not valid base64
   */
  private static Optional<Reading> readOcr(String status, JsonNode body) {
    Map<String, String> sides = new HashMap<>();
    Optional<String> face = Optional.empty();
    for (String side : OcrResults.SIDES) {
      JsonNode result = body.path(side).path("processing_result");
      if (!result.isObject()) {
        continue;
      }
      sides.put(side, result.path(STATUS).asText(""));
      if (face.isEmpty()) {
        face = text(result.path("images"), "face_image");
      }
    }
    if (face.isPresent() && decode(face.get()).isEmpty()) {
      return Optional.empty();
    }
    Optional<JsonNode> reading = Optional.empty();
    if (STATUS_COMPLETED.equals(status)) {
      try {
        reading = OcrResults.combine(List.of(body), LocalDate.now());
      } catch (IOException e) {
        reading = Optional.empty();
      }
    }
    boolean passed = reading.isPresent() && face.isPresent();
    return Optional.of(
        new Reading(
            passed,
            passed ? face : Optional.empty(),
            passed ? reading.map(JsonNode::toString) : Optional.empty(),
            sides));
  }

  /**
   * Waits up to the given time for the final result of the token's last session, as the browser may
   * learn that the session ended before Keycloak gets the result callback.
   */
  public Optional<SessionResult> awaitResult(String token, int timeoutSeconds) {
    long polls = timeoutSeconds * 1000L / POLL_INTERVAL_MS;
    for (long poll = 0; ; poll++) {
      Optional<SessionResult> result = finalResult(token);
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

  private Optional<SessionResult> finalResult(String token) {
    Map<String, String> entry = token == null ? null : store.get(key(token));
    if (entry == null || RUNNING.contains(entry.get(STATUS))) {
      return Optional.empty();
    }
    Optional<byte[]> face = Optional.ofNullable(entry.get(FACE)).flatMap(OnPremSessions::decode);
    Optional<JsonNode> reading =
        Optional.ofNullable(entry.get(READING)).flatMap(OnPremSessions::parse);
    boolean passed =
        Boolean.parseBoolean(entry.get(PASSED))
            && face.isPresent()
            && (service != OnPremService.OCR || reading.isPresent());
    Map<String, String> sides = new HashMap<>();
    entry.forEach(
        (key, value) -> {
          if (key.startsWith(SIDE_PREFIX)) {
            sides.put(key.substring(SIDE_PREFIX.length()), value);
          }
        });
    return Optional.of(
        new SessionResult(
            entry.get(STATUS),
            passed,
            passed ? face : Optional.empty(),
            passed ? reading : Optional.empty(),
            Map.copyOf(sides)));
  }

  private static Optional<JsonNode> parse(String json) {
    try {
      return Optional.ofNullable(MAPPER.readTree(json));
    } catch (IOException e) {
      return Optional.empty();
    }
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

  private String key(String token) {
    return "scanovate-" + service.path() + ":" + token;
  }
}
