// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

/** Raised when the authenticator configuration cannot be applied to a verification result. */
public class ScanovateException extends Exception {
  public ScanovateException(String message) {
    super(message);
  }

  public ScanovateException(String message, Throwable cause) {
    super(message, cause);
  }
}
