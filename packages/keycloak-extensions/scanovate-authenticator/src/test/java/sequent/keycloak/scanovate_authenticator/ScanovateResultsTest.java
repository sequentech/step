// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.util.Optional;
import org.junit.jupiter.api.Test;

class ScanovateResultsTest {
  private static final ObjectMapper MAPPER = new ObjectMapper();

  static JsonNode json(String value) {
    try {
      return MAPPER.readTree(value);
    } catch (Exception e) {
      throw new IllegalArgumentException(e);
    }
  }

  static final String SUCCESSFUL_RESULTS =
      """
      {
        "success": true,
        "errorCode": 0,
        "data": {
          "success": true,
          "errorMessage": "",
          "errorCode": 0,
          "resultsList": [
            {
              "process": "ocr",
              "success": false,
              "message": "user pressed close",
              "count": 1
            },
            {
              "process": "ocr",
              "success": true,
              "count": 2,
              "firstName": "JUAN",
              "idNumber": "123456789",
              "dob": "03.07.2004",
              "authentication": {"verify.expiryDate": true},
              "nationality": {"alpha3": "PHL"}
            },
            {
              "process": "liveness_plus",
              "success": true,
              "score": 0.97
            },
            {
              "process": "biometric_match",
              "success": true,
              "score": 0.80433
            },
            {
              "mobileForm": {"formId": "f1", "formName": "extra"},
              "success": true
            }
          ]
        }
      }
      """;

  @Test
  void successfulResultsHaveNoError() {
    assertEquals(Optional.empty(), ScanovateResults.outcomeError(json(SUCCESSFUL_RESULTS)));
  }

  @Test
  void apiLevelFailureIsInternalError() {
    assertEquals(
        Optional.of(ScanovateError.INTERNAL),
        ScanovateResults.outcomeError(json("{\"success\": false, \"errorCode\": 500}")));
  }

  @Test
  void missingDataIsInternalError() {
    assertEquals(
        Optional.of(ScanovateError.INTERNAL),
        ScanovateResults.outcomeError(json("{\"success\": true, \"errorCode\": 0}")));
  }

  @Test
  void nullResponseIsInternalError() {
    assertEquals(Optional.of(ScanovateError.INTERNAL), ScanovateResults.outcomeError(null));
  }

  @Test
  void maximumOcrTrialsMapsToMaxTrials() {
    assertEquals(
        Optional.of(ScanovateError.MAX_TRIALS), ScanovateResults.outcomeError(flowFailure(1020)));
  }

  @Test
  void maximumLivenessTrialsMapsToMaxTrials() {
    assertEquals(
        Optional.of(ScanovateError.MAX_TRIALS), ScanovateResults.outcomeError(flowFailure(1030)));
  }

  @Test
  void documentAuthenticationFailureIsReported() {
    assertEquals(
        Optional.of(ScanovateError.DOCUMENT_AUTHENTICATION),
        ScanovateResults.outcomeError(flowFailure(1026)));
  }

  @Test
  void configurableConditionFailureIsVerificationFailure() {
    assertEquals(
        Optional.of(ScanovateError.VERIFICATION_FAILED),
        ScanovateResults.outcomeError(flowFailure(-1)));
  }

  @Test
  void unsuccessfulFlowWithoutErrorCodeIsVerificationFailure() {
    JsonNode response =
        json(
            "{\"success\": true, \"errorCode\": 0, \"data\": {\"success\": false, \"errorCode\": 0}}");
    assertEquals(
        Optional.of(ScanovateError.VERIFICATION_FAILED), ScanovateResults.outcomeError(response));
  }

  @Test
  void resolvePrefersLastSuccessfulAttemptOfProcess() {
    assertEquals(
        Optional.of("JUAN"),
        ScanovateResults.resolveText(json(SUCCESSFUL_RESULTS), "ocr", "/firstName"));
  }

  @Test
  void resolveMatchesProcessIgnoringCase() {
    assertEquals(
        Optional.of("0.80433"),
        ScanovateResults.resolveText(json(SUCCESSFUL_RESULTS), "BIOMETRIC_MATCH", "/score"));
  }

  @Test
  void resolveSupportsKeysContainingDots() {
    assertEquals(
        Optional.of("true"),
        ScanovateResults.resolveText(
            json(SUCCESSFUL_RESULTS), "ocr", "/authentication/verify.expiryDate"));
  }

  @Test
  void resolveSupportsMobileFormEntries() {
    assertEquals(
        Optional.of("extra"),
        ScanovateResults.resolveText(
            json(SUCCESSFUL_RESULTS), "mobileForm", "/mobileForm/formName"));
  }

  @Test
  void resolveWithoutProcessUsesWholeResponse() {
    assertEquals(
        Optional.of("0"),
        ScanovateResults.resolveText(json(SUCCESSFUL_RESULTS), null, "/data/errorCode"));
  }

  @Test
  void resolveFallsBackToLastAttemptWhenNoneSucceeded() {
    JsonNode response =
        json(
            """
            {"data": {"resultsList": [
              {"process": "liveness_plus", "success": false, "message": "camera not found"},
              {"process": "liveness_plus", "success": false, "message": "user left page"}
            ]}}
            """);
    assertEquals(
        Optional.of("user left page"),
        ScanovateResults.resolveText(response, "liveness_plus", "/message"));
  }

  @Test
  void resolveMissingProcessIsEmpty() {
    assertTrue(
        ScanovateResults.resolveText(json(SUCCESSFUL_RESULTS), "STT", "/sttScore").isEmpty());
  }

  @Test
  void resolveMissingFieldIsEmpty() {
    assertTrue(
        ScanovateResults.resolveText(json(SUCCESSFUL_RESULTS), "ocr", "/lastName").isEmpty());
  }

  @Test
  void resolveObjectNodeIsEmpty() {
    assertTrue(
        ScanovateResults.resolveText(json(SUCCESSFUL_RESULTS), "ocr", "/nationality").isEmpty());
  }

  @Test
  void resolveInvalidPointerIsEmpty() {
    assertTrue(
        ScanovateResults.resolveText(json(SUCCESSFUL_RESULTS), "ocr", "firstName").isEmpty());
  }

  private static JsonNode flowFailure(int errorCode) {
    return json(
        String.format(
            "{\"success\": true, \"errorCode\": 0, \"data\": {\"success\": false, \"errorCode\": %d}}",
            errorCode));
  }
}
