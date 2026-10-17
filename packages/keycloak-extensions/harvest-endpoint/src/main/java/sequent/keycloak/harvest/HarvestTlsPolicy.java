// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.harvest;

/** Which harvest URL schemes the Keycloak extensions accept. */
public enum HarvestTlsPolicy {
  /** Accepts {@code http} and {@code https} harvest URLs. */
  PLAINTEXT_ALLOWED,
  /** Accepts only {@code https} harvest URLs. */
  REQUIRE_TLS
}
