// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

import com.fasterxml.jackson.databind.ObjectMapper;
import jakarta.persistence.EntityManager;
import jakarta.persistence.TypedQuery;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.ServiceLoader;
import java.util.stream.Stream;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.connections.jpa.JpaConnectionProvider;
import org.keycloak.events.Event;
import org.keycloak.events.EventQuery;
import org.keycloak.events.EventStoreProvider;
import org.keycloak.events.EventStoreProviderFactory;
import org.keycloak.events.EventType;
import org.keycloak.events.admin.AdminEvent;
import org.keycloak.events.admin.AdminEventQuery;
import org.keycloak.events.admin.AuthDetails;
import org.keycloak.events.admin.OperationType;
import org.keycloak.events.admin.ResourceType;
import org.keycloak.events.jpa.JpaEventStoreProviderFactory;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.RealmModel;
import org.keycloak.models.RealmProvider;
import org.keycloak.models.jpa.entities.RealmAttributeEntity;
import org.keycloak.storage.datastore.PeriodicEventInvalidation;
import org.mockito.ArgumentCaptor;

class PrivacyEventStoreProviderTest {
  private final KeycloakSession session = mock(KeycloakSession.class);
  private final EventStoreProvider jpa = mock(EventStoreProvider.class);
  private final RealmProvider realms = mock(RealmProvider.class);
  private final RealmModel realm = mock(RealmModel.class);
  private final EntityManager entityManager = mock(EntityManager.class);

  @BeforeEach
  void configureRealm() {
    when(session.realms()).thenReturn(realms);
    when(realms.getRealm("event-realm-id")).thenReturn(realm);
    when(realm.getName())
        .thenReturn(
            "tenant-11111111-1111-4111-8111-111111111111-event-22222222-2222-4222-8222-222222222222");
    when(session.getProvider(EventStoreProvider.class, "jpa")).thenReturn(jpa);
    JpaConnectionProvider connection = mock(JpaConnectionProvider.class);
    when(session.getProvider(JpaConnectionProvider.class)).thenReturn(connection);
    when(connection.getEntityManager()).thenReturn(entityManager);
  }

  private EventStoreProviderFactory factory() {
    return ServiceLoader.load(EventStoreProviderFactory.class).stream()
        .map(ServiceLoader.Provider::get)
        .filter(candidate -> "sequent-privacy".equals(candidate.getId()))
        .findFirst()
        .orElse(null);
  }

  private EventStoreProvider store() {
    // Before the privacy factory is registered, the actual default is the raw JPA store.
    EventStoreProviderFactory configured = factory();
    return configured == null ? jpa : configured.create(session);
  }

  private Event event(EventType type) {
    Event event = new Event();
    event.setId("login-event");
    event.setRealmId("event-realm-id");
    event.setRealmName("master"); // The authoritative realm lookup must win.
    event.setUserId("voter");
    event.setClientId("voting-portal");
    event.setSessionId("session");
    event.setType(type);
    event.setTime(1700000000000L);
    event.setIpAddress("198.51.100.7");
    event.setDetails(
        new LinkedHashMap<>(
            Map.of(
                "auth_method", "openid-connect",
                "username", "voter-name",
                "country", "PH",
                "user_agent", "private-browser",
                "deviceId", "private-device",
                "unexpected_extension", "another-private-browser",
                "redirect_uri", "https://198.51.100.8/callback")));
    if (type == EventType.LOGIN_ERROR) event.setError("invalid_user_credentials");
    return event;
  }

  @Test
  void persistsLoginAuditCopyAndLeavesOriginalForForensics() {
    assertSafeAuditCopy(EventType.LOGIN);
  }

  @Test
  void persistsLoginFailureAuditCopyAndLeavesOriginalForForensics() {
    assertSafeAuditCopy(EventType.LOGIN_ERROR);
  }

  private void assertSafeAuditCopy(EventType type) {
    Event original = event(type);
    store().onEvent(original);
    ArgumentCaptor<Event> persisted = ArgumentCaptor.forClass(Event.class);
    verify(jpa).onEvent(persisted.capture());
    Event saved = persisted.getValue();
    assertNull(saved.getIpAddress(), "ordinary JPA storage must never retain voter IP");
    assertNotSame(original, saved);
    assertNotSame(original.getDetails(), saved.getDetails());
    assertEquals(
        Map.of("auth_method", "openid-connect", "username", "voter-name"), saved.getDetails());
    assertEquals(original.getId(), saved.getId());
    assertEquals(original.getTime(), saved.getTime());
    assertEquals(original.getRealmId(), saved.getRealmId());
    assertEquals(original.getUserId(), saved.getUserId());
    assertEquals(original.getClientId(), saved.getClientId());
    assertEquals(original.getType(), saved.getType());
    assertEquals(original.getError(), saved.getError());
    assertEquals("198.51.100.7", original.getIpAddress());
    assertEquals("private-browser", original.getDetails().get("user_agent"));
  }

  @Test
  void unresolvedRealmCannotFallBackToRawStorage() {
    when(realms.getRealm("event-realm-id")).thenReturn(null);
    Event original = event(EventType.LOGIN);
    store().onEvent(original);
    ArgumentCaptor<Event> persisted = ArgumentCaptor.forClass(Event.class);
    verify(jpa).onEvent(persisted.capture());
    assertNull(persisted.getValue().getIpAddress());
    assertFalse(persisted.getValue().getDetails().containsKey("country"));
  }

  @Test
  void otherRealmsKeepTheirConfiguredAudit() {
    when(realm.getName()).thenReturn("master");
    Event original = event(EventType.LOGIN);
    store().onEvent(original);
    verify(jpa).onEvent(original);
  }

  @Test
  void persistenceFailurePropagatesWithoutRetryingTheRawEvent() {
    doThrow(new IllegalStateException("synthetic persistence failure"))
        .when(jpa)
        .onEvent(any(Event.class));
    Event original = event(EventType.LOGIN);
    IllegalStateException failure =
        assertThrows(IllegalStateException.class, () -> store().onEvent(original));
    assertEquals("synthetic persistence failure", failure.getMessage());
    ArgumentCaptor<Event> persisted = ArgumentCaptor.forClass(Event.class);
    verify(jpa, times(1)).onEvent(persisted.capture());
    assertNull(persisted.getValue().getIpAddress());
    assertEquals("198.51.100.7", original.getIpAddress());
  }

  @Test
  void disabledAdminRepresentationStaysDisabled() {
    AdminEvent original = new AdminEvent();
    original.setRealmId("event-realm-id");
    original.setRepresentation("{\"enabled\":true}");
    store().onEvent(original, false);
    ArgumentCaptor<AdminEvent> persisted = ArgumentCaptor.forClass(AdminEvent.class);
    verify(jpa).onEvent(persisted.capture(), eq(false));
    assertNull(persisted.getValue().getRepresentation());
    assertEquals("{\"enabled\":true}", original.getRepresentation());
  }

  @Test
  void adminCopyPreservesOperationAndSafeRepresentationWithoutNetworkOrSecrets() throws Exception {
    AdminEvent original = new AdminEvent();
    original.setRealmId("event-realm-id");
    original.setResourceType(ResourceType.USER);
    original.setResourcePath("users/voter");
    original.setOperationType(OperationType.UPDATE);
    original.setTime(1700000000000L);
    original.setDetails(Map.of("auth_method", "openid-connect", "country", "PH"));
    original.setRepresentation(
        "{\"username\":\"voter\",\"enabled\":false,\"attributes\":{\"area\":[\"north\"],\"user_agent\":[\"private-browser\"],\"ip\":\"198.51.100.7\"},\"credentials\":[{\"value\":\"private-secret\"}]}");
    AuthDetails auth = new AuthDetails();
    auth.setUserId("administrator");
    auth.setRealmId("tenant-realm");
    auth.setIpAddress("198.51.100.9");
    original.setAuthDetails(auth);
    store().onEvent(original, true);
    ArgumentCaptor<AdminEvent> persisted = ArgumentCaptor.forClass(AdminEvent.class);
    verify(jpa).onEvent(persisted.capture(), eq(true));
    AdminEvent saved = persisted.getValue();
    assertNull(saved.getAuthDetails().getIpAddress());
    assertEquals("administrator", saved.getAuthDetails().getUserId());
    assertEquals(original.getOperationType(), saved.getOperationType());
    assertEquals(original.getResourceType(), saved.getResourceType());
    assertEquals(original.getResourcePath(), saved.getResourcePath());
    assertEquals(Map.of("auth_method", "openid-connect"), saved.getDetails());
    var representation = new ObjectMapper().readTree(saved.getRepresentation());
    assertEquals("voter", representation.path("username").asText());
    assertFalse(representation.path("enabled").asBoolean());
    assertEquals("north", representation.path("attributes").path("area").get(0).asText());
    assertFalse(saved.getRepresentation().contains("private-"));
    assertFalse(saved.getRepresentation().contains("198.51.100"));
    assertEquals("198.51.100.9", original.getAuthDetails().getIpAddress());
    assertTrue(original.getRepresentation().contains("private-browser"));
  }

  @Test
  void forwardsQueriesRetentionAndAdminRepresentationFlagWithoutClosingSessionOwnedJpa() {
    EventStoreProvider store = store();
    EventQuery query = mock(EventQuery.class);
    AdminEventQuery adminQuery = mock(AdminEventQuery.class);
    when(jpa.createQuery()).thenReturn(query);
    when(jpa.createAdminQuery()).thenReturn(adminQuery);
    assertSame(query, store.createQuery());
    assertSame(adminQuery, store.createAdminQuery());
    store.clear();
    store.clear(realm);
    store.clear(realm, 123L);
    store.clearExpiredEvents();
    store.clearAdmin();
    store.clearAdmin(realm);
    store.clearAdmin(realm, 456L);
    verify(jpa).clear();
    verify(jpa).clear(realm);
    verify(jpa).clear(realm, 123L);
    verify(jpa).clearExpiredEvents();
    verify(jpa).clearAdmin();
    verify(jpa).clearAdmin(realm);
    verify(jpa).clearAdmin(realm, 456L);
    store.close();
    verify(jpa, never()).close();
  }

  @Test
  @SuppressWarnings("unchecked")
  void pinnedJpaMaintenanceCanStillExpireAdminEventsThroughTheDefaultProvider() {
    EventStoreProvider provider = store();
    when(session.getProvider(EventStoreProvider.class)).thenReturn(provider);
    TypedQuery<RealmAttributeEntity> query = mock(TypedQuery.class);
    when(entityManager.createNamedQuery(
            "selectRealmAttributesNotEmptyByName", RealmAttributeEntity.class))
        .thenReturn(query);
    when(query.setParameter("name", "adminEventsExpiration")).thenReturn(query);
    when(query.getResultStream()).thenReturn(Stream.empty());
    new JpaEventStoreProviderFactory()
        .invalidate(session, PeriodicEventInvalidation.JPA_EVENT_STORE);
    verify(query).getResultStream();
  }

  @Test
  void listenerFactoryRejectsRawJpaEvenWhenProviderSelectionUnloadsThePrivacyFactory() {
    KeycloakSessionFactory sessions = mock(KeycloakSessionFactory.class);
    when(sessions.getProviderFactory(EventStoreProvider.class))
        .thenReturn(new JpaEventStoreProviderFactory());
    CustomEventListenerProviderFactory listener =
        new CustomEventListenerProviderFactory(mock(PgmqEventPublisher.class));
    try {
      IllegalStateException failure =
          assertThrows(IllegalStateException.class, () -> listener.postInit(sessions));
      assertTrue(failure.getMessage().contains("sequent-privacy event store must be selected"));
    } finally {
      listener.close();
    }
  }

  @Test
  void startupRefusesAnInstalledWrapperThatIsNotTheSelectedDefault() {
    EventStoreProviderFactory provider = factory();
    assertNotNull(provider, "privacy store must be packaged as a Keycloak SPI");
    KeycloakSessionFactory sessions = mock(KeycloakSessionFactory.class);
    when(sessions.getProviderFactory(EventStoreProvider.class)).thenReturn(provider);
    when(sessions.getProviderFactory(EventStoreProvider.class, "jpa"))
        .thenReturn(new JpaEventStoreProviderFactory());
    assertDoesNotThrow(() -> provider.postInit(sessions));
    when(sessions.getProviderFactory(EventStoreProvider.class, "jpa")).thenReturn(null);
    assertThrows(IllegalStateException.class, () -> provider.postInit(sessions));
    when(sessions.getProviderFactory(EventStoreProvider.class))
        .thenReturn(mock(EventStoreProviderFactory.class));
    assertThrows(IllegalStateException.class, () -> provider.postInit(sessions));
  }
}
