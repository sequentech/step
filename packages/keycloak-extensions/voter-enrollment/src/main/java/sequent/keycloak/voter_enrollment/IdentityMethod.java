// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import java.util.Arrays;
import java.util.Optional;

/**
 * How an identity verification step established the applicant's identity, which the approval matrix
 * decides with. Only Keycloak sets it, in the {@link LookupAndUpdateUser#IDENTITY_METHOD} auth
 * note: never from a form.
 */
public enum IdentityMethod {
  /** The step verified the identity document and the applicant, who confirmed what it read. */
  VERIFIED,
  /** The applicant typed the identity data by hand. */
  MANUAL_ENTRY;

  /** The method the note names, or empty when unset or unknown: never VERIFIED by default. */
  public static Optional<IdentityMethod> fromNote(String value) {
    return Arrays.stream(values()).filter(method -> method.name().equals(value)).findFirst();
  }
}
