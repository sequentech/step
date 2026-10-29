// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.idp_attribute_mapper;

import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.function.UnaryOperator;

/**
 * In-memory table that maps an input key to an output value.
 *
 * <p>Each non-empty line that does not start with {@code #} is one of:
 *
 * <ul>
 *   <li>{@code <value> = <key>, <key>, <from>..<to>}: every listed key, and every key in each
 *       range, maps to {@code <value>}.
 *   <li>{@code <key>,<value>}: one key per row, for generated tables.
 * </ul>
 *
 * <p>A range covers the keys that share the prefix of {@code <from>} and whose last run of digits
 * lies between the digits of {@code <from>} and {@code <to>}. {@code <to>} can repeat the prefix or
 * give only the digits. When both bounds have the same number of digits, a key must have that many
 * digits too, so {@code 001..420} matches {@code 007} but not {@code 7}.
 */
public final class LookupTable {

  private final Map<String, String> exact;
  private final Map<String, List<Range>> rangesByPrefix;

  private LookupTable(Map<String, String> exact, Map<String, List<Range>> rangesByPrefix) {
    this.exact = exact;
    this.rangesByPrefix = rangesByPrefix;
  }

  /** Thrown when the table text cannot be parsed; the message names the line. */
  public static final class ParseException extends IllegalArgumentException {
    ParseException(int lineNumber, String message) {
      super("Lookup table line " + lineNumber + ": " + message);
    }

    ParseException(String message) {
      super("Lookup table: " + message);
    }
  }

  private record Range(long from, long to, int width, String value) {}

  /**
   * Parses a table.
   *
   * @param text the table text
   * @param keyNormalizer applied to each key, so keys match the normalized input
   */
  public static LookupTable parse(String text, UnaryOperator<String> keyNormalizer) {
    Map<String, String> exact = new HashMap<>();
    Map<String, List<Range>> ranges = new HashMap<>();
    String[] lines = text.split("\\r?\\n");
    for (int i = 0; i < lines.length; i++) {
      int lineNumber = i + 1;
      String line = lines[i].strip();
      if (line.isEmpty() || line.startsWith("#")) {
        continue;
      }
      int equals = line.indexOf('=');
      if (equals >= 0) {
        String value = line.substring(0, equals).strip();
        if (value.isEmpty()) {
          throw new ParseException(lineNumber, "missing value before '='");
        }
        for (String item : line.substring(equals + 1).split(",")) {
          String key = item.strip();
          if (key.isEmpty()) {
            throw new ParseException(lineNumber, "empty key");
          }
          int dots = key.indexOf("..");
          if (dots >= 0) {
            addRange(
                ranges,
                keyNormalizer.apply(key.substring(0, dots).strip()),
                keyNormalizer.apply(key.substring(dots + 2).strip()),
                value,
                lineNumber);
          } else {
            putExact(exact, keyNormalizer.apply(key), value, lineNumber);
          }
        }
      } else {
        String[] fields = line.split(",", -1);
        if (fields.length != 2 || fields[0].isBlank() || fields[1].isBlank()) {
          throw new ParseException(
              lineNumber, "expected '<value> = <keys>' or '<key>,<value>', got: " + line);
        }
        putExact(exact, keyNormalizer.apply(fields[0].strip()), fields[1].strip(), lineNumber);
      }
    }
    for (Map.Entry<String, List<Range>> entry : ranges.entrySet()) {
      List<Range> list = entry.getValue();
      list.sort((a, b) -> Long.compare(a.from(), b.from()));
      for (int i = 1; i < list.size(); i++) {
        Range previous = list.get(i - 1);
        Range current = list.get(i);
        if (current.from() <= previous.to() && current.width() == previous.width()) {
          throw new ParseException("overlapping ranges for prefix '" + entry.getKey() + "'");
        }
      }
    }
    return new LookupTable(Collections.unmodifiableMap(exact), Collections.unmodifiableMap(ranges));
  }

  private static void putExact(Map<String, String> exact, String key, String value, int line) {
    String previous = exact.putIfAbsent(key, value);
    if (previous != null && !previous.equals(value)) {
      throw new ParseException(
          line, "key '" + key + "' maps to both '" + previous + "' and '" + value + "'");
    }
  }

  private static void addRange(
      Map<String, List<Range>> ranges, String from, String to, String value, int line) {
    int fromDigits = trailingDigitsStart(from);
    if (fromDigits == from.length()) {
      throw new ParseException(line, "range start '" + from + "' does not end in digits");
    }
    String prefix = from.substring(0, fromDigits);
    String fromNumber = from.substring(fromDigits);
    String toNumber;
    if (to.chars().allMatch(Character::isDigit) && !to.isEmpty()) {
      toNumber = to;
    } else if (to.startsWith(prefix) && trailingDigitsStart(to) == prefix.length()) {
      toNumber = to.substring(prefix.length());
    } else {
      throw new ParseException(
          line, "range end '" + to + "' must be digits or share the prefix '" + prefix + "'");
    }
    if (toNumber.isEmpty()) {
      throw new ParseException(line, "range end '" + to + "' does not end in digits");
    }
    long low = parseNumber(fromNumber, line);
    long high = parseNumber(toNumber, line);
    if (low > high) {
      throw new ParseException(line, "range '" + from + ".." + to + "' is reversed");
    }
    int width = fromNumber.length() == toNumber.length() ? fromNumber.length() : 0;
    ranges.computeIfAbsent(prefix, k -> new ArrayList<>()).add(new Range(low, high, width, value));
  }

  private static long parseNumber(String digits, int line) {
    if (digits.length() > 18) {
      throw new ParseException(line, "number '" + digits + "' is too long");
    }
    return Long.parseLong(digits);
  }

  private static int trailingDigitsStart(String key) {
    int i = key.length();
    while (i > 0 && Character.isDigit(key.charAt(i - 1))) {
      i--;
    }
    return i;
  }

  /** Returns the value for {@code key}, or {@code null} when no row matches. */
  public String lookup(String key) {
    String value = exact.get(key);
    if (value != null || rangesByPrefix.isEmpty()) {
      return value;
    }
    int digitsStart = trailingDigitsStart(key);
    if (digitsStart == key.length() || key.length() - digitsStart > 18) {
      return null;
    }
    List<Range> list = rangesByPrefix.get(key.substring(0, digitsStart));
    if (list == null) {
      return null;
    }
    String digits = key.substring(digitsStart);
    long number = Long.parseLong(digits);
    // Ranges are sorted by start; find the last one starting at or before the number.
    int low = 0;
    int high = list.size() - 1;
    int found = -1;
    while (low <= high) {
      int middle = (low + high) >>> 1;
      if (list.get(middle).from() <= number) {
        found = middle;
        low = middle + 1;
      } else {
        high = middle - 1;
      }
    }
    // Ranges of different widths can overlap, so check every earlier candidate.
    for (int i = found; i >= 0; i--) {
      Range range = list.get(i);
      if (number <= range.to() && (range.width() == 0 || range.width() == digits.length())) {
        return range.value();
      }
    }
    return null;
  }

  /** Number of exact keys plus number of ranges, for logging. */
  public int size() {
    return exact.size() + rangesByPrefix.values().stream().mapToInt(List::size).sum();
  }
}
