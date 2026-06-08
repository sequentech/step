// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link.hmac;

/** Thrown when an HMAC Smart Link auth-token fails validation. Carries a {@link SmartLinkError}. */
public class SmartLinkValidationException extends Exception {

  private final SmartLinkError error;

  public SmartLinkValidationException(SmartLinkError error, String detail) {
    super(detail);
    this.error = error;
  }

  public SmartLinkError getError() {
    return error;
  }
}
