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

class DocumentLivenessSettingsTest {
  private static Map<String, String> config() {
    Map<String, String> config = new HashMap<>();
    config.put(
        ScanovateAuthenticatorFactory.DOCUMENT_LIVENESS_URL, "http://document-liveness:8080/");
    return config;
  }

  @Test
  void defaultsRunEveryCheckAsTheGuideRecommends() throws ScanovateException {
    DocumentLivenessSettings settings = DocumentLivenessSettings.fromConfig(config());

    assertEquals(URI.create("http://document-liveness:8080"), settings.url());
    assertEquals(
        List.of("default-sr", "default-pc", "default-ps", "default-dm"), settings.pipelines());
    assertEquals("REGULAR", settings.calibration());
    assertEquals(0.5, settings.minProbability());
  }

  @Test
  void readsTheConfiguredChecks() throws ScanovateException {
    Map<String, String> config = config();
    config.put(
        ScanovateAuthenticatorFactory.DOCUMENT_LIVENESS_PIPELINES, " default-sr ,default-sr");
    config.put(ScanovateAuthenticatorFactory.DOCUMENT_LIVENESS_CALIBRATION, "HARD");
    config.put(ScanovateAuthenticatorFactory.DOCUMENT_LIVENESS_MIN_PROBABILITY, "0.7");

    DocumentLivenessSettings settings = DocumentLivenessSettings.fromConfig(config);

    assertEquals(List.of("default-sr"), settings.pipelines());
    assertEquals("HARD", settings.calibration());
    assertEquals(0.7, settings.minProbability());
  }

  @Test
  void malformedSettingsAreRejected() {
    List<Map.Entry<String, String>> broken =
        List.of(
            Map.entry(ScanovateAuthenticatorFactory.DOCUMENT_LIVENESS_URL, ""),
            Map.entry(ScanovateAuthenticatorFactory.DOCUMENT_LIVENESS_URL, "document-liveness"),
            Map.entry(ScanovateAuthenticatorFactory.DOCUMENT_LIVENESS_PIPELINES, " , "),
            Map.entry(ScanovateAuthenticatorFactory.DOCUMENT_LIVENESS_PIPELINES, "Default SR"),
            Map.entry(ScanovateAuthenticatorFactory.DOCUMENT_LIVENESS_CALIBRATION, "LOW"),
            Map.entry(ScanovateAuthenticatorFactory.DOCUMENT_LIVENESS_MIN_PROBABILITY, "high"),
            Map.entry(ScanovateAuthenticatorFactory.DOCUMENT_LIVENESS_MIN_PROBABILITY, "0"),
            Map.entry(ScanovateAuthenticatorFactory.DOCUMENT_LIVENESS_MIN_PROBABILITY, "1.5"));
    for (Map.Entry<String, String> setting : broken) {
      Map<String, String> config = config();
      config.put(setting.getKey(), setting.getValue());
      assertThrows(
          ScanovateException.class,
          () -> DocumentLivenessSettings.fromConfig(config),
          setting.toString());
    }
  }
}
