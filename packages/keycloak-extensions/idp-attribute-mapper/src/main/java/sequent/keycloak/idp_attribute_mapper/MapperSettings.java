// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.idp_attribute_mapper;

import java.util.Arrays;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.function.UnaryOperator;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import java.util.regex.PatternSyntaxException;
import org.keycloak.models.Constants;

/** Parsed and compiled mapper configuration, and the value pipeline it runs. */
public final class MapperSettings {

  public static final String SOURCES = "sources";
  public static final String EXTRACT_REGEX = "extract.regex";
  public static final String SEPARATOR = "separator";
  public static final String CASE = "case";
  public static final String VALIDATION_REGEX = "validation.regex";
  public static final String LOOKUP_TABLE = "lookup.table";
  public static final String TARGET = "target";
  public static final String TARGET_ATTRIBUTE = "target.attribute";
  public static final String ON_MISS = "on.miss";
  public static final String DEFAULT_VALUE = "default.value";
  public static final String ERROR_MESSAGE = "error.message";

  public static final String DEFAULT_SEPARATOR = ":";
  public static final String DEFAULT_ERROR_MESSAGE = "identityProviderUnexpectedErrorMessage";

  /** Source name for the SAML subject NameID. */
  public static final String SOURCE_NAME_ID = "$nameid";

  /** Source name for the brokered user id (SAML principal, OIDC {@code sub}). */
  public static final String SOURCE_BROKER_ID = "$id";

  public enum CaseMode {
    NONE,
    LOWER,
    UPPER;

    String apply(String value) {
      return switch (this) {
        case NONE -> value;
        case LOWER -> value.toLowerCase(Locale.ROOT);
        case UPPER -> value.toUpperCase(Locale.ROOT);
      };
    }
  }

  public enum Target {
    USER_ATTRIBUTE,
    USERNAME
  }

  public enum MissPolicy {
    UNSET,
    DEFAULT,
    REJECT
  }

  /** Why the pipeline produced no value. */
  public enum MissReason {
    SOURCE_MISSING,
    EXTRACT_FAILED,
    VALIDATION_FAILED,
    NOT_IN_TABLE
  }

  /** Outcome of the pipeline: a value, or the reason there is none. */
  public record Resolution(String value, MissReason missReason, String input) {
    static Resolution hit(String value, String input) {
      return new Resolution(value, null, input);
    }

    static Resolution miss(MissReason reason, String input) {
      return new Resolution(null, reason, input);
    }

    public boolean isHit() {
      return missReason == null;
    }
  }

  private final List<String> sources;
  private final Pattern extract;
  private final String separator;
  private final CaseMode caseMode;
  private final Pattern validation;
  private final LookupTable table;
  private final Target target;
  private final String targetAttribute;
  private final MissPolicy missPolicy;
  private final String defaultValue;
  private final String errorMessage;

  private MapperSettings(Map<String, String> config) {
    this.sources =
        Arrays.stream(get(config, SOURCES, "").split(Constants.CFG_DELIMITER))
            .map(String::strip)
            .filter(s -> !s.isEmpty())
            .toList();
    if (sources.isEmpty()) {
      throw new IllegalArgumentException("At least one source is required");
    }
    this.extract = compile(config.get(EXTRACT_REGEX), EXTRACT_REGEX);
    this.separator = get(config, SEPARATOR, DEFAULT_SEPARATOR);
    this.caseMode = parseEnum(CaseMode.class, config.get(CASE), CaseMode.NONE);
    this.validation = compile(config.get(VALIDATION_REGEX), VALIDATION_REGEX);
    String tableText = config.get(LOOKUP_TABLE);
    this.table =
        tableText == null || tableText.isBlank()
            ? null
            : LookupTable.parse(tableText, caseMode::apply);
    this.target = parseEnum(Target.class, config.get(TARGET), Target.USER_ATTRIBUTE);
    this.targetAttribute = config.get(TARGET_ATTRIBUTE);
    if (target == Target.USER_ATTRIBUTE && (targetAttribute == null || targetAttribute.isBlank())) {
      throw new IllegalArgumentException("Target attribute is required for target USER_ATTRIBUTE");
    }
    this.missPolicy = parseEnum(MissPolicy.class, config.get(ON_MISS), MissPolicy.UNSET);
    this.defaultValue = config.get(DEFAULT_VALUE);
    if (missPolicy == MissPolicy.DEFAULT && (defaultValue == null || defaultValue.isEmpty())) {
      throw new IllegalArgumentException("Default value is required for policy DEFAULT");
    }
    this.errorMessage = get(config, ERROR_MESSAGE, DEFAULT_ERROR_MESSAGE);
  }

  /**
   * Parses a mapper configuration.
   *
   * @throws IllegalArgumentException when the configuration is invalid
   */
  public static MapperSettings parse(Map<String, String> config) {
    return new MapperSettings(config);
  }

  private static String get(Map<String, String> config, String key, String fallback) {
    String value = config.get(key);
    return value == null || value.isEmpty() ? fallback : value;
  }

  private static Pattern compile(String regex, String key) {
    if (regex == null || regex.isEmpty()) {
      return null;
    }
    try {
      return Pattern.compile(regex);
    } catch (PatternSyntaxException e) {
      throw new IllegalArgumentException("Invalid " + key + ": " + e.getMessage(), e);
    }
  }

  private static <E extends Enum<E>> E parseEnum(Class<E> type, String value, E fallback) {
    if (value == null || value.isEmpty()) {
      return fallback;
    }
    try {
      return Enum.valueOf(type, value);
    } catch (IllegalArgumentException e) {
      throw new IllegalArgumentException("Invalid " + type.getSimpleName() + " '" + value + "'", e);
    }
  }

  /**
   * Runs extract, normalize, validate and lookup over the source values.
   *
   * @param sourceValue returns the value of a source, or {@code null} when it is absent
   */
  public Resolution resolve(UnaryOperator<String> sourceValue) {
    StringBuilder joined = new StringBuilder();
    for (String source : sources) {
      String raw = sourceValue.apply(source);
      if (raw == null || raw.isBlank()) {
        return Resolution.miss(MissReason.SOURCE_MISSING, null);
      }
      String part = raw;
      if (extract != null) {
        Matcher matcher = extract.matcher(raw);
        if (!matcher.find()) {
          return Resolution.miss(MissReason.EXTRACT_FAILED, raw);
        }
        part = matcher.groupCount() >= 1 ? matcher.group(1) : matcher.group();
        if (part == null) {
          return Resolution.miss(MissReason.EXTRACT_FAILED, raw);
        }
      }
      if (joined.length() > 0) {
        joined.append(separator);
      }
      joined.append(caseMode.apply(part.strip()));
    }
    String input = joined.toString();
    if (validation != null && !validation.matcher(input).matches()) {
      return Resolution.miss(MissReason.VALIDATION_FAILED, input);
    }
    if (table == null) {
      return Resolution.hit(input, input);
    }
    String value = table.lookup(input);
    return value == null
        ? Resolution.miss(MissReason.NOT_IN_TABLE, input)
        : Resolution.hit(value, input);
  }

  public List<String> sources() {
    return sources;
  }

  public Target target() {
    return target;
  }

  public String targetAttribute() {
    return targetAttribute;
  }

  public MissPolicy missPolicy() {
    return missPolicy;
  }

  public String defaultValue() {
    return defaultValue;
  }

  public String errorMessage() {
    return errorMessage;
  }

  public LookupTable table() {
    return table;
  }
}
