// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static sequent.keycloak.scanovate_authenticator.ScanovateResultsTest.SUCCESSFUL_RESULTS;
import static sequent.keycloak.scanovate_authenticator.ScanovateResultsTest.json;

import com.fasterxml.jackson.databind.JsonNode;
import java.time.LocalDate;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.function.Function;
import org.junit.jupiter.api.Test;

class AttributeRulesTest {
  private static final LocalDate TODAY = LocalDate.of(2026, 9, 26);
  private static final JsonNode RESULTS = json(SUCCESSFUL_RESULTS);
  private static final Function<String, String> NO_AUTH_NOTES = key -> null;

  @Test
  void rulesForReturnsDocTypeEntry() throws ScanovateException {
    Optional<JsonNode> rules =
        AttributeRules.rulesFor(
            "{\"passport\": [{\"type\": \"text\"}], \"default\": []}", "passport");
    assertEquals(1, rules.orElseThrow().size());
  }

  @Test
  void rulesForFallsBackToDefaultEntry() throws ScanovateException {
    Optional<JsonNode> rules =
        AttributeRules.rulesFor("{\"passport\": [{\"type\": \"text\"}], \"default\": []}", "other");
    assertEquals(0, rules.orElseThrow().size());
  }

  @Test
  void rulesForUnknownDocTypeWithoutDefaultIsEmpty() throws ScanovateException {
    assertTrue(AttributeRules.rulesFor("{\"passport\": []}", "other").isEmpty());
  }

  @Test
  void rulesForNullDocTypeUsesDefault() throws ScanovateException {
    assertEquals(0, AttributeRules.rulesFor("{\"default\": []}", null).orElseThrow().size());
  }

  @Test
  void rulesForBlankConfigIsEmptyList() throws ScanovateException {
    assertEquals(0, AttributeRules.rulesFor("  ", "passport").orElseThrow().size());
  }

  @Test
  void rulesForInvalidJsonThrows() {
    assertThrows(ScanovateException.class, () -> AttributeRules.rulesFor("{not json", "passport"));
  }

  @Test
  void rulesForNonArrayEntryThrows() {
    assertThrows(
        ScanovateException.class, () -> AttributeRules.rulesFor("{\"passport\": {}}", "passport"));
  }

  @Test
  void equalValueMatchesIgnoringCaseAndAccents() throws ScanovateException {
    assertEquals(Optional.empty(), validate(rule("equalValue", "juán", "ocr", "/firstName")));
  }

  @Test
  void equalValueMismatchReturnsConfiguredError() throws ScanovateException {
    JsonNode rules =
        json(
            """
            [{"type": "equalValue", "equalValue": "ESP", "process": "ocr",
              "attributePath": "/nationality/alpha3", "errorMsg": "customError"}]
            """);
    assertEquals(Optional.of("customError"), validate(rules));
  }

  @Test
  void equalValueWorksWithBooleanFields() throws ScanovateException {
    assertEquals(
        Optional.empty(),
        validate(rule("equalValue", "true", "ocr", "/authentication/verify.expiryDate")));
  }

  @Test
  void missingSourceValueFailsWithDefaultError() throws ScanovateException {
    assertEquals(
        Optional.of(ScanovateError.ATTRIBUTES.messageKey()),
        validate(rule("equalValue", "x", "ocr", "/lastName")));
  }

  @Test
  void minValueAcceptsScoreAtThreshold() throws ScanovateException {
    assertEquals(
        Optional.empty(), validate(rule("minValue", "0.80433", "biometric_match", "/score")));
  }

  @Test
  void minValueRejectsLowerScore() throws ScanovateException {
    assertEquals(
        Optional.of(ScanovateError.ATTRIBUTES.messageKey()),
        validate(rule("minValue", "0.9", "biometric_match", "/score")));
  }

  @Test
  void minValueRejectsNonNumericSourceValue() throws ScanovateException {
    assertEquals(
        Optional.of(ScanovateError.ATTRIBUTES.messageKey()),
        validate(rule("minValue", "0.5", "ocr", "/firstName")));
  }

  @Test
  void minValueWithNonNumericConfigThrows() {
    assertThrows(
        ScanovateException.class,
        () -> validate(rule("minValue", "high", "biometric_match", "/score")));
  }

  @Test
  void equalAuthNoteMatches() throws ScanovateException {
    JsonNode rules = rule("equalAuthnoteAttributeId", "id-card", "ocr", "/idNumber");
    assertEquals(
        Optional.empty(),
        AttributeRules.validate(RESULTS, rules, Map.of("id-card", " 123456789 ")::get, TODAY));
  }

  @Test
  void equalAuthNoteMismatchFails() throws ScanovateException {
    JsonNode rules = rule("equalAuthnoteAttributeId", "id-card", "ocr", "/idNumber");
    assertEquals(
        Optional.of(ScanovateError.ATTRIBUTES.messageKey()),
        AttributeRules.validate(RESULTS, rules, Map.of("id-card", "999")::get, TODAY));
  }

  @Test
  void equalAuthNoteMissingNoteFails() throws ScanovateException {
    JsonNode rules = rule("equalAuthnoteAttributeId", "id-card", "ocr", "/idNumber");
    assertEquals(Optional.of(ScanovateError.ATTRIBUTES.messageKey()), validate(rules));
  }

  @Test
  void equalDateAuthNoteComparesDifferentFormats() throws ScanovateException {
    JsonNode rules =
        json(
            """
            [{"type": "equalDateAuthnoteAttributeId", "equalDateAuthnoteAttributeId": "dateOfBirth",
              "valueDateFormat": "yyyy-MM-dd", "sourceDateFormat": "dd.MM.yyyy",
              "process": "ocr", "attributePath": "/dob"}]
            """);
    assertEquals(
        Optional.empty(),
        AttributeRules.validate(RESULTS, rules, Map.of("dateOfBirth", "2004-07-03")::get, TODAY));
    assertEquals(
        Optional.of(ScanovateError.ATTRIBUTES.messageKey()),
        AttributeRules.validate(RESULTS, rules, Map.of("dateOfBirth", "2004-07-04")::get, TODAY));
  }

  @Test
  void equalDateWithUnparseableValueFails() throws ScanovateException {
    JsonNode rules =
        json(
            """
            [{"type": "equalDateAuthnoteAttributeId", "equalDateAuthnoteAttributeId": "dateOfBirth",
              "valueDateFormat": "yyyy-MM-dd", "sourceDateFormat": "dd.MM.yyyy",
              "process": "ocr", "attributePath": "/dob"}]
            """);
    assertEquals(
        Optional.of(ScanovateError.ATTRIBUTES.messageKey()),
        AttributeRules.validate(RESULTS, rules, Map.of("dateOfBirth", "garbage")::get, TODAY));
  }

  @Test
  void isBeforeDateNowAcceptsFutureExpiry() throws ScanovateException {
    JsonNode response =
        json(
            "{\"data\": {\"resultsList\": [{\"process\": \"ocr\", \"success\": true, \"expiryDate\": \"09.08.2030\"}]}}");
    JsonNode rules = expiryRule();
    assertEquals(Optional.empty(), AttributeRules.validate(response, rules, NO_AUTH_NOTES, TODAY));
  }

  @Test
  void isBeforeDateNowRejectsExpiredDocument() throws ScanovateException {
    JsonNode response =
        json(
            "{\"data\": {\"resultsList\": [{\"process\": \"ocr\", \"success\": true, \"expiryDate\": \"09.08.2020\"}]}}");
    assertEquals(
        Optional.of(ScanovateError.ATTRIBUTES.messageKey()),
        AttributeRules.validate(response, expiryRule(), NO_AUTH_NOTES, TODAY));
  }

  @Test
  void unknownValidationTypeThrows() {
    assertThrows(
        ScanovateException.class, () -> validate(rule("regex", ".*", "ocr", "/firstName")));
  }

  @Test
  void ruleWithoutPathThrows() {
    assertThrows(
        ScanovateException.class,
        () -> validate(json("[{\"type\": \"equalValue\", \"equalValue\": \"x\"}]")));
  }

  @Test
  void ruleWithoutExpectedValueThrows() {
    assertThrows(
        ScanovateException.class,
        () -> validate(json("[{\"type\": \"equalValue\", \"attributePath\": \"/data\"}]")));
  }

  @Test
  void firstFailingRuleWins() throws ScanovateException {
    JsonNode rules =
        json(
            """
            [
              {"type": "equalValue", "equalValue": "JUAN", "process": "ocr", "attributePath": "/firstName"},
              {"type": "minValue", "minValue": "0.99", "process": "liveness_plus", "attributePath": "/score",
               "errorMsg": "scoringError"},
              {"type": "equalValue", "equalValue": "x", "process": "ocr", "attributePath": "/firstName",
               "errorMsg": "neverReached"}
            ]
            """);
    assertEquals(Optional.of("scoringError"), validate(rules));
  }

  @Test
  void extractStoresTextAndReformattedDates() throws ScanovateException {
    JsonNode rules =
        json(
            """
            [
              {"UserAttribute": "firstName", "process": "ocr", "attributePath": "/firstName", "type": "text"},
              {"UserAttribute": "dateOfBirth", "process": "ocr", "attributePath": "/dob", "type": "date",
               "sourceDateFormat": "dd.MM.yyyy", "storeDateFormat": "yyyy-MM-dd"}
            ]
            """);
    assertEquals(
        List.of(
            new StoredAttribute("firstName", "JUAN", "text"),
            new StoredAttribute("dateOfBirth", "2004-07-03", "date")),
        AttributeRules.extract(RESULTS, rules));
  }

  @Test
  void extractMissingValueStoresEmptyString() throws ScanovateException {
    JsonNode rules =
        json(
            "[{\"UserAttribute\": \"lastName\", \"process\": \"ocr\", \"attributePath\": \"/lastName\", \"type\": \"text\"}]");
    assertEquals(
        List.of(new StoredAttribute("lastName", "", "text")),
        AttributeRules.extract(RESULTS, rules));
  }

  @Test
  void extractUnparseableDateThrows() {
    JsonNode rules =
        json(
            """
            [{"UserAttribute": "dateOfBirth", "process": "ocr", "attributePath": "/firstName", "type": "date",
              "sourceDateFormat": "dd.MM.yyyy", "storeDateFormat": "yyyy-MM-dd"}]
            """);
    assertThrows(ScanovateException.class, () -> AttributeRules.extract(RESULTS, rules));
  }

  @Test
  void extractUnknownTypeThrows() {
    JsonNode rules =
        json(
            "[{\"UserAttribute\": \"x\", \"process\": \"ocr\", \"attributePath\": \"/firstName\", \"type\": \"blob\"}]");
    assertThrows(ScanovateException.class, () -> AttributeRules.extract(RESULTS, rules));
  }

  @Test
  void extractWithoutUserAttributeThrows() {
    JsonNode rules = json("[{\"attributePath\": \"/data\", \"type\": \"text\"}]");
    assertThrows(ScanovateException.class, () -> AttributeRules.extract(RESULTS, rules));
  }

  private static Optional<String> validate(JsonNode rules) throws ScanovateException {
    return AttributeRules.validate(RESULTS, rules, NO_AUTH_NOTES, TODAY);
  }

  private static JsonNode rule(String type, String expected, String process, String path) {
    return json(
        String.format(
            "[{\"type\": \"%s\", \"%s\": \"%s\", \"process\": \"%s\", \"attributePath\": \"%s\"}]",
            type, type, expected, process, path));
  }

  private static JsonNode expiryRule() {
    return json(
        """
        [{"type": "isBeforeDateValue", "isBeforeDateValue": "now", "sourceDateFormat": "dd.MM.yyyy",
          "process": "ocr", "attributePath": "/expiryDate"}]
        """);
  }
}
