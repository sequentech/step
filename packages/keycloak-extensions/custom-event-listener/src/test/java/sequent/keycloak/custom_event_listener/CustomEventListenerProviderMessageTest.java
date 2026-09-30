// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.junit.jupiter.api.Test;

class CustomEventListenerProviderMessageTest {

  private final ObjectMapper om = new ObjectMapper();

  @Test
  void sendsWhenTheEventHappenedAsANumberNextToTheFieldsItAlwaysSent() throws Exception {
    JsonNode message =
        om.readTree(
            om.writeValueAsBytes(
                CustomEventListenerProvider.buildMessage(
                    "event-1", "LOGIN", "null", "user-1", "tenant-1", "voter", 1778511600123L)));

    assertTrue(message.isArray());
    assertEquals(3, message.size());
    JsonNode input = message.get(1).get("input");
    assertEquals("event-1", input.get("election_event_id").asText());
    assertEquals("LOGIN", input.get("message_type").asText());
    assertEquals("null", input.get("body").asText());
    assertEquals("user-1", input.get("user_id").asText());
    assertEquals("tenant-1", input.get("tenant_id").asText());
    assertEquals("voter", input.get("username").asText());
    assertTrue(input.get("event_time_ms").isIntegralNumber());
    assertEquals(1778511600123L, input.get("event_time_ms").asLong());
    assertEquals(7, input.size());
  }

  @Test
  void aMissingValueIsSentAsTheStringNull() throws Exception {
    JsonNode input =
        om.readTree(
                om.writeValueAsBytes(
                    CustomEventListenerProvider.buildMessage(
                        null, null, null, null, null, null, 0L)))
            .get(1)
            .get("input");

    for (String field :
        new String[] {
          "election_event_id", "message_type", "body", "user_id", "tenant_id", "username"
        }) {
      assertEquals("null", input.get(field).asText(), field);
    }
    assertEquals(0L, input.get("event_time_ms").asLong());
  }
}
