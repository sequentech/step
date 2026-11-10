// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.custom_event_listener;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

import java.sql.SQLException;
import java.util.Map;
import java.util.stream.Stream;
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

  private UserSessionModel account(KeycloakSession session, String serviceClient, String... roles) {
    RealmModel realm = session.getContext().getRealm();
    UserModel user = mock(UserModel.class);
    when(user.getServiceAccountClientLink()).thenReturn(serviceClient);
    for (String name : roles) {
      RoleModel role = mock(RoleModel.class);
      when(realm.getRole(name)).thenReturn(role);
      when(user.hasRole(role)).thenReturn(true);
    }
    UserSessionModel userSession = mock(UserSessionModel.class);
    when(userSession.getUser()).thenReturn(user);
    return userSession;
  }

  private AuditStorageTokenMapper withoutStorageRead() {
    return new AuditStorageTokenMapper(
        true,
        () -> {
          fail("Recovery accounts must not require a storage read");
          return false;
        });
  }

  @Test
  void everyVoterTokenTransformationRefusesUnavailableStorage() {
    OIDCAccessTokenMapper mapper = new AuditStorageTokenMapper(true, () -> false);
    var error =
        assertThrows(
            ErrorResponseException.class,
            () ->
                mapper.transformAccessToken(
                    new AccessToken(),
                    new ProtocolMapperModel(),
                    session("tenant-acme-event-election"),
                    null,
                    null));
    assertEquals("temporarily_unavailable", error.getError());
    assertEquals(503, error.getResponse().getStatus());
  }

  @Test
  void databaseFailureIsClosed() {
    var mapper =
        new AuditStorageTokenMapper(
            true,
            () -> {
              throw new SQLException("offline");
            });
    assertThrows(
        ErrorResponseException.class,
        () ->
            mapper.transformAccessToken(
                new AccessToken(),
                new ProtocolMapperModel(),
                session("tenant-acme-event-election"),
                null,
                null));
  }

  @Test
  void freshAvailableStorageAllowsAnAccessToken() {
    var token = new AccessToken();
    var mapper = new AuditStorageTokenMapper(true, () -> true);
    assertSame(
        token,
        mapper.transformAccessToken(
            token, new ProtocolMapperModel(), session("tenant-acme-event-election"), null, null));
  }

  @Test
  void administratorsAndBoardMembersRemainAvailableForRecoveryWithoutDatabaseRead() {
    var mapper = withoutStorageRead();
    var admin = session("tenant-acme");
    var token = new AccessToken();
    assertSame(
        token,
        mapper.transformAccessToken(
            token,
            new ProtocolMapperModel(),
            admin,
            account(admin, null, "admin-user", "tenant-write"),
            null));
    var board = session("tenant-acme");
    assertSame(
        token,
        mapper.transformAccessToken(
            token,
            new ProtocolMapperModel(),
            board,
            account(board, null, "election-state-write", "voter-read"),
            null));
  }

  @Test
  void tenantServiceAccountsAreNotRefused() {
    var service = session("tenant-acme");
    var token = new AccessToken();
    assertSame(
        token,
        withoutStorageRead()
            .transformAccessToken(
                token, new ProtocolMapperModel(), service, account(service, "client-id"), null));
  }

  @Test
  void trusteeAndOtherTenantTokensAreRefusedWhenStorageIsUnavailable() {
    var mapper = new AuditStorageTokenMapper(true, () -> false);
    for (String[] roles :
        new String[][] {
          {"trustee-ceremony", "sign-key-ceremony", "sign-tally-key"},
          {"election-state-write", "admin-user"},
          {}
        }) {
      var trustee = session("tenant-acme");
      var error =
          assertThrows(
              ErrorResponseException.class,
              () ->
                  mapper.transformAccessToken(
                      new AccessToken(),
                      new ProtocolMapperModel(),
                      trustee,
                      account(trustee, null, roles),
                      null));
      assertEquals("temporarily_unavailable", error.getError());
      assertEquals(
          "Sign-in is temporarily unavailable. Please try again later.",
          error.getErrorDescription());
      assertEquals(503, error.getResponse().getStatus());
    }
  }

  @Test
  void tenantDatabaseFailureIsClosedAndAvailableStorageAdmits() {
    var trustee = session("tenant-acme");
    var user = account(trustee, null, "trustee-ceremony");
    var failing =
        new AuditStorageTokenMapper(
            true,
            () -> {
              throw new SQLException("offline");
            });
    assertThrows(
        ErrorResponseException.class,
        () ->
            failing.transformAccessToken(
                new AccessToken(), new ProtocolMapperModel(), trustee, user, null));
    var token = new AccessToken();
    assertSame(
        token,
        new AuditStorageTokenMapper(true, () -> true)
            .transformAccessToken(token, new ProtocolMapperModel(), trustee, user, null));
  }

  @Test
  void otherRealmsAreNotGuarded() {
    var token = new AccessToken();
    assertSame(
        token,
        withoutStorageRead()
            .transformAccessToken(token, new ProtocolMapperModel(), session("master"), null, null));
  }

  @Test
  void guardIsInstalledOnEventAndTenantRealmClientsOnly() {
    for (String name : new String[] {"tenant-acme", "tenant-acme-event-election", "master"}) {
      RealmModel realm = mock(RealmModel.class);
      when(realm.getName()).thenReturn(name);
      ClientModel client = mock(ClientModel.class);
      when(client.getRealm()).thenReturn(realm);
      when(client.getProtocol()).thenReturn("openid-connect");
      when(client.getProtocolMappersStream()).thenReturn(Stream.empty());
      AuditStorageTokenMapper.install(client);
      verify(client, times("master".equals(name) ? 0 : 1))
          .addProtocolMapper(
              argThat(
                  model -> AuditStorageTokenMapper.PROVIDER_ID.equals(model.getProtocolMapper())));
    }
  }

  @Test
  void disabledIsExplicitAndInvalidSettingsAreRejected() {
    assertFalse(AuditStorageTokenMapper.required(Map.of()));
    assertFalse(AuditStorageTokenMapper.required(Map.of("AUDIT_STORAGE_POLICY", "disabled")));
    assertTrue(AuditStorageTokenMapper.required(Map.of("AUDIT_STORAGE_POLICY", "required")));
    assertThrows(
        IllegalStateException.class,
        () -> AuditStorageTokenMapper.required(Map.of("AUDIT_STORAGE_POLICY", "optional")));
  }

  @Test
  void disabledPolicyDoesNotReadStorageEvenForEventRealms() {
    var mapper =
        new AuditStorageTokenMapper(
            false,
            () -> {
              fail("Disabled policy must not require database access");
              return false;
            });
    var token = new AccessToken();
    assertSame(
        token,
        mapper.transformAccessToken(
            token, new ProtocolMapperModel(), session("tenant-acme-event-election"), null, null));
  }

  @Test
  void clientConfigurationCannotDisableRequiredTokenGuard() {
    var model = new ProtocolMapperModel();
    model.setConfig(Map.of("access.token.claim", "false"));
    var mapper = new AuditStorageTokenMapper(true, () -> false);
    assertThrows(
        ErrorResponseException.class,
        () ->
            mapper.transformAccessToken(
                new AccessToken(), model, session("tenant-acme-event-election"), null, null));
  }

  @Test
  void realmDetectionUsesServerRealmAndRejectsMissingEvent() {
    assertTrue(AuditStorageTokenMapper.eventRealm("tenant-acme-event-election"));
    assertFalse(AuditStorageTokenMapper.eventRealm("tenant-acme"));
    assertFalse(AuditStorageTokenMapper.eventRealm("tenant-acme-event-"));
    assertFalse(AuditStorageTokenMapper.eventRealm(null));
    assertTrue(AuditStorageTokenMapper.protectedRealm("tenant-acme"));
    assertTrue(AuditStorageTokenMapper.protectedRealm("tenant-acme-event-election"));
    assertFalse(AuditStorageTokenMapper.protectedRealm("tenant-acme-event-"));
    assertFalse(AuditStorageTokenMapper.protectedRealm("master"));
    assertFalse(AuditStorageTokenMapper.protectedRealm(null));
  }
}
