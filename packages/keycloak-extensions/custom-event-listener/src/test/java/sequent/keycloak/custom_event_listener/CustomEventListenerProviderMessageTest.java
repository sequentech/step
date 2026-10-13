// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.junit.jupiter.api.Test;
import org.keycloak.events.EventType;

class CustomEventListenerProviderMessageTest {

  private final ObjectMapper om = new ObjectMapper();

  @Test
  void sendsWhenTheEventHappenedAsANumberNextToTheFieldsItAlwaysSent() throws Exception {
    JsonNode message =
        om.readTree(
            om.writeValueAsBytes(
                CustomEventListenerProvider.taskMessage(
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
                    CustomEventListenerProvider.taskMessage(
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

  private static final String TENANT = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
  private static final String EVENT = "33f18502-a67c-4853-8333-a58630663559";

  @Test
  void anElectionEventRealmNamesItsTenantAndEvent() {
    assertArrayEquals(
        new String[] {TENANT, EVENT},
        CustomEventListenerProvider.tenantAndEvent("tenant-" + TENANT + "-event-" + EVENT));
  }

  @Test
  void aTenantRealmNamesItsTenantAndNoEvent() {
    assertArrayEquals(
        new String[] {TENANT, ""}, CustomEventListenerProvider.tenantAndEvent("tenant-" + TENANT));
  }

  @Test
  void anyOtherRealmNamesNeither() {
    for (String realm : new String[] {"master", "", "event-" + EVENT, null}) {
      assertArrayEquals(
          new String[] {"", ""}, CustomEventListenerProvider.tenantAndEvent(realm), realm);
    }
  }

  @Test
  void aTenantRealmLeavesOutTokenHousekeepingAndKeepsEverythingElse() {
    for (EventType skipped :
        new EventType[] {
          EventType.CODE_TO_TOKEN,
          EventType.REFRESH_TOKEN,
          EventType.INTROSPECT_TOKEN,
          EventType.USER_INFO_REQUEST,
          EventType.CLIENT_LOGIN,
          EventType.TOKEN_EXCHANGE,
          EventType.PERMISSION_TOKEN
        }) {
      assertFalse(CustomEventListenerProvider.isLogged(TENANT, "", skipped), skipped.name());
    }
    for (EventType kept :
        new EventType[] {
          EventType.LOGIN,
          EventType.LOGIN_ERROR,
          EventType.LOGOUT,
          EventType.UPDATE_PASSWORD,
          EventType.UPDATE_TOTP,
          EventType.REMOVE_TOTP,
          EventType.REFRESH_TOKEN_ERROR,
          EventType.CODE_TO_TOKEN_ERROR
        }) {
      assertTrue(CustomEventListenerProvider.isLogged(TENANT, "", kept), kept.name());
    }
  }

  @Test
  void anElectionEventRealmAndAnUnknownRealmSendEveryEvent() {
    for (EventType type : EventType.values()) {
      assertTrue(CustomEventListenerProvider.isLogged(TENANT, EVENT, type), type.name());
      assertTrue(CustomEventListenerProvider.isLogged("", "", type), type.name());
    }
  }
}
