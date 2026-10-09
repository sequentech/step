// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.util.List;
import org.junit.jupiter.api.Test;
import org.keycloak.provider.ProviderConfigProperty;

class NoMatchingVoterPolicyTest {
  @Test
  void readsTheConfiguredPolicy() {
    assertEquals(
        NoMatchingVoterPolicy.PENDING_APPROVAL,
        NoMatchingVoterPolicy.fromConfig("PENDING_APPROVAL"));
    assertEquals(NoMatchingVoterPolicy.REJECT, NoMatchingVoterPolicy.fromConfig("REJECT"));
  }

  /** Realms configured before the setting existed keep rejecting. */
  @Test
  void rejectsWithoutAKnownPolicy() {
    assertEquals(NoMatchingVoterPolicy.REJECT, NoMatchingVoterPolicy.fromConfig(null));
    assertEquals(NoMatchingVoterPolicy.REJECT, NoMatchingVoterPolicy.fromConfig(""));
    assertEquals(NoMatchingVoterPolicy.REJECT, NoMatchingVoterPolicy.fromConfig("unknown"));
  }

  @Test
  void isAListSettingOfTheLookup() {
    ProviderConfigProperty property =
        new LookupAndUpdateUser()
            .getConfigProperties().stream()
                .filter(p -> p.getName().equals(LookupAndUpdateUser.NO_MATCHING_VOTER_POLICY))
                .findFirst()
                .orElseThrow();
    assertEquals(ProviderConfigProperty.LIST_TYPE, property.getType());
    assertEquals(NoMatchingVoterPolicy.REJECT.name(), property.getDefaultValue());
    assertEquals(List.of("REJECT", "PENDING_APPROVAL"), property.getOptions());
  }
}
