// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.util.Map;

/** A value extracted from the verification results and stored as an authentication note. */
public record StoredAttribute(String key, String value, String type) {
  /** Plain map for the login pages, as Keycloakify only serialises maps and lists. */
  public Map<String, String> toTemplateModel() {
    return Map.of("key", key, "value", value, "type", type);
  }
}
