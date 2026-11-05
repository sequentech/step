// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import java.util.Optional;

/** Who carries the messaging-app channels, decided when Keycloak runs and not when it is built. */
public enum MessageSenderPolicy {
  /** Harvest carries them whenever its address is configured. */
  HARVEST_WHEN_CONFIGURED,
  /** No messaging-app channel: only Keycloak's own email and SMS providers send. */
  KEYCLOAK_ONLY;

  /** Empty for no value. A value that is not a policy keeps every message in Keycloak. */
  public static Optional<MessageSenderPolicy> parse(String value) {
    if (value == null || value.isBlank()) {
      return Optional.empty();
    }
    for (MessageSenderPolicy policy : values()) {
      if (policy.name().equalsIgnoreCase(value.trim())) {
        return Optional.of(policy);
      }
    }
    return Optional.of(KEYCLOAK_ONLY);
  }
}
