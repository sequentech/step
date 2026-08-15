// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.realm;

import java.util.Arrays;
import java.util.Optional;

public final class RealmNames {

  private RealmNames() {}

  private static final String TENANT_SEGMENT = "tenant";
  private static final String EVENT_SEGMENT = "event";

  public record EventRealm(String tenantId, String electionEventId) {}

  public static Optional<EventRealm> parseEventRealmName(String realmName) {
    if (realmName == null || realmName.isBlank()) {
      return Optional.empty();
    }

    String[] parts = realmName.split("-", -1);
    if (parts.length < 4 || !TENANT_SEGMENT.equals(parts[0])) {
      return Optional.empty();
    }

    int eventIndex = -1;
    for (int i = 1; i < parts.length; i++) {
      if (EVENT_SEGMENT.equals(parts[i])) {
        eventIndex = i;
        break;
      }
    }

    if (eventIndex <= 1 || eventIndex >= parts.length - 1) {
      return Optional.empty();
    }

    String tenantId = String.join("-", Arrays.copyOfRange(parts, 1, eventIndex));
    String electionEventId =
        String.join("-", Arrays.copyOfRange(parts, eventIndex + 1, parts.length));
    if (tenantId.isBlank() || electionEventId.isBlank()) {
      return Optional.empty();
    }

    return Optional.of(new EventRealm(tenantId, electionEventId));
  }

  public static Optional<String> electionEventIdFromRealmName(String realmName) {
    return parseEventRealmName(realmName).map(EventRealm::electionEventId);
  }
}
