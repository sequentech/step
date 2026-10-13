// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.TreeMap;
import org.keycloak.events.Event;

/** The {@code details} object sent with every user event. */
final class UserEventDetails {
  /** Whether Keycloak knows the account the event is about. */
  enum Account {
    KNOWN,
    UNKNOWN
  }

  private static final List<String> NAMED = List.of("auth_method", "auth_type", "credential_type");
  private static final String IDENTITY_PROVIDER = "identity_provider";

  /** A detail whose name contains one of these may hold a secret and is never sent. */
  private static final List<String> SECRET_NAME_PARTS =
      List.of("password", "secret", "token", "code");

  /** What was typed at a form: not sent when the account is unknown. */
  private static final Set<String> TYPED_IDENTITY =
      Set.of("username", "email", "identity_provider_identity");

  private UserEventDetails() {}

  static Map<String, Object> of(Event event, Account account) {
    Map<String, String> keycloakDetails =
        event.getDetails() != null ? event.getDetails() : Map.of();
    Map<String, Object> details = new LinkedHashMap<>();
    details.put("client_id", event.getClientId());
    details.put("ip_address", event.getIpAddress());
    details.put("error", event.getError());
    for (String name : NAMED) {
      details.put(name, keycloakDetails.get(name));
    }
    if (keycloakDetails.containsKey(IDENTITY_PROVIDER)) {
      details.put(IDENTITY_PROVIDER, keycloakDetails.get(IDENTITY_PROVIDER));
    }
    if (account == Account.UNKNOWN) {
      details.put("unknown_account", true);
    }

    Map<String, String> remaining = new TreeMap<>();
    keycloakDetails.forEach(
        (name, value) -> {
          boolean sentElsewhere = NAMED.contains(name) || IDENTITY_PROVIDER.equals(name);
          boolean typed = account == Account.UNKNOWN && TYPED_IDENTITY.contains(name);
          if (!sentElsewhere && !typed && !maySayASecret(name)) {
            remaining.put(name, value);
          }
        });
    details.put("details", remaining);
    return details;
  }

  static boolean maySayASecret(String detailName) {
    String name = detailName.toLowerCase(Locale.ROOT);
    return SECRET_NAME_PARTS.stream().anyMatch(name::contains);
  }
}
