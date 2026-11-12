// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.Test;

class HoldingPhotoPolicyTest {
  @Test
  void theHoldingPhotoIsRequiredUnlessConfiguredOtherwise() throws ScanovateException {
    assertEquals(HoldingPhotoPolicy.REQUIRED, HoldingPhotoPolicy.fromConfig(Map.of()));
    Map<String, String> blank = new HashMap<>();
    blank.put(ScanovateAuthenticatorFactory.HOLDING_PHOTO, " ");
    assertEquals(HoldingPhotoPolicy.REQUIRED, HoldingPhotoPolicy.fromConfig(blank));
  }

  @Test
  void theConfiguredPolicyIsRead() throws ScanovateException {
    assertEquals(
        HoldingPhotoPolicy.NONE,
        HoldingPhotoPolicy.fromConfig(Map.of(ScanovateAuthenticatorFactory.HOLDING_PHOTO, "NONE")));
    assertEquals(
        HoldingPhotoPolicy.REQUIRED,
        HoldingPhotoPolicy.fromConfig(
            Map.of(ScanovateAuthenticatorFactory.HOLDING_PHOTO, " required ")));
  }

  @Test
  void anUnknownPolicyIsAConfigurationError() {
    assertThrows(
        ScanovateException.class,
        () ->
            HoldingPhotoPolicy.fromConfig(
                Map.of(ScanovateAuthenticatorFactory.HOLDING_PHOTO, "SOMETIMES")));
  }
}
