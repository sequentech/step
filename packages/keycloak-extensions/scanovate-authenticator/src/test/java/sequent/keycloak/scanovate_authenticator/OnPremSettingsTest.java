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

class OnPremSettingsTest {
  static Map<String, String> livenessConfig() {
    Map<String, String> config = new HashMap<>();
    config.put(ScanovateAuthenticatorFactory.LIVENESS_URL, "https://liveness.example.com:8443/");
    config.put(ScanovateAuthenticatorFactory.LIVENESS_SECRET, "callback-secret");
    config.put(ScanovateAuthenticatorFactory.FACE_MATCH_URL, "http://face-match:3000");
    config.put(
        ScanovateAuthenticatorFactory.DOCUMENT_LIVENESS_URL, "http://document-liveness:8080");
    return config;
  }

  @Test
  void defaultsApplyWhenUnset() throws ScanovateException {
    OnPremSettings settings = OnPremSettings.fromConfig(livenessConfig());

    assertEquals("callback-secret", settings.secret());
    assertEquals(
        ScanovateAuthenticatorFactory.DEFAULT_LIVENESS_RESULT_WAIT_SECONDS,
        settings.resultWaitSeconds());
  }

  @Test
  void pagesAreServedUnderTheBaseUrlWithTheirTokenAndCase() throws ScanovateException {
    OnPremSettings settings = OnPremSettings.fromConfig(livenessConfig());

    assertEquals(
        "https://liveness.example.com:8443/liveness/?case_id=case-1&token=t_0-k",
        settings.pageUrl(OnPremService.LIVENESS, "case-1", "t_0-k", Map.of()));
    assertEquals(
        "https://liveness.example.com:8443/ocr/?ocr_type=passport&case_id=case%201&token=a%2Bb",
        settings.pageUrl(OnPremService.OCR, "case 1", "a+b", Map.of("ocr_type", "passport")));
  }

  @Test
  void theOriginOfThePagesIsTheSchemeAndAuthorityOfTheBaseUrl() throws ScanovateException {
    assertEquals(
        "https://liveness.example.com:8443", OnPremSettings.fromConfig(livenessConfig()).origin());
  }

  @Test
  void configuredValuesAreRead() throws ScanovateException {
    Map<String, String> config = livenessConfig();
    config.put(ScanovateAuthenticatorFactory.LIVENESS_URL, "http://127.0.0.1:5050/biometric");
    config.put(ScanovateAuthenticatorFactory.LIVENESS_RESULT_WAIT_SECONDS, "30");

    OnPremSettings settings = OnPremSettings.fromConfig(config);

    assertEquals(30, settings.resultWaitSeconds());
    assertEquals(
        "http://127.0.0.1:5050/biometric/liveness/?case_id=c&token=t",
        settings.pageUrl(OnPremService.LIVENESS, "c", "t", Map.of()));
  }

  @Test
  void missingOrInvalidSettingsAreRejected() {
    for (Map.Entry<String, String> invalid :
        List.of(
            Map.entry(ScanovateAuthenticatorFactory.LIVENESS_URL, ""),
            Map.entry(ScanovateAuthenticatorFactory.LIVENESS_URL, "liveness.example.com"),
            Map.entry(ScanovateAuthenticatorFactory.LIVENESS_URL, "ftp://liveness.example.com"),
            Map.entry(ScanovateAuthenticatorFactory.LIVENESS_URL, "https://"),
            Map.entry(ScanovateAuthenticatorFactory.LIVENESS_SECRET, " "),
            Map.entry(ScanovateAuthenticatorFactory.LIVENESS_RESULT_WAIT_SECONDS, "0"))) {
      Map<String, String> config = livenessConfig();
      config.put(invalid.getKey(), invalid.getValue());
      assertThrows(
          ScanovateException.class, () -> OnPremSettings.fromConfig(config), invalid.toString());
    }
    Map<String, String> withoutSecret = livenessConfig();
    withoutSecret.remove(ScanovateAuthenticatorFactory.LIVENESS_SECRET);
    assertThrows(ScanovateException.class, () -> OnPremSettings.fromConfig(withoutSecret));
  }
}
