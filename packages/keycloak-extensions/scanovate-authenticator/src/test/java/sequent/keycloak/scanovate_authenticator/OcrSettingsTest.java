// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.net.URI;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

class OcrSettingsTest {
  private static Map<String, String> config(String ocrTypes) {
    Map<String, String> config = new HashMap<>();
    config.put(ScanovateAuthenticatorFactory.OCR_URL, "http://scanovate-ocr:5040/");
    if (ocrTypes != null) {
      config.put(ScanovateAuthenticatorFactory.OCR_TYPES, ocrTypes);
    }
    return config;
  }

  @Test
  void readsTheUrlAndTheOcrTypeOfTheDocumentType() throws ScanovateException {
    OcrSettings settings =
        OcrSettings.fromConfig(
            config("{\"philippinePassport\": \"passport\", \"default\": \"regula\"}"),
            "philippinePassport");

    assertEquals(new OcrSettings(URI.create("http://scanovate-ocr:5040"), "passport"), settings);
  }

  @Test
  void otherDocumentTypesUseTheDefault() throws ScanovateException {
    assertEquals(
        "regula",
        OcrSettings.fromConfig(
                config("{\"philippinePassport\": \"passport\", \"default\": \"regula\"}"),
                "driversLicense")
            .ocrType());
  }

  @Test
  void withoutConfigurationEveryDocumentIsReadAsAPassport() throws ScanovateException {
    assertEquals("passport", OcrSettings.fromConfig(config(null), "anything").ocrType());
    assertEquals("passport", OcrSettings.fromConfig(config("  "), null).ocrType());
  }

  @Test
  void documentTypesWithoutAnOcrTypeAreRejected() {
    assertThrows(
        ScanovateException.class,
        () -> OcrSettings.fromConfig(config("{\"philippinePassport\": \"passport\"}"), "iBP"));
  }

  @Test
  void malformedConfigurationIsRejected() {
    for (String ocrTypes : List.of("{not json", "[]", "{\"default\": 1}", "{\"default\": \" \"}")) {
      assertThrows(
          ScanovateException.class,
          () -> OcrSettings.fromConfig(config(ocrTypes), "philippinePassport"),
          ocrTypes);
    }
  }

  @Test
  void theUrlIsRequired() {
    Map<String, String> config = config(null);
    config.remove(ScanovateAuthenticatorFactory.OCR_URL);
    assertThrows(ScanovateException.class, () -> OcrSettings.fromConfig(config, "passport"));
  }

  /** Passports are read from an ICAO TD3 data page; other documents are ID-1 cards. */
  @Test
  void documentFormatFollowsTheOcrType() throws ScanovateException {
    Map<String, String> ocrTypes =
        config("{\"philippinePassport\": \"passport\", \"default\": \"regula\"}");
    assertEquals(
        DocumentFormat.TD3, OcrSettings.fromConfig(ocrTypes, "philippinePassport").format());
    assertEquals(DocumentFormat.ID_1, OcrSettings.fromConfig(ocrTypes, "philSysID").format());
  }
}
