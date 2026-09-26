// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.util.Arrays;
import java.util.Optional;

/** Actions submitted from the confirmation and error pages. */
public enum FormAction {
  CONFIRM("confirm"),
  RETRY("retry");

  private final String value;

  FormAction(String value) {
    this.value = value;
  }

  public String value() {
    return value;
  }

  public static Optional<FormAction> fromValue(String value) {
    return Arrays.stream(values()).filter(action -> action.value.equals(value)).findFirst();
  }
}
