// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.util.HashMap;
import java.util.Map;
import java.util.Set;

/** Dependency-free privacy contract, also run by the JUnit suite. All inputs are synthetic. */
final class DiagnosticContract {
  public static void main(String[] args) {
    verify();
    System.out.println("Scanovate diagnostic privacy contract passed");
  }

  static void verify() {
    equal("missing", ScanovateDiagnostics.presence(null));
    equal("missing", ScanovateDiagnostics.presence("  "));
    equal("present_redacted", ScanovateDiagnostics.presence("person@example.invalid"));
    equal("present_redacted", ScanovateDiagnostics.presence("+12025550123"));
    equal("passport", ScanovateDiagnostics.selection("passport", Set.of("passport")));
    equal(
        "unrecognized_redacted",
        ScanovateDiagnostics.selection("not configured", Set.of("passport")));
    equal(
        "unrecognized_redacted",
        ScanovateDiagnostics.selection("a".repeat(200), Set.of("a".repeat(200))));
    equal(
        "unrecognized_redacted",
        ScanovateDiagnostics.selection(
            "https://example.invalid/?secret=x", Set.of("https://example.invalid/?secret=x")));
    equal(
        "unrecognized_redacted",
        ScanovateDiagnostics.selection("person@example.invalid", Set.of("person@example.invalid")));
    equal(
        "unrecognized_redacted",
        ScanovateDiagnostics.selection("P123456789", Set.of("P123456789")));
    equal("unknown", ScanovateDiagnostics.status("completed\nsecret=synthetic"));
    equal("completed", ScanovateDiagnostics.status("completed"));
    equal("unknown", ScanovateDiagnostics.status(null));
    equal("PHL", ScanovateDiagnostics.country("PHL"));
    equal("unrecognized_redacted", ScanovateDiagnostics.country("PHL-secret"));
    equal("missing", ScanovateDiagnostics.country(null));
    equal("0.5000", ScanovateDiagnostics.score(0.5));
    equal("invalid", ScanovateDiagnostics.score(Double.NaN));
    equal("invalid", ScanovateDiagnostics.score(Double.POSITIVE_INFINITY));
    equal("invalid", ScanovateDiagnostics.score(-0.1));
    equal("invalid", ScanovateDiagnostics.score(1.1));
    equal("missing", ScanovateDiagnostics.check(null));
    equal("invalid", ScanovateDiagnostics.check("true"));
    equal("false", ScanovateDiagnostics.check(false));
    equal("scanovateAttributesError", ScanovateDiagnostics.failure("custom-secret-error"));
    equal("scanovateLivenessError", ScanovateDiagnostics.failure("scanovateLivenessError"));
    Map<String, String> input = new HashMap<>();
    input.put("session_id", "synthetic-session");
    input.put("email", "person@example.invalid");
    input.put("scanovate_raw_response", "synthetic-secret");
    input.put("scanovate_stage", "ocr");
    input.put("scanovate_error", "scanovateLivenessError");
    input.put("scanovate_case_id", "00000000-0000-4000-8000-000000000001");
    input.put("scanovate_entered_email", "person@example.invalid");
    input.put("scanovate_extracted_document_number", "P123456789");
    input.put("scanovate_extracted_issuing_country", "PHL");
    input.put("scanovate_check_expiry_date_valid", "true");
    Map<String, String> expected =
        Map.of(
            "scanovate_stage", "ocr",
            "scanovate_error", "scanovateLivenessError",
            "scanovate_case_id", "00000000-0000-4000-8000-000000000001",
            "scanovate_extracted_issuing_country", "PHL",
            "scanovate_check_expiry_date_valid", "true");
    equal(expected, ScanovateDiagnostics.filter(input));
    input.put("scanovate_stage", "ocr?secret=synthetic");
    input.put("scanovate_case_id", "https://example.invalid/?token=synthetic");
    input.put("scanovate_error", "a".repeat(10000));
    Map<String, String> filtered = ScanovateDiagnostics.filter(input);
    equal(false, filtered.containsKey("scanovate_stage"));
    equal(false, filtered.containsKey("scanovate_case_id"));
    equal(false, filtered.containsKey("scanovate_error"));
    equal(Map.of(), ScanovateDiagnostics.filter(null));
  }

  private static void equal(Object expected, Object actual) {
    if (!expected.equals(actual)) {
      throw new AssertionError("Expected " + expected + ", got " + actual);
    }
  }
}
