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
   * Keycloak's own page checks the voter's liveness with the API of the on-premise Scanovate
   * Liveness Plus service and takes a photo of the voter holding the ID. Keycloak compares Liveness
   * Plus's picture of the voter with the photo of the ID and the photo holding it using the
   * on-premise Scanovate Face Match service. Only the photos of the ID are sent to B-Trust.
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
