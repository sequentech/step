// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import static org.junit.jupiter.api.Assertions.assertEquals;

import org.junit.jupiter.api.Test;

class LogRealmTest {
  private static final String TENANT = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
  private static final String EVENT = "6f1c3a6e-2a61-4d6b-9a52-3f0f8a3c2b10";

  @Test
  void anEventRealmNamesItsTenantAndItsElectionEvent() {
    LogRealm realm = LogRealm.fromName("tenant-" + TENANT + "-event-" + EVENT);

    assertEquals(LogRealm.Kind.EVENT, realm.kind());
    assertEquals(TENANT, realm.tenantId());
    assertEquals(EVENT, realm.electionEventId());
  }

  @Test
  void aTenantRealmNamesItsTenantAndNoElectionEvent() {
    LogRealm realm = LogRealm.fromName("tenant-" + TENANT);

    assertEquals(LogRealm.Kind.TENANT, realm.kind());
    assertEquals(TENANT, realm.tenantId());
    assertEquals("", realm.electionEventId());
  }

  @Test
  void anyOtherRealmHasNoLog() {
    for (String name :
        new String[] {
          "master",
          "tenant-",
          "tenant-acme",
          "tenant-" + TENANT + "-copy",
          "event-" + EVENT,
          "",
          null
        }) {
      assertEquals(LogRealm.Kind.OTHER, LogRealm.fromName(name).kind(), String.valueOf(name));
    }
  }
}
