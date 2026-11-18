// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.util.Arrays;
import java.util.Optional;

/**
 * The on-premise Scanovate services whose own capture page the voter goes through, in this order.
 *
 * <p>Each one serves its page and its API under {@code <base>/<path>/}, checks Keycloak's one-time
 * token with {@code onprem.token_verification_url} before starting a session, and posts the result
 * of the session to {@code onprem.callback_url}, server to server, see {@link OnPremSessions}.
 */
public enum OnPremService {
  /** Scans the voter's document. */
  OCR("ocr"),
  /** Checks that the voter is a live person in front of the camera. */
  LIVENESS("liveness");

  private final String path;

  OnPremService(String path) {
    this.path = path;
  }

  /** Path of the service under the base URL, and of its endpoints in Keycloak. */
  public String path() {
    return path;
  }

  public static Optional<OnPremService> fromPath(String path) {
    return Arrays.stream(values()).filter(service -> service.path.equals(path)).findFirst();
  }
}
