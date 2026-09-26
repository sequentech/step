// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.util.Arrays;
import java.util.Optional;

/** How the voter goes through the B-Trust flow. */
public enum ExecutionMode {
  /** The voter is redirected to B-Trust and results are fetched once they come back. */
  INTERACTIVE("interactive"),
  /**
   * Results are fetched right after creating the session, without redirecting the voter. Only
   * meaningful against a mock server that completes sessions instantly, e.g. for load testing.
   */
  AUTO_COMPLETE("auto-complete");

  private final String value;

  ExecutionMode(String value) {
    this.value = value;
  }

  public String value() {
    return value;
  }

  public static Optional<ExecutionMode> fromValue(String value) {
    return Arrays.stream(values()).filter(mode -> mode.value.equals(value)).findFirst();
  }
}
