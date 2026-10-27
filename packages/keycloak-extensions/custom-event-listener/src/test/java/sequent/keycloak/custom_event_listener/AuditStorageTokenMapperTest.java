// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.custom_event_listener;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;
import java.sql.SQLException;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.keycloak.models.*;
import org.keycloak.protocol.oidc.mappers.OIDCAccessTokenMapper;
import org.keycloak.representations.AccessToken;
import org.keycloak.services.ErrorResponseException;

class AuditStorageTokenMapperTest {
  private KeycloakSession session(String name) {
    KeycloakSession s = mock(KeycloakSession.class);
    KeycloakContext context = mock(KeycloakContext.class);
    RealmModel realm = mock(RealmModel.class);
    when(s.getContext()).thenReturn(context);
    when(context.getRealm()).thenReturn(realm);
    when(realm.getName()).thenReturn(name);
    return s;
  }
  @Test void everyVoterTokenTransformationRefusesUnavailableStorage() {
    OIDCAccessTokenMapper mapper = new AuditStorageTokenMapper(true, () -> false);
    var error = assertThrows(ErrorResponseException.class, () -> mapper.transformAccessToken(
        new AccessToken(), new ProtocolMapperModel(), session("tenant-acme-event-election"), null, null));
    assertEquals("temporarily_unavailable", error.getError());
    assertEquals(503, error.getResponse().getStatus());
  }
  @Test void databaseFailureIsClosed() {
    var mapper = new AuditStorageTokenMapper(true, () -> {throw new SQLException("offline");});
    assertThrows(ErrorResponseException.class, () -> mapper.transformAccessToken(
        new AccessToken(), new ProtocolMapperModel(), session("tenant-acme-event-election"), null, null));
  }
  @Test void freshAvailableStorageAllowsAnAccessToken() {
    var token = new AccessToken();
    var mapper = new AuditStorageTokenMapper(true, () -> true);
    assertSame(token, mapper.transformAccessToken(token, new ProtocolMapperModel(), session("tenant-acme-event-election"), null, null));
  }
  @Test void staffRemainAvailableForRecoveryWithoutDatabaseRead() {
    var mapper = new AuditStorageTokenMapper(true, () -> {fail("Staff must not require voter storage");return false;});
    var token = new AccessToken();
    assertSame(token, mapper.transformAccessToken(token, new ProtocolMapperModel(), session("tenant-acme"), null, null));
  }
  @Test void disabledIsExplicitAndInvalidSettingsAreRejected() {
    assertFalse(AuditStorageTokenMapper.required(Map.of()));
    assertFalse(AuditStorageTokenMapper.required(Map.of("AUDIT_STORAGE_POLICY","disabled")));
    assertTrue(AuditStorageTokenMapper.required(Map.of("AUDIT_STORAGE_POLICY","required")));
    assertThrows(IllegalStateException.class, () -> AuditStorageTokenMapper.required(Map.of("AUDIT_STORAGE_POLICY","optional")));
  }
  @Test void disabledPolicyDoesNotReadStorageEvenForEventRealms() {
    var mapper = new AuditStorageTokenMapper(false, () -> {fail("Disabled policy must not require database access");return false;});
    var token = new AccessToken();
    assertSame(token,mapper.transformAccessToken(token,new ProtocolMapperModel(),session("tenant-acme-event-election"),null,null));
  }
  @Test void clientConfigurationCannotDisableRequiredTokenGuard() {
    var model=new ProtocolMapperModel();model.setConfig(Map.of("access.token.claim","false"));
    var mapper = new AuditStorageTokenMapper(true, () -> false);
    assertThrows(ErrorResponseException.class, () -> mapper.transformAccessToken(new AccessToken(),model,session("tenant-acme-event-election"),null,null));
  }
  @Test void realmDetectionUsesServerRealmAndRejectsMissingEvent() {
    assertTrue(AuditStorageTokenMapper.eventRealm("tenant-acme-event-election"));
    assertFalse(AuditStorageTokenMapper.eventRealm("tenant-acme"));
    assertFalse(AuditStorageTokenMapper.eventRealm("tenant-acme-event-"));
    assertFalse(AuditStorageTokenMapper.eventRealm(null));
  }
}
