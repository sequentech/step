// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import java.util.Arrays;

/**
 * Which part of the enrollment a lookup step runs. Splitting it lets the flow check the voter's
 * contact, which sends an SMS or email, only for the voters the decision lets continue.
 */
public enum LookupStage {
  /** Decides and finishes the enrollment, sending the result to the contact the voter gave. */
  DECIDE_AND_FINISH,
  /**
   * Only decides. A rejected voter sees the result and is sent nothing; an accepted or pending
   * voter goes on to the next step.
   */
  DECIDE,
  /** Finishes what a {@link #DECIDE} lookup earlier in the flow decided. */
  FINISH;

  /** The configured stage, or {@link #DECIDE_AND_FINISH} when unset or unknown. */
  public static LookupStage fromConfig(String value) {
    return Arrays.stream(values())
        .filter(stage -> stage.name().equals(value))
        .findFirst()
        .orElse(DECIDE_AND_FINISH);
  }

  /**
   * Whether a {@link #DECIDE} lookup lets an enrollment with this status go on to the next step.
   */
  public static boolean continuesAfterDecision(String status) {
    return "ACCEPTED".equals(status) || "PENDING".equals(status);
  }

  /** Whether this stage may message a contact no step has checked yet. */
  public boolean messagesUncheckedContacts() {
    return this != DECIDE;
  }
}
