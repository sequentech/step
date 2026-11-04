// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.util.Locale;
import java.util.Map;

/**
 * Whether the voter also takes a photo of themselves holding their document, which Keycloak
 * compares with their live face, after Scanovate's document scan and face check.
 */
public enum HoldingPhotoPolicy {
  /** The capture page asks for the photo, and the verification needs it to match the live face. */
  REQUIRED,
  /** No holding photo. */
  NONE;

  /**
   * Reads the policy from the authenticator configuration: {@link #REQUIRED} unless set.
   *
   * @throws ScanovateException if the setting names no policy
   */
  public static HoldingPhotoPolicy fromConfig(Map<String, String> config)
      throws ScanovateException {
    String value = config.get(ScanovateAuthenticatorFactory.HOLDING_PHOTO);
    if (value == null || value.isBlank()) {
      return REQUIRED;
    }
    try {
      return valueOf(value.trim().toUpperCase(Locale.ROOT));
    } catch (IllegalArgumentException e) {
      throw new ScanovateException(
          "Invalid " + ScanovateAuthenticatorFactory.HOLDING_PHOTO + ": " + value, e);
    }
  }
}
