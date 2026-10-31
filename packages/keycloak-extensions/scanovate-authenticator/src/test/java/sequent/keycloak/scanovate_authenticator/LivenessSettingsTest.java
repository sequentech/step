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

class LivenessSettingsTest {
  static Map<String, String> livenessConfig() {
    Map<String, String> config = new HashMap<>();
    config.put(ScanovateAuthenticatorFactory.LIVENESS_URL, "https://liveness.example.com:8443/");
    config.put(ScanovateAuthenticatorFactory.LIVENESS_SECRET, "callback-secret");
    config.put(ScanovateAuthenticatorFactory.FACE_MATCH_URL, "http://face-match:3000");
    return config;
  }

  @Test
  void defaultsApplyWhenUnset() throws ScanovateException {
    LivenessSettings settings = LivenessSettings.fromConfig(livenessConfig());

    assertEquals("callback-secret", settings.secret());
    assertEquals(
        ScanovateAuthenticatorFactory.DEFAULT_LIVENESS_RESULT_WAIT_SECONDS,
        settings.resultWaitSeconds());
  }

  @Test
  void apiUrlIsTheLivenessPathOfTheService() throws ScanovateException {
    assertEquals(
        "https://liveness.example.com:8443/liveness",
        LivenessSettings.fromConfig(livenessConfig()).apiUrl());
  }

  @Test
  void configuredValuesAreRead() throws ScanovateException {
    Map<String, String> config = livenessConfig();
    config.put(ScanovateAuthenticatorFactory.LIVENESS_URL, "http://127.0.0.1:5050/biometric");
    config.put(ScanovateAuthenticatorFactory.LIVENESS_RESULT_WAIT_SECONDS, "30");

    LivenessSettings settings = LivenessSettings.fromConfig(config);

    assertEquals(30, settings.resultWaitSeconds());
    assertEquals("http://127.0.0.1:5050/biometric/liveness", settings.apiUrl());
  }

  @Test
  void settingsOfTheRemovedIframeAreIgnored() throws ScanovateException {
    Map<String, String> config = livenessConfig();
    config.put("liveness-ui-theme", "tenant_ui");
    config.put("liveness-translation-variant", "tenant");
    config.put("liveness-languages", "en,tl");

    assertEquals(
        "https://liveness.example.com:8443/liveness", LivenessSettings.fromConfig(config).apiUrl());
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
          ScanovateException.class, () -> LivenessSettings.fromConfig(config), invalid.toString());
    }
    Map<String, String> withoutSecret = livenessConfig();
    withoutSecret.remove(ScanovateAuthenticatorFactory.LIVENESS_SECRET);
    assertThrows(ScanovateException.class, () -> LivenessSettings.fromConfig(withoutSecret));
  }
}
