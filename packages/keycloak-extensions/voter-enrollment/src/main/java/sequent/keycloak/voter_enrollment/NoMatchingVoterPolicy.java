// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import java.util.Arrays;

/** What an enrollment whose identity matches no voter of the census becomes. */
public enum NoMatchingVoterPolicy {
  /** Rejected automatically. */
  REJECT,
  /** Pending, for an election manager to review in the election event's Approvals. */
  PENDING_APPROVAL;

  /** The configured policy, or {@link #REJECT} when unset or unknown. */
  public static NoMatchingVoterPolicy fromConfig(String value) {
    return Arrays.stream(values())
        .filter(policy -> policy.name().equals(value))
        .findFirst()
        .orElse(REJECT);
  }
}
