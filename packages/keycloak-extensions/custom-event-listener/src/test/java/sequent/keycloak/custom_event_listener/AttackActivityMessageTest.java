// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.custom_event_listener;
import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;
import static org.mockito.ArgumentMatchers.*;
import org.keycloak.models.*;
import org.keycloak.events.Event;
import org.keycloak.events.EventType;
import org.mockito.ArgumentCaptor;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.junit.jupiter.api.Test;
class AttackActivityMessageTest {
  static final String TENANT="93d6f880-3d80-4721-ad4c-3141df3c4bc0";
  @Test void staffFailuresAreRecordedWithoutAnElection() throws Exception {
    var om=new ObjectMapper();var msg=CustomEventListenerProvider.attackMessage("tenant-"+TENANT,"event-id","staff-id",123,5,900);
    var input=om.valueToTree(msg).get(1).get("input");
    assertEquals(TENANT,input.get("tenant_id").asText());assertTrue(input.get("election_event_id").isNull());
    assertEquals(5,input.get("failure_limit").asInt());assertEquals(900,input.get("window_seconds").asInt());
    assertEquals(8,input.size());assertEquals(msg,CustomEventListenerProvider.attackMessage("tenant-"+TENANT,"event-id","staff-id",123,5,900));
  }
  @Test void realmIsParsedRatherThanTrustingCallerSuppliedTenant() {
    assertNull(CustomEventListenerProvider.attackMessage("master","id","a",1,5,900));
    assertNull(CustomEventListenerProvider.attackMessage("tenant-invalid","id","a",1,5,900));
    assertNotNull(CustomEventListenerProvider.attackMessage("tenant-"+TENANT+"-event-"+TENANT,"id",null,1,5,900));
  }
  @Test void listenerRecordsStaffFailuresEvenWhenEventStorageIsDisabled() throws Exception {
    var session=mock(KeycloakSession.class);var realms=mock(RealmProvider.class);var realm=mock(RealmModel.class);
    var publisher=mock(PgmqEventPublisher.class);
    when(session.realms()).thenReturn(realms);when(realms.getRealm("realm-id")).thenReturn(realm);
    when(realm.getName()).thenReturn("tenant-"+TENANT);when(realm.getFailureFactor()).thenReturn(5);
    when(realm.getMaxDeltaTimeSeconds()).thenReturn(900);when(realm.isEventsEnabled()).thenReturn(false);
    var event=new Event();event.setId("failure-id");event.setRealmId("realm-id");
    event.setType(EventType.LOGIN_ERROR);event.setUserId("known-user");event.setTime(123L);
    new CustomEventListenerProvider(session,publisher).onEvent(event);
    var bytes=ArgumentCaptor.forClass(byte[].class);
    verify(publisher).publish(eq(session),anyString(),eq("record_attack_auth_failure"),bytes.capture());
    var input=new ObjectMapper().readTree(bytes.getValue()).get(1).get("input");
    assertEquals("known-user",input.get("account").asText());
    assertEquals(5,input.get("failure_limit").asInt());
    verifyNoMoreInteractions(publisher);
  }
  @Test void ratesCannotDisableHeuristics() {
    assertThrows(IllegalArgumentException.class,()->CustomEventListenerProvider.attackMessage("tenant-"+TENANT,"id","a",1,0,900));
  }
}
