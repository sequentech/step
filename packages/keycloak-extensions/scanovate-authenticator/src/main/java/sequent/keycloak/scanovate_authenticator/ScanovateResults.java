// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import com.fasterxml.jackson.core.JsonPointer;
import com.fasterxml.jackson.databind.JsonNode;
import java.util.Optional;
import lombok.experimental.UtilityClass;

/** Interprets the JSON returned by the B-Trust results endpoints. */
@UtilityClass
public class ScanovateResults {
  public static final String MOBILE_FORM_PROCESS = "mobileForm";
  static final int ERROR_CODE_MAX_TRIALS_OCR = 1020;
  static final int ERROR_CODE_DOCUMENT_AUTHENTICATION = 1026;
  static final int ERROR_CODE_MAX_TRIALS_LIVENESS = 1030;

  /**
   * Returns the error to show to the voter, if any, following the layered error model of B-Trust:
   * the top level reports whether the API call itself succeeded, and {@code data} reports the
   * outcome of the verification flow.
   */
  public static Optional<ScanovateError> outcomeError(JsonNode response) {
    if (response == null
        || !response.path("success").asBoolean(false)
        || response.path("errorCode").asInt(-1) != 0) {
      return Optional.of(ScanovateError.INTERNAL);
    }
    JsonNode data = response.path("data");
    if (!data.isObject()) {
      return Optional.of(ScanovateError.INTERNAL);
    }
    int errorCode = data.path("errorCode").asInt(0);
    switch (errorCode) {
      case 0:
        break;
      case ERROR_CODE_MAX_TRIALS_OCR:
      case ERROR_CODE_MAX_TRIALS_LIVENESS:
        return Optional.of(ScanovateError.MAX_TRIALS);
      case ERROR_CODE_DOCUMENT_AUTHENTICATION:
        return Optional.of(ScanovateError.DOCUMENT_AUTHENTICATION);
      default:
        return Optional.of(ScanovateError.VERIFICATION_FAILED);
    }
    if (!data.path("success").asBoolean(false)) {
      return Optional.of(ScanovateError.VERIFICATION_FAILED);
    }
    return Optional.empty();
  }

  /**
   * Resolves a JSON pointer against the results.
   *
   * <p>When {@code process} is given, the pointer is relative to the entry of {@code
   * data.resultsList} for that process ({@code ocr}, {@code liveness_plus}, {@code
   * biometric_match}, ... or {@code mobileForm}). Each attempt of a task produces its own entry, so
   * the last successful one is used, or the last one if none succeeded. Without {@code process},
   * the pointer is relative to the whole response.
   *
   * @return the textual value, or empty if it's missing or is not a scalar
   */
  public static Optional<String> resolveText(JsonNode response, String process, String pointer) {
    if (response == null || pointer == null) {
      return Optional.empty();
    }
    JsonPointer jsonPointer;
    try {
      jsonPointer = JsonPointer.compile(pointer);
    } catch (IllegalArgumentException e) {
      return Optional.empty();
    }
    Optional<JsonNode> root =
        (process == null || process.isBlank())
            ? Optional.of(response)
            : findProcessEntry(response, process);
    return root.map(node -> node.at(jsonPointer))
        .filter(JsonNode::isValueNode)
        .filter(node -> !node.isNull())
        .map(JsonNode::asText);
  }

  private static Optional<JsonNode> findProcessEntry(JsonNode response, String process) {
    JsonNode lastMatch = null;
    JsonNode lastSuccessfulMatch = null;
    for (JsonNode entry : response.path("data").path("resultsList")) {
      if (matchesProcess(entry, process)) {
        lastMatch = entry;
        if (entry.path("success").asBoolean(false)) {
          lastSuccessfulMatch = entry;
        }
      }
    }
    return Optional.ofNullable(lastSuccessfulMatch != null ? lastSuccessfulMatch : lastMatch);
  }

  private static boolean matchesProcess(JsonNode entry, String process) {
    if (MOBILE_FORM_PROCESS.equalsIgnoreCase(process)) {
      return entry.has(MOBILE_FORM_PROCESS);
    }
    return process.equalsIgnoreCase(entry.path("process").asText(null));
  }
}
