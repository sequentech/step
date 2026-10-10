// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static sequent.keycloak.scanovate_authenticator.OcrResultsTest.SUCCESSFUL_RESULTS;
import static sequent.keycloak.scanovate_authenticator.TestJson.json;

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
  private static final JsonNode SCORES =
      json("{\"ocr\": {\"score\": 0.80433, \"name\": \"JUAN\"}}");
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
    assertEquals(
        Optional.empty(), validate(rule("equalValue", "juán", "ocr", "/first_name_english")));
  }

  @Test
  void equalValueMismatchReturnsConfiguredError() throws ScanovateException {
    JsonNode rules =
        json(
            """
            [{"type": "equalValue", "equalValue": "ESP", "process": "ocr",
              "attributePath": "/nationality_code", "errorMsg": "customError"}]
            """);
    assertEquals(Optional.of("customError"), validate(rules));
  }

  @Test
  void equalValueWorksWithBooleanFields() throws ScanovateException {
    assertEquals(
        Optional.empty(),
        validate(rule("equalValue", "true", "authentications", "/expiry_date_valid")));
  }

  @Test
  void missingSourceValueFailsWithDefaultError() throws ScanovateException {
    assertEquals(
        Optional.of(ScanovateError.ATTRIBUTES.messageKey()),
        validate(rule("equalValue", "x", "ocr", "/middle_name_english")));
  }

  @Test
  void minValueAcceptsScoreAtThreshold() throws ScanovateException {
    assertEquals(
        Optional.empty(),
        AttributeRules.validate(
            SCORES, rule("minValue", "0.80433", "ocr", "/score"), NO_AUTH_NOTES, TODAY));
  }

  @Test
  void minValueRejectsLowerScore() throws ScanovateException {
    assertEquals(
        Optional.of(ScanovateError.ATTRIBUTES.messageKey()),
        AttributeRules.validate(
            SCORES, rule("minValue", "0.9", "ocr", "/score"), NO_AUTH_NOTES, TODAY));
  }

  @Test
  void minValueRejectsNonNumericSourceValue() throws ScanovateException {
    assertEquals(
        Optional.of(ScanovateError.ATTRIBUTES.messageKey()),
        validate(rule("minValue", "0.5", "ocr", "/first_name_english")));
  }

  @Test
  void minValueWithNonNumericConfigThrows() {
    assertThrows(
        ScanovateException.class,
        () -> validate(rule("minValue", "high", "ocr", "/document_number")));
  }

  @Test
  void equalAuthNoteMatches() throws ScanovateException {
    JsonNode rules = rule("equalAuthnoteAttributeId", "id-card", "ocr", "/document_number");
    assertEquals(
        Optional.empty(),
        AttributeRules.validate(RESULTS, rules, Map.of("id-card", " p1234567a ")::get, TODAY));
  }

  @Test
  void equalAuthNoteMismatchFails() throws ScanovateException {
    JsonNode rules = rule("equalAuthnoteAttributeId", "id-card", "ocr", "/document_number");
    assertEquals(
        Optional.of(ScanovateError.ATTRIBUTES.messageKey()),
        AttributeRules.validate(RESULTS, rules, Map.of("id-card", "999")::get, TODAY));
  }

  @Test
  void equalAuthNoteMissingNoteFails() throws ScanovateException {
    JsonNode rules = rule("equalAuthnoteAttributeId", "id-card", "ocr", "/document_number");
    assertEquals(Optional.of(ScanovateError.ATTRIBUTES.messageKey()), validate(rules));
  }

  @Test
  void equalDateAuthNoteComparesDifferentFormats() throws ScanovateException {
    JsonNode rules =
        json(
            """
            [{"type": "equalDateAuthnoteAttributeId", "equalDateAuthnoteAttributeId": "dateOfBirth",
              "valueDateFormat": "dd/MM/yyyy", "sourceDateFormat": "yyyy-MM-dd",
              "process": "ocr", "attributePath": "/date_of_birth"}]
            """);
    assertEquals(
        Optional.empty(),
        AttributeRules.validate(RESULTS, rules, Map.of("dateOfBirth", "15/01/1990")::get, TODAY));
    assertEquals(
        Optional.of(ScanovateError.ATTRIBUTES.messageKey()),
        AttributeRules.validate(RESULTS, rules, Map.of("dateOfBirth", "16/01/1990")::get, TODAY));
  }

  @Test
  void equalDateWithUnparseableValueFails() throws ScanovateException {
    JsonNode rules =
        json(
            """
            [{"type": "equalDateAuthnoteAttributeId", "equalDateAuthnoteAttributeId": "dateOfBirth",
              "valueDateFormat": "dd/MM/yyyy", "sourceDateFormat": "yyyy-MM-dd",
              "process": "ocr", "attributePath": "/date_of_birth"}]
            """);
    assertEquals(
        Optional.of(ScanovateError.ATTRIBUTES.messageKey()),
        AttributeRules.validate(RESULTS, rules, Map.of("dateOfBirth", "garbage")::get, TODAY));
  }

  @Test
  void isBeforeDateNowAcceptsFutureExpiry() throws ScanovateException {
    JsonNode response = json("{\"ocr\": {\"date_of_expiry\": \"2030-08-09\"}}");
    JsonNode rules = expiryRule();
    assertEquals(Optional.empty(), AttributeRules.validate(response, rules, NO_AUTH_NOTES, TODAY));
  }

  @Test
  void isBeforeDateNowRejectsExpiredDocument() throws ScanovateException {
    JsonNode response = json("{\"ocr\": {\"date_of_expiry\": \"2020-08-09\"}}");
    assertEquals(
        Optional.of(ScanovateError.ATTRIBUTES.messageKey()),
        AttributeRules.validate(response, expiryRule(), NO_AUTH_NOTES, TODAY));
  }

  @Test
  void unknownValidationTypeThrows() {
    assertThrows(
        ScanovateException.class,
        () -> validate(rule("regex", ".*", "ocr", "/first_name_english")));
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
              {"type": "equalValue", "equalValue": "JUAN", "process": "ocr", "attributePath": "/first_name_english"},
              {"type": "equalValue", "equalValue": "false", "process": "authentications",
               "attributePath": "/expiry_date_valid", "errorMsg": "expiredError"},
              {"type": "equalValue", "equalValue": "x", "process": "ocr", "attributePath": "/first_name_english",
               "errorMsg": "neverReached"}
            ]
            """);
    assertEquals(Optional.of("expiredError"), validate(rules));
  }

  @Test
  void extractStoresTextAndReformattedDates() throws ScanovateException {
    JsonNode rules =
        json(
            """
            [
              {"UserAttribute": "firstName", "process": "ocr", "attributePath": "/first_name_english", "type": "text"},
              {"UserAttribute": "dateOfBirth", "process": "ocr", "attributePath": "/date_of_birth", "type": "date",
               "sourceDateFormat": "yyyy-MM-dd", "storeDateFormat": "dd/MM/yyyy"}
            ]
            """);
    assertEquals(
        List.of(
            new StoredAttribute("firstName", "JUAN", "text"),
            new StoredAttribute("dateOfBirth", "15/01/1990", "date")),
        AttributeRules.extract(RESULTS, rules));
  }

  @Test
  void extractMissingValueStoresEmptyString() throws ScanovateException {
    JsonNode rules =
        json(
            "[{\"UserAttribute\": \"middleName\", \"process\": \"ocr\", \"attributePath\": \"/middle_name_english\", \"type\": \"text\"}]");
    assertEquals(
        List.of(new StoredAttribute("middleName", "", "text")),
        AttributeRules.extract(RESULTS, rules));
  }

  @Test
  void extractUnparseableDateThrows() {
    JsonNode rules =
        json(
            """
            [{"UserAttribute": "dateOfBirth", "process": "ocr", "attributePath": "/first_name_english", "type": "date",
              "sourceDateFormat": "yyyy-MM-dd", "storeDateFormat": "yyyy-MM-dd"}]
            """);
    assertThrows(ScanovateException.class, () -> AttributeRules.extract(RESULTS, rules));
  }

  @Test
  void extractUnknownTypeThrows() {
    JsonNode rules =
        json(
            "[{\"UserAttribute\": \"x\", \"process\": \"ocr\", \"attributePath\": \"/first_name_english\", \"type\": \"blob\"}]");
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
        [{"type": "isBeforeDateValue", "isBeforeDateValue": "now", "sourceDateFormat": "yyyy-MM-dd",
          "process": "ocr", "attributePath": "/date_of_expiry"}]
        """);
  }
}
