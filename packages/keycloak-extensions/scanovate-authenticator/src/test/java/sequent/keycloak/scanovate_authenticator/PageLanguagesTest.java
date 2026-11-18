// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.util.HashMap;
import java.util.Locale;
import java.util.Map;
import java.util.Optional;
import org.junit.jupiter.api.Test;

class PageLanguagesTest {
  private static PageLanguages languages(String value) throws ScanovateException {
    Map<String, String> config = new HashMap<>();
    if (value != null) {
      config.put(ScanovateAuthenticatorFactory.PAGE_LANGUAGES, value);
    }
    return PageLanguages.fromConfig(config);
  }

  @Test
  void withoutTheSettingThePagesKeepTheirDefaultLanguage() throws ScanovateException {
    assertEquals(Optional.empty(), languages(null).of(Locale.forLanguageTag("tl")));
    assertEquals(Optional.empty(), languages("  ").of(Locale.forLanguageTag("tl")));
  }

  @Test
  void aListedLanguageIsTheOneOfThePages() throws ScanovateException {
    PageLanguages languages = languages("en, tl");

    assertEquals(Optional.of("tl"), languages.of(Locale.forLanguageTag("tl")));
    assertEquals(Optional.of("en"), languages.of(Locale.ENGLISH));
  }

  @Test
  void aLanguageThePagesDoNotHaveKeepsTheirDefault() throws ScanovateException {
    // A page asked for a language without a locale file can't start its session.
    assertEquals(Optional.empty(), languages("en,tl").of(Locale.forLanguageTag("es")));
    assertEquals(Optional.empty(), languages("en,tl").of(null));
  }

  @Test
  void aRegionalLocaleUsesItsOwnFileOrTheOneOfItsLanguage() throws ScanovateException {
    assertEquals(Optional.of("en"), languages("en,tl").of(Locale.forLanguageTag("en-PH")));
    assertEquals(Optional.of("pt-BR"), languages("pt,pt-BR").of(Locale.forLanguageTag("pt-BR")));
  }

  @Test
  void aLanguageIsTheNameOfALocaleFile() {
    assertThrows(ScanovateException.class, () -> languages("en,../tl"));
    assertThrows(ScanovateException.class, () -> languages("en,t l"));
    assertThrows(ScanovateException.class, () -> languages("en,,tl"));
  }
}
