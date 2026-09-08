// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.realm;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

class RealmNamesTest {

  @Test
  void parseEventRealmName_extractsTenantAndElectionEventIds() {
    var parsed =
        RealmNames.parseEventRealmName(
                "tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-388b3eff-e583-4a56-82b7-0ad15eaa409a")
            .orElseThrow();

    assertEquals("90505c8a-23a9-4cdf-a26b-4e19f6a097d5", parsed.tenantId());
    assertEquals("388b3eff-e583-4a56-82b7-0ad15eaa409a", parsed.electionEventId());
  }

  @Test
  void parseEventRealmName_acceptsHyphenatedNonUuidIds() {
    var parsed = RealmNames.parseEventRealmName("tenant-acme-prod-event-spring-2026").orElseThrow();

    assertEquals("acme-prod", parsed.tenantId());
    assertEquals("spring-2026", parsed.electionEventId());
  }

  @Test
  void parseEventRealmName_rejectsTenantRealmAndMalformedNames() {
    assertTrue(RealmNames.parseEventRealmName("tenant-acme-prod").isEmpty());
    assertTrue(RealmNames.parseEventRealmName("tenant-acme-event").isEmpty());
    assertTrue(RealmNames.parseEventRealmName("acme-event-150017").isEmpty());
    assertTrue(RealmNames.parseEventRealmName(null).isEmpty());
  }

  @Test
  void electionEventIdFromRealmName_returnsOnlyTheEventId() {
    assertEquals(
        "150017",
        RealmNames.electionEventIdFromRealmName("tenant-acme-event-150017").orElseThrow());
  }

  @Test
  void tenantIdFromRealmName_returnsOnlyTenantRealms() {
    assertEquals(
        "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
        RealmNames.tenantIdFromRealmName("tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5")
            .orElseThrow());
    assertTrue(RealmNames.tenantIdFromRealmName("tenant-acme-event-150017").isEmpty());
    assertTrue(RealmNames.tenantIdFromRealmName("tenant-acme-event").isEmpty());
    assertTrue(RealmNames.tenantIdFromRealmName("acme").isEmpty());
    assertTrue(RealmNames.tenantIdFromRealmName(null).isEmpty());
  }
}
