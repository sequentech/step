// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.authenticator;

import java.util.Arrays;

/** Whether the code page tells the voter which of the flow's codes it asks for. */
public enum CodeProgressPolicy {
  /** The page doesn't count the codes. */
  NONE,
  /** With more than one code in the flow, the page shows "Code n of m". */
  SHOW;

  /** The configured policy, or {@link #NONE} when unset or unknown. */
  public static CodeProgressPolicy fromConfig(String value) {
    return Arrays.stream(values())
        .filter(policy -> policy.name().equals(value))
        .findFirst()
        .orElse(NONE);
  }
}
