// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.authenticator.diagnostics;

import java.util.LinkedHashMap;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.regex.Pattern;

/** Bounded, non-secret fields for the existing signed event log, never a vendor payload. */
public final class ScanovateDiagnostics {
  public static final String PREFIX = "scanovate_";
  public static final String ATTEMPT_NOTE = "scanovate-diagnostic-attempt";
  public static final String EVENT_ERROR = "scanovate_verification_failed";
  public static final String ENTERED_NOTE = "scanovate-diagnostic-entered";
  public static final Set<String> ENTERED_FIELDS =
      Set.of(
          "entered_email_state",
          "entered_phone_state",
          "entered_document_number_state",
          "entered_document_type",
          "entered_embassy",
          "entered_country");

  public enum ServiceError {
    HTTP_ERROR,
    TIMEOUT,
    CONNECTION,
    TLS,
    TRANSPORT,
    MALFORMED_RESPONSE,
    PROVIDER_RESPONSE,
    INTERRUPTED;

    public String value() {
      return name().toLowerCase(Locale.ROOT);
    }
  }

  public enum ConfigComponent {
    CAPTURE,
    LIVENESS,
    FACE_MATCH,
    OCR,
    VALIDATION_RULES,
    STORAGE_RULES;

    public String value() {
      return name().toLowerCase(Locale.ROOT);
    }
  }

  public static final String MISSING = "missing";
  public static final String REDACTED = "unrecognized_redacted";
  public static final Set<String> CHECKS =
      Set.of(
          "template_matching_valid",
          "document_in_frame_valid",
          "expiry_date_valid",
          "face_size_valid",
          "barcode_valid");
  private static final Set<String> STAGES =
      Set.of(
          "configuration",
          "capture",
          "liveness",
          "face_document",
          "face_holding",
          "ocr",
          "validation");
  private static final Set<String> STATUSES =
      Set.of(
          "completed",
          "success",
          "pending",
          "session_started",
          "expired",
          "aborted",
          "client_error",
          "card_not_detected",
          "fail_to_recognize_mrz",
          "failed",
          "error",
          "unknown");
  private static final Set<String> PRESENCE = Set.of(MISSING, "present_redacted", "unknown");
  private static final Set<String> CHECK_VALUES = Set.of(MISSING, "true", "false", "invalid");
  private static final Pattern UUID =
      Pattern.compile("[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}");
  private static final Pattern SELECTION = Pattern.compile("[\\p{L}\\p{N} ._()'/-]{1,80}");
  private static final Pattern NUMBER = Pattern.compile("[0-9]{5}");
  private static final Set<String> SELECTION_FIELDS =
      Set.of("entered_document_type", "entered_embassy", "entered_country");
  private static final Set<String> PRESENCE_FIELDS =
      Set.of(
          "entered_email_state",
          "entered_phone_state",
          "entered_document_number_state",
          "extracted_document_number_state");
  private static final Set<String> REASONS =
      Set.of(
          "scanovateInternalError",
          "scanovateVerificationFailedError",
          "scanovateDocumentAuthenticationError",
          "scanovateDocumentUnreadableError",
          "scanovateAttributesError",
          "scanovateScoringError",
          "scanovateMaxRetriesError",
          "scanovateCaptureInvalidError",
          "scanovateLivenessError",
          "scanovateFaceMismatchError",
          "scanovateFaceNotFoundError");

  private ScanovateDiagnostics() {}

  public static String presence(String value) {
    return value == null || value.isBlank() ? MISSING : "present_redacted";
  }

  /** Only a bounded value actually offered by the realm may be recorded as an entered choice. */
  public static String selection(String value, Set<String> options) {
    if (value == null || value.isBlank()) return MISSING;
    return options.contains(value) && safeSelection(value) ? value : REDACTED;
  }

  private static boolean safeSelection(String value) {
    return SELECTION.matcher(value).matches() && !NUMBER.matcher(value).find();
  }

  public static String status(String value) {
    return value != null && STATUSES.contains(value) ? value : "unknown";
  }

  public static String country(String value) {
    if (value == null || value.isBlank()) return MISSING;
    return value.matches("[A-Z]{3}") ? value : REDACTED;
  }

  public static String documentType(String value) {
    if (value == null || value.isBlank()) return MISSING;
    return Set.of("P", "I", "A", "C", "V", "ID", "TD1", "TD2", "TD3").contains(value)
        ? value
        : REDACTED;
  }

  public static String score(double value) {
    return Double.isFinite(value) && value >= 0 && value <= 1
        ? String.format(Locale.ROOT, "%.4f", value)
        : "invalid";
  }

  public static String check(Object value) {
    return value == null ? MISSING : value instanceof Boolean ? value.toString() : "invalid";
  }

  /** A custom rule's display text is not a safe diagnostic failure code. */
  public static String failure(String value) {
    return value != null && REASONS.contains(value) ? value : "scanovateAttributesError";
  }

  public static String caseId(String value) {
    return value != null && UUID.matcher(value).matches() ? value : "unavailable";
  }

  /** Strip inherited form/session details as well as unknown diagnostic fields before emission. */
  public static Map<String, String> filter(Map<String, String> details) {
    Map<String, String> result = new LinkedHashMap<>();
    if (details == null) return result;
    details.forEach(
        (key, value) -> {
          if (key != null
              && key.startsWith(PREFIX)
              && value != null
              && value.length() <= 80
              && allowed(key.substring(PREFIX.length()), value)) {
            result.put(key, value);
          }
        });
    return result;
  }

  private static boolean allowed(String key, String value) {
    if (SELECTION_FIELDS.contains(key)) return safeSelection(value);
    if (PRESENCE_FIELDS.contains(key)) return PRESENCE.contains(value);
    if (key.equals("case_id") || key.equals("attempt_id")) return UUID.matcher(value).matches();
    if (key.equals("service_error"))
      return java.util.Arrays.stream(ServiceError.values())
          .anyMatch(kind -> kind.value().equals(value));
    if (key.equals("config_component"))
      return java.util.Arrays.stream(ConfigComponent.values())
          .anyMatch(kind -> kind.value().equals(value));
    if (key.equals("http_status")) return value.matches("[1-5][0-9]{2}");
    if (key.equals("occurred_at")) {
      try {
        return java.time.Instant.parse(value).toString().equals(value);
      } catch (java.time.DateTimeException ignored) {
        return false;
      }
    }
    if (key.equals("stage")) return STAGES.contains(value);
    if (key.equals("error") || key.equals("failure_reason")) return REASONS.contains(value);
    if (key.equals("can_retry")) return Set.of("true", "false").contains(value);
    if (key.equals("failed_attempts")) return value.matches("[0-9]{1,4}");
    if (key.equals("liveness_status")) return STATUSES.contains(value);
    if (key.equals("liveness_passed")) return CHECK_VALUES.contains(value);
    if (key.equals("extracted_issuing_country") || key.equals("extracted_nationality")) {
      return country(value).equals(value) || Set.of(MISSING, REDACTED).contains(value);
    }
    if (key.equals("extracted_document_type")) {
      return documentType(value).equals(value) || Set.of(MISSING, REDACTED).contains(value);
    }
    if (key.equals("face_match_document") || key.equals("face_match_holding")) {
      return value.equals("invalid") || value.matches("(?:0\\.[0-9]{4}|1\\.0000)");
    }
    for (String side : Set.of("front", "back", "scan")) {
      if (key.equals("ocr_" + side + "_status") || key.equals("ocr_" + side + "_result")) {
        return STATUSES.contains(value);
      }
    }
    return key.startsWith("check_")
        && CHECKS.contains(key.substring(6))
        && CHECK_VALUES.contains(value);
  }
}
