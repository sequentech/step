// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

/** The photos posted by the capture page are missing, too big or not JPEG images. */
public class InvalidCaptureException extends Exception {
  public InvalidCaptureException(String message) {
    super(message);
  }

  public InvalidCaptureException(String message, Throwable cause) {
    super(message, cause);
  }
}
