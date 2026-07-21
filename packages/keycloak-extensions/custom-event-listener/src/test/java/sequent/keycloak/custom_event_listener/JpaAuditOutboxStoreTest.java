// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.mockito.Mockito.doThrow;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import jakarta.persistence.EntityManager;
import jakarta.persistence.PersistenceException;
import java.nio.charset.StandardCharsets;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.connections.jpa.JpaConnectionProvider;
import org.keycloak.models.KeycloakSession;
import org.mockito.ArgumentCaptor;

class JpaAuditOutboxStoreTest {

  private EntityManager entityManager;
  private KeycloakSession session;
  private JpaAuditOutboxStore store;
  private RabbitMqOutboxMessage message;

  @BeforeEach
  void setUp() {
    entityManager = mock(EntityManager.class);
    JpaConnectionProvider jpaConnectionProvider = mock(JpaConnectionProvider.class);
    session = mock(KeycloakSession.class);
    when(session.getProvider(JpaConnectionProvider.class)).thenReturn(jpaConnectionProvider);
    when(jpaConnectionProvider.getEntityManager()).thenReturn(entityManager);
    store = new JpaAuditOutboxStore();
    message =
        new RabbitMqOutboxMessage(
            "event-id",
            "audit-queue",
            "audit-task",
            "event".getBytes(StandardCharsets.UTF_8),
            0,
            null);
  }

  @Test
  void persistsAndFlushesBeforeReturningToKeycloak() {
    store.enqueue(session, message);

    ArgumentCaptor<AuditOutboxEntity> entityCaptor =
        ArgumentCaptor.forClass(AuditOutboxEntity.class);
    verify(entityManager).persist(entityCaptor.capture());
    verify(entityManager).flush();
    AuditOutboxEntity entity = entityCaptor.getValue();
    assertEquals("event-id", entity.getId());
    assertEquals("audit-queue", entity.getQueueName());
    assertEquals("audit-task", entity.getTaskName());
    assertArrayEquals(message.body(), entity.getBody());
  }

  @Test
  void propagatesDurabilityFailureToTheKeycloakRequest() {
    PersistenceException databaseFailure = new PersistenceException("database unavailable");
    doThrow(databaseFailure).when(entityManager).flush();

    AuditEventPersistenceException exception =
        assertThrows(AuditEventPersistenceException.class, () -> store.enqueue(session, message));
    assertEquals(databaseFailure, exception.getCause());
  }
}
