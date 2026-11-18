// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.util.HashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

class OcrSettingsTest {
  private static Map<String, String> config(String ocrTypes) {
    Map<String, String> config = new HashMap<>();
    // The OCR service is no longer called by Keycloak: an earlier URL is ignored.
    config.put("ocr-url", "http://scanovate-ocr:5040/");
    if (ocrTypes != null) {
      config.put(ScanovateAuthenticatorFactory.OCR_TYPES, ocrTypes);
    }
    return config;
  }

  @Test
  void readsTheOcrTypeOfTheDocumentType() throws ScanovateException {
    OcrSettings settings =
        OcrSettings.fromConfig(
            config("{\"philippinePassport\": \"passport\", \"default\": \"regula\"}"),
            "philippinePassport");

    assertEquals(new OcrSettings("passport"), settings);
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
}
