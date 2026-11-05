// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import java.util.Set;
import org.keycloak.events.Event;

/**
 * Which user events of a tenant realm go to the tenant log: sign-ins, sign-outs, credential
 * changes, lockouts, impersonation, messages sent to the account and every error. Successful token
 * refreshes, code exchanges, client logins and the like are left out.
 */
final class TenantRealmEvents {
  private static final String ERROR_SUFFIX = "_ERROR";

  /** Names of {@link org.keycloak.events.EventType}, so a type Keycloak drops still compiles. */
  static final Set<String> KEPT_WHEN_SUCCESSFUL =
      Set.of(
          "LOGIN",
          "LOGOUT",
          "REGISTER",
          "IDENTITY_PROVIDER_LOGIN",
          "IDENTITY_PROVIDER_FIRST_LOGIN",
          "IMPERSONATE",
          "UPDATE_PASSWORD",
          "RESET_PASSWORD",
          "SEND_RESET_PASSWORD",
          "UPDATE_TOTP",
          "REMOVE_TOTP",
          "UPDATE_CREDENTIAL",
          "REMOVE_CREDENTIAL",
          "FEDERATED_IDENTITY_LINK",
          "REMOVE_FEDERATED_IDENTITY",
          "USER_DISABLED_BY_TEMPORARY_LOCKOUT",
          "USER_DISABLED_BY_PERMANENT_LOCKOUT",
          "VERIFY_EMAIL",
          "UPDATE_EMAIL",
          "UPDATE_PROFILE",
          "EXECUTE_ACTIONS",
          "CUSTOM_REQUIRED_ACTION",
          "DELETE_ACCOUNT");

  private TenantRealmEvents() {}

  static boolean keeps(Event event) {
    String type = event.getType().name();
    return event.getError() != null
        || type.endsWith(ERROR_SUFFIX)
        || KEPT_WHEN_SUCCESSFUL.contains(type)
        || Utils.isCommunication(event.getDetails());
  }
}
