// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import com.fasterxml.jackson.core.JsonPointer;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.io.IOException;
import java.time.DateTimeException;
import java.time.LocalDate;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.regex.Pattern;
import lombok.experimental.UtilityClass;
import lombok.extern.jbosslog.JBossLog;

/**
 * Interprets the responses of the on-premise Scanovate OCR service, see {@link OcrClient}.
 *
 * <p>Each side of the document is read on its own. The rules of the authenticator apply to their
 * combined results, a JSON object with the fields read from the document under {@value
 * #OCR_PROCESS} and the checks of the service under {@value #AUTHENTICATIONS_PROCESS}:
 *
 * <pre>{"ocr": {"document_number": "P1234567A", ...}, "authentications": {"expiry_date_valid":
 * true, ...}}</pre>
 *
 * <p>Dates of the machine readable zone ({@code yyMMdd}) are converted to {@code yyyy-MM-dd}. The
 * images the service returns are never kept.
 */
@JBossLog
@UtilityClass
public class OcrResults {
  public static final String OCR_PROCESS = "ocr";
  public static final String AUTHENTICATIONS_PROCESS = "authentications";
  static final String COMPLETED = "completed";
  static final String SUCCESS = "success";
  static final List<String> SIDES = List.of("front", "back", "scan");
  static final List<String> PAST_DATES = List.of("date_of_birth", "date_of_issue");
  static final List<String> FUTURE_DATES = List.of("date_of_expiry");

  /** Expiry dates further in the future than this belong to the previous century. */
  static final int MAX_YEARS_TO_EXPIRY = 50;

  private static final ObjectMapper MAPPER = new ObjectMapper();
  private static final Pattern MRZ_DATE = Pattern.compile("\\d{6}");

  /**
   * Combines the responses for the sides of a document, front first. A field read from several
   * sides keeps the first value that isn't empty.
   *
   * @return the results the rules apply to, or empty if a side could not be read, such as a blurred
   *     photo or one of another kind of document
   * @throws IOException if the service could not process an image
   */
  public static Optional<JsonNode> combine(List<JsonNode> responses, LocalDate today)
      throws IOException {
    ObjectNode fields = MAPPER.createObjectNode();
    ObjectNode authentications = MAPPER.createObjectNode();
    for (JsonNode response : responses) {
      String status = response == null ? "" : response.path("status").asText("");
      if (!COMPLETED.equals(status)) {
        throw new IOException("The OCR service could not process the image: " + status);
      }
      JsonNode result = processingResult(response);
      String resultStatus = result.path("status").asText("");
      if (!SUCCESS.equals(resultStatus)) {
        log.warnv("combine: a side of the document could not be read: {0}", resultStatus);
        return Optional.empty();
      }
      merge(fields, result.path("fields"));
      merge(authentications, response.path("auth"));
    }
    PAST_DATES.forEach(field -> convertDate(fields, field, today, false));
    FUTURE_DATES.forEach(field -> convertDate(fields, field, today, true));

    ObjectNode results = MAPPER.createObjectNode();
    results.set(OCR_PROCESS, fields);
    results.set(AUTHENTICATIONS_PROCESS, authentications);
    return Optional.of(results);
  }

  /**
   * Resolves a JSON pointer against the results.
   *
   * <p>When {@code process} is given ({@value #OCR_PROCESS} or {@value #AUTHENTICATIONS_PROCESS}),
   * the pointer is relative to it. Without {@code process}, it is relative to the whole results.
   *
   * @return the textual value, or empty if it's missing or is not a scalar
   */
  public static Optional<String> resolveText(JsonNode results, String process, String pointer) {
    if (results == null || pointer == null) {
      return Optional.empty();
    }
    JsonPointer jsonPointer;
    try {
      jsonPointer = JsonPointer.compile(pointer);
    } catch (IllegalArgumentException e) {
      return Optional.empty();
    }
    JsonNode root = (process == null || process.isBlank()) ? results : results.path(process);
    JsonNode value = root.at(jsonPointer);
    return value.isValueNode() && !value.isNull() ? Optional.of(value.asText()) : Optional.empty();
  }

  private static JsonNode processingResult(JsonNode response) {
    for (String side : SIDES) {
      JsonNode result = response.path(side).path("processing_result");
      if (result.isObject()) {
        return result;
      }
    }
    return MAPPER.createObjectNode();
  }

  /** Copies the scalar values of {@code source}, unless {@code target} already has a value. */
  private static void merge(ObjectNode target, JsonNode source) {
    if (!source.isObject()) {
      return;
    }
    for (Map.Entry<String, JsonNode> entry : source.properties()) {
      JsonNode value = entry.getValue();
      if (!value.isValueNode() || value.isNull()) {
        continue;
      }
      JsonNode current = target.get(entry.getKey());
      if (current == null || (current.asText().isEmpty() && !value.asText().isEmpty())) {
        target.set(entry.getKey(), value);
      }
    }
  }

  /**
   * Converts a {@code yyMMdd} date of the machine readable zone to {@code yyyy-MM-dd}. Dates of
   * birth and issue are never in the future, and expiry dates are in this century unless they would
   * be implausibly far away.
   */
  private static void convertDate(
      ObjectNode fields, String field, LocalDate today, boolean future) {
    JsonNode value = fields.get(field);
    if (value == null || !MRZ_DATE.matcher(value.asText()).matches()) {
      return;
    }
    String text = value.asText();
    int year = Integer.parseInt(text.substring(0, 2));
    int month = Integer.parseInt(text.substring(2, 4));
    int day = Integer.parseInt(text.substring(4, 6));
    LocalDate date;
    try {
      date = LocalDate.of(2000 + year, month, day);
    } catch (DateTimeException e) {
      log.warnv("convertDate: {0} is not a valid date", field);
      return;
    }
    boolean previousCentury =
        future ? date.isAfter(today.plusYears(MAX_YEARS_TO_EXPIRY)) : date.isAfter(today);
    fields.put(field, (previousCentury ? date.minusYears(100) : date).toString());
  }
}
