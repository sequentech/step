// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.math.BigDecimal;
import java.text.Collator;
import java.time.LocalDate;
import java.time.format.DateTimeFormatter;
import java.time.format.DateTimeParseException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Optional;
import java.util.function.Function;
import lombok.experimental.UtilityClass;
import lombok.extern.jbosslog.JBossLog;

/**
 * Applies the per document type rules configured in the authenticator to the B-Trust results.
 *
 * <p>Both the validation and the store configurations are JSON objects keyed by document type (the
 * value of the document type auth note), each holding a list of rules. A {@code default} key is
 * used for document types without their own entry.
 */
@JBossLog
@UtilityClass
public class AttributeRules {
  public static final String DEFAULT_DOC_TYPE = "default";
  public static final String TYPE = "type";
  public static final String PROCESS = "process";
  public static final String ATTRIBUTE_PATH = "attributePath";
  public static final String ERROR_MSG = "errorMsg";
  public static final String USER_ATTRIBUTE = "UserAttribute";
  public static final String VALUE_DATE_FORMAT = "valueDateFormat";
  public static final String SOURCE_DATE_FORMAT = "sourceDateFormat";
  public static final String STORE_DATE_FORMAT = "storeDateFormat";
  public static final String NOW = "now";

  private static final ObjectMapper MAPPER = new ObjectMapper();

  /** Supported validation rules. The expected value is read from the field named as the type. */
  public enum ValidationType {
    /** The source value equals the value of the given auth note. */
    EQUAL_AUTH_NOTE("equalAuthnoteAttributeId"),
    /** The source value equals the given literal. */
    EQUAL_VALUE("equalValue"),
    /** The source value is a number greater than or equal to the given one. */
    MIN_VALUE("minValue"),
    /** The source date equals the date stored in the given auth note. */
    EQUAL_DATE_AUTH_NOTE("equalDateAuthnoteAttributeId"),
    /** The given date (or {@code now}) is strictly before the source date. */
    IS_BEFORE_DATE("isBeforeDateValue");

    private final String value;

    ValidationType(String value) {
      this.value = value;
    }

    public String value() {
      return value;
    }

    public static Optional<ValidationType> fromValue(String value) {
      return Arrays.stream(values()).filter(type -> type.value.equals(value)).findFirst();
    }
  }

  /** How an extracted value is stored. */
  public enum StoreType {
    TEXT("text"),
    DATE("date");

    private final String value;

    StoreType(String value) {
      this.value = value;
    }

    public String value() {
      return value;
    }

    public static Optional<StoreType> fromValue(String value) {
      return Arrays.stream(values()).filter(type -> type.value.equals(value)).findFirst();
    }
  }

  /**
   * Returns the rules that apply to the given document type.
   *
   * @return the rules, an empty list if the configuration is blank, or empty if the configuration
   *     has neither an entry for the document type nor a default one
   */
  public static Optional<JsonNode> rulesFor(String configuration, String docType)
      throws ScanovateException {
    if (configuration == null || configuration.isBlank()) {
      return Optional.of(MAPPER.createArrayNode());
    }
    JsonNode root;
    try {
      root = MAPPER.readTree(configuration);
    } catch (Exception e) {
      throw new ScanovateException("Invalid rules configuration", e);
    }
    JsonNode rules =
        docType != null && root.has(docType) ? root.get(docType) : root.get(DEFAULT_DOC_TYPE);
    if (rules == null) {
      return Optional.empty();
    }
    if (!rules.isArray()) {
      throw new ScanovateException("Rules for document type " + docType + " must be a list");
    }
    return Optional.of(rules);
  }

  /**
   * Validates the results against the rules.
   *
   * @return the message key of the first failing rule, or empty if all of them pass
   * @throws ScanovateException if a rule is malformed
   */
  public static Optional<String> validate(
      JsonNode response, JsonNode rules, Function<String, String> authNotes, LocalDate today)
      throws ScanovateException {
    for (JsonNode rule : rules) {
      String typeName = requiredText(rule, TYPE);
      ValidationType type =
          ValidationType.fromValue(typeName)
              .orElseThrow(() -> new ScanovateException("Unknown validation type " + typeName));
      String expected = requiredText(rule, type.value());
      String path = requiredText(rule, ATTRIBUTE_PATH);
      String process = rule.path(PROCESS).asText(null);
      String error = rule.path(ERROR_MSG).asText(ScanovateError.ATTRIBUTES.messageKey());

      Optional<String> sourceValue = ScanovateResults.resolveText(response, process, path);
      if (sourceValue.isEmpty()) {
        log.warnv("validate: no value for process={0} path={1}", process, path);
        return Optional.of(error);
      }
      if (!passes(type, rule, expected, sourceValue.get(), authNotes, today)) {
        log.warnv("validate: rule {0} failed for process={1} path={2}", type, process, path);
        return Optional.of(error);
      }
    }
    return Optional.empty();
  }

  /**
   * Extracts the values to be stored from the results.
   *
   * <p>Missing values are stored as empty strings so that the voter still gets to review them.
   *
   * @throws ScanovateException if a rule is malformed or a date cannot be converted
   */
  public static List<StoredAttribute> extract(JsonNode response, JsonNode rules)
      throws ScanovateException {
    List<StoredAttribute> stored = new ArrayList<>();
    for (JsonNode rule : rules) {
      String attribute = requiredText(rule, USER_ATTRIBUTE);
      String path = requiredText(rule, ATTRIBUTE_PATH);
      String typeName = requiredText(rule, TYPE);
      StoreType type =
          StoreType.fromValue(typeName)
              .orElseThrow(() -> new ScanovateException("Unknown store type " + typeName));
      String process = rule.path(PROCESS).asText(null);
      String sourceValue = ScanovateResults.resolveText(response, process, path).orElse("");

      String value =
          switch (type) {
            case TEXT -> sourceValue;
            case DATE -> sourceValue.isEmpty() ? "" : convertDate(rule, sourceValue);
          };
      stored.add(new StoredAttribute(attribute, value, type.value()));
    }
    return stored;
  }

  private static boolean passes(
      ValidationType type,
      JsonNode rule,
      String expected,
      String sourceValue,
      Function<String, String> authNotes,
      LocalDate today)
      throws ScanovateException {
    return switch (type) {
      case EQUAL_VALUE -> textEquals(expected, sourceValue);
      case EQUAL_AUTH_NOTE -> {
        String noteValue = authNotes.apply(expected);
        yield noteValue != null && textEquals(noteValue, sourceValue);
      }
      case MIN_VALUE -> {
        BigDecimal minimum = parseNumber(expected);
        if (minimum == null) {
          throw new ScanovateException("Invalid minimum value " + expected);
        }
        BigDecimal actual = parseNumber(sourceValue);
        yield actual != null && actual.compareTo(minimum) >= 0;
      }
      case EQUAL_DATE_AUTH_NOTE -> {
        Optional<LocalDate> noteDate =
            parseDate(authNotes.apply(expected), requiredText(rule, VALUE_DATE_FORMAT));
        Optional<LocalDate> sourceDate =
            parseDate(sourceValue, requiredText(rule, SOURCE_DATE_FORMAT));
        yield noteDate.isPresent() && noteDate.equals(sourceDate);
      }
      case IS_BEFORE_DATE -> {
        Optional<LocalDate> referenceDate =
            NOW.equalsIgnoreCase(expected)
                ? Optional.of(today)
                : parseDate(expected, requiredText(rule, VALUE_DATE_FORMAT));
        Optional<LocalDate> sourceDate =
            parseDate(sourceValue, requiredText(rule, SOURCE_DATE_FORMAT));
        yield referenceDate.isPresent()
            && sourceDate.isPresent()
            && referenceDate.get().isBefore(sourceDate.get());
      }
    };
  }

  /** Compares ignoring case, accents and surrounding whitespace. */
  private static boolean textEquals(String left, String right) {
    Collator collator = Collator.getInstance();
    collator.setDecomposition(Collator.FULL_DECOMPOSITION);
    collator.setStrength(Collator.PRIMARY);
    return collator.compare(left.trim(), right.trim()) == 0;
  }

  private static BigDecimal parseNumber(String value) {
    try {
      return new BigDecimal(value.trim());
    } catch (NumberFormatException e) {
      return null;
    }
  }

  private static Optional<LocalDate> parseDate(String value, String pattern)
      throws ScanovateException {
    if (value == null) {
      return Optional.empty();
    }
    DateTimeFormatter formatter = formatter(pattern);
    try {
      return Optional.of(LocalDate.parse(value.trim(), formatter));
    } catch (DateTimeParseException e) {
      log.warnv("parseDate: could not parse {0} with pattern {1}", value, pattern);
      return Optional.empty();
    }
  }

  private static String convertDate(JsonNode rule, String sourceValue) throws ScanovateException {
    String sourcePattern = requiredText(rule, SOURCE_DATE_FORMAT);
    DateTimeFormatter storeFormatter = formatter(requiredText(rule, STORE_DATE_FORMAT));
    return parseDate(sourceValue, sourcePattern)
        .map(storeFormatter::format)
        .orElseThrow(
            () ->
                new ScanovateException(
                    "Could not parse date " + sourceValue + " with pattern " + sourcePattern));
  }

  private static DateTimeFormatter formatter(String pattern) throws ScanovateException {
    try {
      return DateTimeFormatter.ofPattern(pattern);
    } catch (IllegalArgumentException e) {
      throw new ScanovateException("Invalid date pattern " + pattern, e);
    }
  }

  private static String requiredText(JsonNode rule, String field) throws ScanovateException {
    JsonNode value = rule.get(field);
    if (value == null || !value.isValueNode() || value.asText().isBlank()) {
      throw new ScanovateException("Rule " + rule + " is missing field " + field);
    }
    return value.asText();
  }
}
