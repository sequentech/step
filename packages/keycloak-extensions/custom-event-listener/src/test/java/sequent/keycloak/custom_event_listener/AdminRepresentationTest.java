// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.HexFormat;
import org.junit.jupiter.api.Test;

class AdminRepresentationTest {
  private final ObjectMapper om = new ObjectMapper();

  private JsonNode logged(String representation) {
    return om.valueToTree(AdminRepresentation.forLog(representation));
  }

  @Test
  void noRepresentationIsNull() {
    assertNull(AdminRepresentation.forLog(null));
  }

  @Test
  void secretValuesAreRedactedAtAnyDepth() throws Exception {
    JsonNode logged =
        logged(
            """
            {"username": "auditor",
             "credentials": [{"type": "password", "value": "hunter2", "temporary": true}],
             "smtpServer": {"host": "smtp.example.com", "Password": "smtp-pass"},
             "config": {"clientSecret": "idp-secret", "bindCredential": ["ldap-pass"],
                        "privateKey": "-----BEGIN PRIVATE KEY-----", "clientId": "step"},
             "attributes": {"registrationToken": ["abc"], "area": ["north"]},
             "value": "kept"}
            """);

    assertEquals(
        om.readTree(
            """
            {"username": "auditor",
             "credentials": [{"type": "password", "value": "**redacted**", "temporary": true}],
             "smtpServer": {"host": "smtp.example.com", "Password": "**redacted**"},
             "config": {"clientSecret": "**redacted**", "bindCredential": "**redacted**",
                        "privateKey": "**redacted**", "clientId": "step"},
             "attributes": {"registrationToken": "**redacted**", "area": ["north"]},
             "value": "kept"}
            """),
        logged);
    assertFalse(logged.toString().contains("hunter2"));
  }

  @Test
  void settingsNamedAfterASecretKeepTheirNumbersAndSwitches() throws Exception {
    String settings =
        "{\"accessTokenLifespan\":300,\"resetPasswordAllowed\":true,\"revokeRefreshToken\":false}";

    assertEquals(om.readTree(settings), logged(settings));
  }

  @Test
  void aRepresentationThatIsNotJsonIsKeptAsText() {
    assertEquals("not json", AdminRepresentation.forLog("not json"));
  }

  @Test
  void aRepresentationOver64KibibytesIsReplacedByItsHash() throws Exception {
    String atTheLimit = "{\"description\":\"" + "a".repeat(65536 - 18) + "\"}";
    String overTheLimit = "{\"description\":\"" + "a".repeat(65536 - 17) + "\"}";
    assertEquals(65536, atTheLimit.getBytes(StandardCharsets.UTF_8).length);

    assertEquals(om.readTree(atTheLimit), logged(atTheLimit));

    String sha256 =
        HexFormat.of()
            .formatHex(
                MessageDigest.getInstance("SHA-256")
                    .digest(overTheLimit.getBytes(StandardCharsets.UTF_8)));
    assertEquals(
        om.readTree("{\"sha256\":\"" + sha256 + "\",\"truncated\":true}"), logged(overTheLimit));
  }

  @Test
  void theHashOfALongRepresentationIsTakenAfterRedacting() throws Exception {
    String padding = "a".repeat(70000);
    String redacted = "{\"password\":\"**redacted**\",\"description\":\"" + padding + "\"}";
    String sha256 =
        HexFormat.of()
            .formatHex(
                MessageDigest.getInstance("SHA-256")
                    .digest(redacted.getBytes(StandardCharsets.UTF_8)));

    assertEquals(
        sha256,
        logged("{\"password\":\"hunter2\",\"description\":\"" + padding + "\"}")
            .get("sha256")
            .asText());
  }
}
