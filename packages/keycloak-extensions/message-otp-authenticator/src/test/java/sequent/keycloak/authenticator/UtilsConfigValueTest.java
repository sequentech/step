// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.keycloak.models.AuthenticatorConfigModel;

class UtilsConfigValueTest {

  private static final String KEY = Utils.RESEND_ACTIVATION_TIMER;
  private static final String DEFAULT = Utils.RESEND_ACTIVATION_TIMER_DEFAULT;

  private AuthenticatorConfigModel configWith(Map<String, String> values) {
    AuthenticatorConfigModel config = new AuthenticatorConfigModel();
    config.setConfig(values);
    return config;
  }

  @Test
  void returnsTheConfiguredValue() {
    assertEquals("90", Utils.getConfigValue(configWith(Map.of(KEY, "90")), KEY, DEFAULT));
  }

  @Test
  void returnsTheDefaultWhenTheKeyIsMissing() {
    assertEquals(DEFAULT, Utils.getConfigValue(configWith(new HashMap<>()), KEY, DEFAULT));
  }

  @Test
  void returnsTheDefaultWhenTheValueIsBlank() {
    Map<String, String> values = new HashMap<>();
    values.put(KEY, "  ");
    assertEquals(DEFAULT, Utils.getConfigValue(configWith(values), KEY, DEFAULT));
  }

  @Test
  void returnsTheDefaultWhenTheConfigIsNull() {
    assertEquals(DEFAULT, Utils.getConfigValue((AuthenticatorConfigModel) null, KEY, DEFAULT));
  }

  @Test
  void returnsTheDefaultWhenTheConfigMapIsNull() {
    assertEquals(DEFAULT, Utils.getConfigValue(configWith(null), KEY, DEFAULT));
    assertEquals(DEFAULT, Utils.getConfigValue((Map<String, String>) null, KEY, DEFAULT));
  }

  @Test
  void readsFromAPlainConfigMap() {
    assertEquals("90", Utils.getConfigValue(Map.of(KEY, "90"), KEY, DEFAULT));
    assertEquals(DEFAULT, Utils.getConfigValue(Map.of(), KEY, DEFAULT));
  }
}
