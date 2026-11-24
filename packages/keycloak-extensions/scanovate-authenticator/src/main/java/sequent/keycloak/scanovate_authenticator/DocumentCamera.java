// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.util.Locale;
import java.util.Map;

/**
 * The camera the document scan starts with on a device that has one on each side: a phone or a
 * tablet. The voter can change it on the capture page, and the device then keeps their choice.
 */
public enum DocumentCamera {
  /**
   * The camera facing away from the voter, which Scanovate's page opens by itself: for a voter
   * holding their own phone over the document.
   */
  REAR,
  /**
   * The camera facing the voter: for a tablet on a stand, which the voter shows the document to.
   */
  FRONT;

  /**
   * Reads the camera from the authenticator configuration: {@link #REAR} unless set.
   *
   * @throws ScanovateException if the setting names no camera
   */
  public static DocumentCamera fromConfig(Map<String, String> config) throws ScanovateException {
    String value = config.get(ScanovateAuthenticatorFactory.DOCUMENT_CAMERA);
    if (value == null || value.isBlank()) {
      return REAR;
    }
    try {
      return valueOf(value.trim().toUpperCase(Locale.ROOT));
    } catch (IllegalArgumentException e) {
      throw new ScanovateException(
          "Invalid " + ScanovateAuthenticatorFactory.DOCUMENT_CAMERA + ": " + value, e);
    }
  }

  /** The camera as the capture page reads it. */
  public String pageValue() {
    return name().toLowerCase(Locale.ROOT);
  }
}
