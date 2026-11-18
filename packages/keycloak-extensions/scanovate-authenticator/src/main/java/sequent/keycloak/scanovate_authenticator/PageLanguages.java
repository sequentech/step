// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.util.Arrays;
import java.util.LinkedHashSet;
import java.util.Locale;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import java.util.regex.Pattern;

/**
 * The languages of the services' capture pages: those with a locale file ({@code
 * locales/<variant>/<language>.json}) in the configuration of both OCR and Liveness Plus.
 *
 * <p>A page asked for a language without a locale file can't create its session, so Keycloak only
 * asks for the voter's language when it is listed here; otherwise the page uses the {@code
 * translation_language} of its {@code onprem.default_parameters}.
 *
 * @param languages names of the locale files, without their extension
 */
public record PageLanguages(Set<String> languages) {
  /** The query parameter of the pages that names their language. */
  static final String PARAMETER = "translation_language";

  private static final Pattern LANGUAGE = Pattern.compile("[A-Za-z0-9]+([_-][A-Za-z0-9]+)*");

  /**
   * Reads the comma-separated languages of the authenticator configuration: none unless set.
   *
   * @throws ScanovateException if a language can't be the name of a locale file
   */
  public static PageLanguages fromConfig(Map<String, String> config) throws ScanovateException {
    String value = config.get(ScanovateAuthenticatorFactory.PAGE_LANGUAGES);
    if (value == null || value.isBlank()) {
      return new PageLanguages(Set.of());
    }
    Set<String> languages = new LinkedHashSet<>();
    for (String language : Arrays.stream(value.split(",", -1)).map(String::trim).toList()) {
      if (!LANGUAGE.matcher(language).matches()) {
        throw new ScanovateException(
            "Invalid " + ScanovateAuthenticatorFactory.PAGE_LANGUAGES + ": " + value);
      }
      languages.add(language);
    }
    return new PageLanguages(Set.copyOf(languages));
  }

  /** Whether no language is listed: the pages always use their default. */
  public boolean isEmpty() {
    return languages.isEmpty();
  }

  /**
   * The language of the pages for a voter's locale: its own locale file, or the one of its
   * language; empty when the pages have neither.
   */
  public Optional<String> of(Locale locale) {
    if (locale == null) {
      return Optional.empty();
    }
    String tag = locale.toLanguageTag();
    if (languages.contains(tag)) {
      return Optional.of(tag);
    }
    String language = locale.getLanguage();
    return languages.contains(language) ? Optional.of(language) : Optional.empty();
  }
}
