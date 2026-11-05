// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;

final class TestJson {
  private static final ObjectMapper MAPPER = new ObjectMapper();

  private TestJson() {}

  static JsonNode json(String value) {
    try {
      return MAPPER.readTree(value);
    } catch (Exception e) {
      throw new IllegalArgumentException(e);
    }
  }
}
