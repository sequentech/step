// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static sequent.keycloak.scanovate_authenticator.TestJson.json;

import com.fasterxml.jackson.databind.JsonNode;
import java.io.IOException;
import java.time.LocalDate;
import java.util.List;
import java.util.Optional;
import org.junit.jupiter.api.Test;

class OcrResultsTest {
  private static final LocalDate TODAY = LocalDate.of(2026, 9, 26);

  /** A passport read by {@code POST /single_image_ocr}, as the service answers, without images. */
  static final String PASSPORT_RESPONSE =
      """
      {
        "status": "completed",
        "ocr_type": "passport",
        "request_id": "case-1-front",
        "front": {
          "processing_result": {
            "status": "success",
            "card_type": "MRZ",
            "fields": {
              "mrz_text": "P<PHLDELA<CRUZ<<JUAN<<<<<<<<<<<<<<<<<<<<<<<<P1234567A1PHL9001158M3101012<<<<<<<<<<<<<<04",
              "mrz_type": "TD3",
              "document_type": "P",
              "issuing_country_code": "PHL",
              "document_number": "P1234567A",
              "date_of_birth": "900115",
              "date_of_expiry": "310101",
              "nationality_code": "PHL",
              "gender": "M",
              "first_name_english": "JUAN",
              "last_name_english": "DELA CRUZ",
              "personal_number": ""
            },
            "images": {"face_image": "aGVsbG8="}
          }
        },
        "auth": {
          "template_matching_valid": true,
          "document_in_frame_valid": true,
          "expiry_date_valid": true,
          "face_size_valid": true
        }
      }
      """;

  /** The results the rules apply to, as {@link OcrResults#combine} builds them for the passport. */
  static final String SUCCESSFUL_RESULTS =
      """
      {
        "ocr": {
          "mrz_text": "P<PHLDELA<CRUZ<<JUAN<<<<<<<<<<<<<<<<<<<<<<<<P1234567A1PHL9001158M3101012<<<<<<<<<<<<<<04",
          "mrz_type": "TD3",
          "document_type": "P",
          "issuing_country_code": "PHL",
          "document_number": "P1234567A",
          "date_of_birth": "1990-01-15",
          "date_of_expiry": "2031-01-01",
          "nationality_code": "PHL",
          "gender": "M",
          "first_name_english": "JUAN",
          "last_name_english": "DELA CRUZ",
          "personal_number": ""
        },
        "authentications": {
          "template_matching_valid": true,
          "document_in_frame_valid": true,
          "expiry_date_valid": true,
          "face_size_valid": true
        }
      }
      """;

  private static String unreadable(String status) {
    return """
        {"status": "completed", "ocr_type": "passport", "request_id": "r",
         "front": {"processing_result": {"status": "%s", "card_type": "MRZ"}}}
        """
        .formatted(status);
  }

  @Test
  void combinesTheFieldsAndChecksOfTheDocument() throws IOException {
    assertEquals(
        Optional.of(json(SUCCESSFUL_RESULTS)),
        OcrResults.combine(List.of(json(PASSPORT_RESPONSE)), TODAY));
  }

  @Test
  void imagesAreNeverKept() throws IOException {
    JsonNode results = OcrResults.combine(List.of(json(PASSPORT_RESPONSE)), TODAY).orElseThrow();
    assertTrue(results.findValues("images").isEmpty());
    assertTrue(results.findValues("face_image").isEmpty());
  }

  @Test
  void theFrontWinsOverTheBackAndEmptyValuesAreFilledIn() throws IOException {
    JsonNode front =
        json(
            """
            {"status": "completed", "front": {"processing_result": {"status": "success",
             "fields": {"document_number": "N1", "first_name_english": ""}}},
             "auth": {"expiry_date_valid": true}}
            """);
    JsonNode back =
        json(
            """
            {"status": "completed", "back": {"processing_result": {"status": "success",
             "fields": {"document_number": "N2", "first_name_english": "JUAN"}}},
             "auth": {"expiry_date_valid": false, "barcode_valid": true}}
            """);

    JsonNode results = OcrResults.combine(List.of(front, back), TODAY).orElseThrow();

    assertEquals("N1", results.at("/ocr/document_number").asText());
    assertEquals("JUAN", results.at("/ocr/first_name_english").asText());
    assertTrue(results.at("/authentications/expiry_date_valid").asBoolean());
    assertTrue(results.at("/authentications/barcode_valid").asBoolean());
  }

  @Test
  void resultsWithoutAKnownSideAreRead() throws IOException {
    JsonNode scan =
        json(
            """
            {"status": "completed", "scan": {"processing_result": {"status": "success",
             "fields": {"document_number": "N1"}}}}
            """);
    assertEquals(
        "N1",
        OcrResults.combine(List.of(scan), TODAY).orElseThrow().at("/ocr/document_number").asText());
  }

  @Test
  void mrzBirthDatesAreNeverInTheFuture() throws IOException {
    assertEquals("1990-01-15", birthDate("900115"));
    assertEquals("2004-07-03", birthDate("040703"));
    assertEquals("2026-09-26", birthDate("260926"));
    assertEquals("1926-09-27", birthDate("260927"));
  }

  @Test
  void mrzIssueDatesAreNeverInTheFuture() throws IOException {
    assertEquals("2021-01-02", date("date_of_issue", "210102"));
    assertEquals("1999-12-31", date("date_of_issue", "991231"));
  }

  @Test
  void mrzExpiryDatesAreInThisCentury() throws IOException {
    assertEquals("2031-01-01", date("date_of_expiry", "310101"));
    assertEquals("2020-01-01", date("date_of_expiry", "200101"));
  }

  @Test
  void datesThatAreNotMrzDatesAreLeftAsTheyAre() throws IOException {
    assertEquals("15.01.1990", birthDate("15.01.1990"));
    assertEquals("901315", birthDate("901315"));
  }

  @Test
  void anUnreadableSideHasNoResults() throws IOException {
    for (String status :
        List.of("fail_to_recognize_mrz", "card_not_detected", "card_quality_low", "error")) {
      assertEquals(
          Optional.empty(), OcrResults.combine(List.of(json(unreadable(status))), TODAY), status);
    }
    assertEquals(
        Optional.empty(),
        OcrResults.combine(
            List.of(json(PASSPORT_RESPONSE), json(unreadable("card_not_detected"))), TODAY));
  }

  @Test
  void aResponseWithoutAnySideHasNoResults() throws IOException {
    assertEquals(
        Optional.empty(), OcrResults.combine(List.of(json("{\"status\": \"completed\"}")), TODAY));
  }

  @Test
  void imagesTheServiceCouldNotProcessAreErrors() {
    for (String status : List.of("internal error", "cannot read image", "")) {
      JsonNode response = json("{\"status\": \"%s\"}".formatted(status));
      assertThrows(IOException.class, () -> OcrResults.combine(List.of(response), TODAY), status);
    }
  }

  @Test
  void resolvesPointersWithinAProcess() {
    JsonNode results = json(SUCCESSFUL_RESULTS);
    assertEquals(
        Optional.of("P1234567A"), OcrResults.resolveText(results, "ocr", "/document_number"));
    assertEquals(
        Optional.of("true"),
        OcrResults.resolveText(results, "authentications", "/expiry_date_valid"));
    assertEquals(
        Optional.of("JUAN"), OcrResults.resolveText(results, null, "/ocr/first_name_english"));
  }

  @Test
  void missingOrNonScalarValuesDoNotResolve() {
    JsonNode results = json(SUCCESSFUL_RESULTS);
    assertEquals(Optional.empty(), OcrResults.resolveText(results, "ocr", "/missing"));
    assertEquals(Optional.empty(), OcrResults.resolveText(results, "unknown", "/document_number"));
    assertEquals(Optional.empty(), OcrResults.resolveText(results, null, "/ocr"));
    assertEquals(Optional.empty(), OcrResults.resolveText(results, "ocr", "no-slash"));
    assertEquals(Optional.empty(), OcrResults.resolveText(null, "ocr", "/document_number"));
  }

  private static String birthDate(String value) throws IOException {
    return date("date_of_birth", value);
  }

  private static String date(String field, String value) throws IOException {
    JsonNode response =
        json(
            """
            {"status": "completed", "front": {"processing_result": {"status": "success",
             "fields": {"%s": "%s"}}}}
            """
                .formatted(field, value));
    return OcrResults.combine(List.of(response), TODAY).orElseThrow().at("/ocr/" + field).asText();
  }
}
