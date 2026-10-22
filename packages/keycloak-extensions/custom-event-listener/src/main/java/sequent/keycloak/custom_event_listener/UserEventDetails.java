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
import java.util.regex.Pattern;
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

  private static final Pattern ERROR_CODE = Pattern.compile("[a-z_]{1,128}");
  private static final Pattern NETWORK_ADDRESS =
      Pattern.compile("(?i)(?:[0-9]{1,3}\\.){3}[0-9]{1,3}|[0-9a-f]*:[0-9a-f:]*:[0-9a-f:]+");
  private static final Set<String> NETWORK_NAMES =
      Set.of(
          "ip",
          "ipaddress",
          "remoteip",
          "clientip",
          "remoteaddress",
          "forwardedfor",
          "xforwardedfor",
          "country",
          "countrycode",
          "useragent",
          "browser",
          "device",
          "deviceid",
          "gps",
          "geolocation");

  private UserEventDetails() {}

  static Map<String, Object> of(Event event, Account account) {
    Map<String, String> keycloakDetails =
        event.getDetails() != null ? event.getDetails() : Map.of();
    Map<String, Object> details = new LinkedHashMap<>();
    details.put("client_id", event.getClientId());
    details.put("error", safeError(event.getError()));
    for (String name : NAMED) {
      details.put(name, safeDetailValue(keycloakDetails.get(name)));
    }
    if (keycloakDetails.containsKey(IDENTITY_PROVIDER)) {
      details.put(IDENTITY_PROVIDER, safeDetailValue(keycloakDetails.get(IDENTITY_PROVIDER)));
    }
    if (account == Account.UNKNOWN) {
      details.put("unknown_account", true);
    }

    Map<String, String> remaining = new TreeMap<>();
    keycloakDetails.forEach(
        (name, value) -> {
          boolean sentElsewhere = NAMED.contains(name) || IDENTITY_PROVIDER.equals(name);
          boolean typed = account == Account.UNKNOWN && TYPED_IDENTITY.contains(name);
          // A sent message's text goes in the body of an election event's log only.
          boolean messageText = Utils.EVENT_DETAIL_MSG_BODY.equals(name);
          if (!sentElsewhere
              && !typed
              && !messageText
              && !maySayASecret(name)
              && !maySayNetwork(name)) {
            remaining.put(name, safeDetailValue(value));
          }
        });
    details.put("details", remaining);
    return details;
  }

  static String safeError(String value) {
    if (value == null || ERROR_CODE.matcher(value).matches()) {
      return value;
    }
    return "authentication_error";
  }

  static String safeDetailValue(String value) {
    if (value != null && NETWORK_ADDRESS.matcher(value).find()) {
      return "[redacted]";
    }
    return value;
  }

  static boolean maySayNetwork(String detailName) {
    String name = detailName.toLowerCase(Locale.ROOT).replaceAll("[^a-z0-9]", "");
    return NETWORK_NAMES.contains(name);
  }

  static boolean maySayASecret(String detailName) {
    String name = detailName.toLowerCase(Locale.ROOT);
    return SECRET_NAME_PARTS.stream().anyMatch(name::contains);
  }
}
