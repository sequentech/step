// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.util.Arrays;
import java.util.Optional;

/** Data persistence policy requested to B-Trust when creating a session. */
public enum SaveOption {
  DEFAULT(""),
  SAVE("save"),
  DO_NOT_SAVE("do_not_save");

  private final String value;

  SaveOption(String value) {
    this.value = value;
  }

  public String value() {
    return value;
  }

  public static Optional<SaveOption> fromValue(String value) {
    String normalized = value == null ? "" : value.trim();
    return Arrays.stream(values()).filter(option -> option.value.equals(normalized)).findFirst();
  }
}
