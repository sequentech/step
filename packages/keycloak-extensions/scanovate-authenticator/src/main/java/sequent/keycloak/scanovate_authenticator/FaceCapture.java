// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.util.Arrays;
import java.util.Optional;

/** How the voter's face is captured in the embedded mode. */
public enum FaceCapture {
  /**
   * Keycloak's own page takes a photo of the voter and records a video of them holding the ID, and
   * B-Trust checks their liveness.
   */
  PHOTO("photo"),
  /**
   * The on-premise Scanovate Liveness Plus service checks the voter's liveness in an iframe, and
   * its picture of the voter is sent to B-Trust as the face photo. Only the ID is captured by
   * Keycloak's own page.
   */
  LIVENESS("liveness");

  private final String value;

  FaceCapture(String value) {
    this.value = value;
  }

  public String value() {
    return value;
  }

  public static Optional<FaceCapture> fromValue(String value) {
    return Arrays.stream(values()).filter(capture -> capture.value.equals(value)).findFirst();
  }
}
