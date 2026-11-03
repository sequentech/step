// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

package org.keycloak.models.sessions.infinispan;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.ArgumentMatchers.*;
import static org.mockito.Mockito.*;

import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.*;
import org.keycloak.common.Profile;
import org.keycloak.common.util.MultiSiteUtils;
import org.keycloak.common.util.Time;
import org.keycloak.models.*;
import org.keycloak.models.sessions.infinispan.changes.*;
import org.keycloak.models.sessions.infinispan.entities.UserSessionEntity;
import org.mockito.ArgumentCaptor;
import org.mockito.MockedStatic;

class PrivacyPersistentUserSessionProviderTest {
  private MockedStatic<MultiSiteUtils> persistence;
  private MockedStatic<Profile> profile;
  private UserSessionPersistentChangelogBasedTransaction tx;
  private PrivacyPersistentUserSessionProvider provider;
  private RealmModel realm;
  private UserModel user;

  @BeforeEach
  void setup() {
    profile = mockStatic(Profile.class);
    persistence = mockStatic(MultiSiteUtils.class);
    persistence.when(MultiSiteUtils::isPersistentSessionsEnabled).thenReturn(true);
    KeycloakSession session = mock(KeycloakSession.class);
    when(session.getTransactionManager()).thenReturn(mock(KeycloakTransactionManager.class));
    var users = mock(UserProvider.class);
    when(session.users()).thenReturn(users);
    when(users.getUserById(any(), anyString())).thenAnswer(invocation -> user);
    tx = mock(UserSessionPersistentChangelogBasedTransaction.class);
    var builtIn =
        new PersistentUserSessionProvider(
            session, tx, mock(ClientSessionPersistentChangelogBasedTransaction.class));
    provider =
        new PrivacyPersistentUserSessionProvider(
            builtIn,
            r -> r.getName().contains("-event-"),
            name -> name.equals("ip") || name.equals("user_agent"));
    realm = mock(RealmModel.class);
    when(realm.getName()).thenReturn("tenant-test-event-test");
    when(realm.getId()).thenReturn("event-realm");
    when(realm.getSsoSessionMaxLifespan()).thenReturn(86400);
    when(realm.getSsoSessionIdleTimeout()).thenReturn(3600);
    user = mock(UserModel.class);
    when(user.getId()).thenReturn("voter");
  }

  @AfterEach
  void cleanup() {
    persistence.close();
    profile.close();
  }

  @Test
  void creationRemovesTheAddressBeforeTheStorageTransaction() {
    var created =
        provider.createUserSession(
            "id",
            realm,
            user,
            "voter",
            "198.51.100.7",
            "oidc",
            true,
            "b",
            "u",
            UserSessionModel.SessionPersistenceState.PERSISTENT);
    var entity = ArgumentCaptor.forClass(UserSessionEntity.class);
    verify(tx)
        .addTask(
            eq("id"),
            any(),
            entity.capture(),
            eq(UserSessionModel.SessionPersistenceState.PERSISTENT));
    assertNull(entity.getValue().getIpAddress());
    assertNull(created.getIpAddress());
    assertInstanceOf(UserSessionAdapter.class, created);
  }

  @Test
  void tenantSessionsRetainTheirExistingAddress() {
    when(realm.getName()).thenReturn("tenant-test");
    var created =
        provider.createUserSession(
            "tenant-id",
            realm,
            user,
            "admin",
            "198.51.100.7",
            "oidc",
            false,
            null,
            null,
            UserSessionModel.SessionPersistenceState.PERSISTENT);
    assertEquals("198.51.100.7", created.getIpAddress());
  }

  @Test
  void restartAndNotesPreserveTheConcreteAdapterAndRemoveNetworkData() {
    var entity = new UserSessionEntity("id");
    entity.setIpAddress("198.51.100.7");
    entity.setNotes(new HashMap<>(Map.of("user_agent", "private-browser", "auth_time", "123")));
    var session = provider.wrap(realm, entity, false, user);
    assertNull(session.getIpAddress());
    assertEquals(Map.of("auth_time", "123"), session.getNotes());
    assertNull(session.getNote("user_agent"));
    session.restartSession(realm, user, "voter", "198.51.100.8", "oidc", false, null, null);
    var task = ArgumentCaptor.forClass(UserSessionUpdateTask.class);
    verify(tx).restartEntity(eq("id"), task.capture());
    task.getValue().runUpdate(entity);
    assertNull(entity.getIpAddress());
  }

  @Test
  void persistentReloadSanitizesTheEntityBeforeItEntersTheCache() {
    var input = mock(UserSessionModel.class);
    when(input.getRealm()).thenReturn(realm);
    when(input.getId()).thenReturn("id");
    when(input.getUser()).thenReturn(user);
    when(input.getStarted()).thenReturn(Time.currentTime());
    when(input.getLastSessionRefresh()).thenReturn(Time.currentTime());
    when(input.getIpAddress()).thenReturn("198.51.100.7");
    when(input.getNotes()).thenReturn(Map.of("ip", "198.51.100.7", "auth_time", "123"));
    provider.wrapPersistentEntity(realm, false, input);
    var entity = ArgumentCaptor.forClass(UserSessionEntity.class);
    verify(tx)
        .addTask(
            eq("id"),
            isNull(),
            entity.capture(),
            eq(UserSessionModel.SessionPersistenceState.PERSISTENT));
    assertNull(entity.getValue().getIpAddress());
    assertEquals(Map.of("auth_time", "123"), entity.getValue().getNotes());
  }
}
