// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.pin_link;

import java.time.Instant;
import java.time.format.DateTimeParseException;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.function.Function;
import java.util.regex.Pattern;
import org.jboss.logging.Logger;

/**
 * The per-realm configuration, kept in realm attributes. Anything missing or invalid leaves the
 * feature off, the same as the default. The one exception is {@link #assetOrigins}, which only
 * decides what the page may load.
 */
record PinLinkConfig(
    String pinAttribute,
    String tokenHashAttribute,
    String attemptsAttribute,
    int maxAttempts,
    Instant expiresAt) {
  static final String POLICY = "pin-link-policy";
  static final String PIN_ATTRIBUTE = "pin-link-pin-attribute";
  static final String TOKEN_HASH_ATTRIBUTE = "pin-link-token-hash-attribute";
  static final String ATTEMPTS_ATTRIBUTE = "pin-link-attempts-attribute";
  static final String MAX_ATTEMPTS = "pin-link-max-attempts";
  static final String EXPIRES_AT = "pin-link-expires-at";
  static final String ASSET_ORIGINS = "pin-link-asset-origins";

  static final String DEFAULT_PIN_ATTRIBUTE = "pin";
  static final String DEFAULT_TOKEN_HASH_ATTRIBUTE = "pin-link-token-hash";
  static final String DEFAULT_ATTEMPTS_ATTRIBUTE = "pin-link-attempts";
  static final int DEFAULT_MAX_ATTEMPTS = 5;
  static final int MAX_MAX_ATTEMPTS = 100;

  enum Policy {
    DISABLED,
    ONE_TIME_LINK
  }

  // Same grammar the voter import accepts for a column name.
  private static final Pattern ATTRIBUTE_NAME = Pattern.compile("[a-zA-Z0-9._-]{1,128}");

  private static final Pattern ORIGIN_SEPARATOR = Pattern.compile("[\\s,]+");

  // An https origin and nothing else, so an entry can't add a keyword or a directive to the policy.
  private static final Pattern ORIGIN = Pattern.compile("https://[A-Za-z0-9.-]+(:[0-9]{1,5})?");

  private static final Logger LOG = Logger.getLogger(PinLinkConfig.class);

  static Optional<PinLinkConfig> from(Map<String, String> attributes) {
    return from(attributes::get);
  }

  /**
   * Reads the link's configuration from realm attributes. It is empty, meaning the link is off,
   * unless the policy is {@code ONE_TIME_LINK}, the three attribute names are valid and distinct,
   * the attempt limit is between 1 and 100 and the expiry is a valid instant.
   */
  static Optional<PinLinkConfig> from(Function<String, String> attributes) {
    if (!Policy.ONE_TIME_LINK.name().equals(attributes.apply(POLICY))) {
      return Optional.empty();
    }
    String pin = orDefault(attributes.apply(PIN_ATTRIBUTE), DEFAULT_PIN_ATTRIBUTE);
    String hash = orDefault(attributes.apply(TOKEN_HASH_ATTRIBUTE), DEFAULT_TOKEN_HASH_ATTRIBUTE);
    String attempts = orDefault(attributes.apply(ATTEMPTS_ATTRIBUTE), DEFAULT_ATTEMPTS_ATTRIBUTE);
    if (!ATTRIBUTE_NAME.matcher(pin).matches()
        || !ATTRIBUTE_NAME.matcher(hash).matches()
        || !ATTRIBUTE_NAME.matcher(attempts).matches()
        || pin.equals(hash)
        || pin.equals(attempts)
        || hash.equals(attempts)) {
      return Optional.empty();
    }
    try {
      int maxAttempts =
          Integer.parseInt(
              orDefault(attributes.apply(MAX_ATTEMPTS), String.valueOf(DEFAULT_MAX_ATTEMPTS)));
      if (maxAttempts < 1 || maxAttempts > MAX_MAX_ATTEMPTS) {
        return Optional.empty();
      }
      // An election's link has an end: without an expiry the link stays off.
      Instant expiresAt = Instant.parse(attributes.apply(EXPIRES_AT).trim());
      return Optional.of(new PinLinkConfig(pin, hash, attempts, maxAttempts, expiresAt));
    } catch (NumberFormatException | DateTimeParseException | NullPointerException error) {
      return Optional.empty();
    }
  }

  /**
   * The origins a realm lets the page load images and fonts from, for its {@code loginCustomCss}.
   * An entry that isn't an https origin is left out with a warning. It doesn't turn the link off,
   * because Windmill doesn't read this attribute and voters may already have their links, and the
   * most a typo does here is block a logo.
   */
  static List<String> assetOrigins(Function<String, String> attributes) {
    String value = attributes.apply(ASSET_ORIGINS);
    if (value == null) {
      return List.of();
    }
    return ORIGIN_SEPARATOR
        .splitAsStream(value.trim())
        .filter(entry -> !entry.isEmpty())
        .map(entry -> entry.endsWith("/") ? entry.substring(0, entry.length() - 1) : entry)
        .filter(PinLinkConfig::isOrigin)
        .distinct()
        .toList();
  }

  private static boolean isOrigin(String entry) {
    boolean origin = ORIGIN.matcher(entry).matches();
    if (!origin) {
      LOG.warnf("pin-link: ignoring an entry of %s that is not an https origin", ASSET_ORIGINS);
    }
    return origin;
  }

  boolean isExpired(Instant now) {
    return !now.isBefore(expiresAt);
  }

  /** The trimmed value, or the fallback when it is missing or blank. */
  private static String orDefault(String value, String fallback) {
    return value == null || value.isBlank() ? fallback : value.trim();
  }
}
