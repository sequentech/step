// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import java.util.regex.Pattern;

/** The log a realm's events belong to, read from the realm's name. */
record LogRealm(Kind kind, String tenantId, String electionEventId) {

  enum Kind {
    /** {@code tenant-<tenant uuid>-event-<election event uuid>}: the election event's log. */
    EVENT,
    /** {@code tenant-<tenant uuid>}: the tenant's log. */
    TENANT,
    /** Any other realm, such as master: its events are not logged. */
    OTHER
  }

  private static final String TENANT_PREFIX = "tenant-";
  private static final String EVENT_SEPARATOR = "-event-";
  private static final Pattern UUID =
      Pattern.compile("[0-9a-fA-F]{8}(-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}");
  private static final LogRealm NONE = new LogRealm(Kind.OTHER, "", "");

  static LogRealm fromName(String realmName) {
    if (realmName == null || !realmName.startsWith(TENANT_PREFIX)) {
      return NONE;
    }
    int eventIndex = realmName.indexOf(EVENT_SEPARATOR);
    if (eventIndex >= 0) {
      return new LogRealm(
          Kind.EVENT,
          realmName.substring(TENANT_PREFIX.length(), eventIndex),
          realmName.substring(eventIndex + EVENT_SEPARATOR.length()));
    }
    String tenantId = realmName.substring(TENANT_PREFIX.length());
    return UUID.matcher(tenantId).matches() ? new LogRealm(Kind.TENANT, tenantId, "") : NONE;
  }
}
