// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.custom_event_listener;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.ArgumentMatchers.*;
import static org.mockito.Mockito.*;

import com.fasterxml.jackson.databind.ObjectMapper;
import jakarta.persistence.EntityManager;
import jakarta.persistence.Query;
import java.nio.charset.StandardCharsets;
import java.util.Base64;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.connections.jpa.JpaConnectionProvider;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakTransactionManager;
import org.mockito.ArgumentCaptor;

class PgmqEventPublisherTest {
  private KeycloakSession session;
  private Query query;
  private KeycloakTransactionManager transaction;
  private PgmqEventPublisher publisher;

  @BeforeEach
  void setUp() {
    session = mock(KeycloakSession.class);
    JpaConnectionProvider jpa = mock(JpaConnectionProvider.class);
    EntityManager entityManager = mock(EntityManager.class);
    query = mock(Query.class);
    transaction = mock(KeycloakTransactionManager.class);
    when(session.getProvider(JpaConnectionProvider.class)).thenReturn(jpa);
    when(session.getTransactionManager()).thenReturn(transaction);
    when(jpa.getEntityManager()).thenReturn(entityManager);
    when(entityManager.createNativeQuery(anyString())).thenReturn(query);
    when(query.setParameter(anyString(), any())).thenReturn(query);
    when(query.getSingleResult()).thenReturn(1L);
    publisher = new PgmqEventPublisher("dev_electoral_log_event_queue");
  }

  @Test
  void publishesCeleryEnvelopeThroughTheRequestTransaction() throws Exception {
    byte[] body = "[[],{\"input\":{}},{}]".getBytes(StandardCharsets.UTF_8);
    publisher.publish(session, "event-id", "enqueue_electoral_log_event", body);
    ArgumentCaptor<String> payload = ArgumentCaptor.forClass(String.class);
    verify(query).setParameter(eq("payload"), payload.capture());
    var envelope = new ObjectMapper().readTree(payload.getValue());
    assertArrayEquals(body, Base64.getDecoder().decode(envelope.get("body").asText()));
    assertEquals("event-id", envelope.at("/headers/id").asText());
    assertEquals("enqueue_electoral_log_event", envelope.at("/headers/task").asText());
    assertTrue(envelope.at("/headers/timelimit/0").isNull());
    assertTrue(envelope.at("/headers/timelimit/1").isNull());
    assertEquals("base64", envelope.at("/properties/body_encoding").asText());
    assertEquals("application/json", envelope.get("content-type").asText());
    verify(query).getSingleResult();
    verifyNoInteractions(transaction);
  }

  @Test
  void databaseFailureMarksTheRequestForRollback() {
    when(query.getSingleResult()).thenThrow(new IllegalStateException("database unavailable"));
    assertThrows(
        IllegalStateException.class,
        () -> publisher.publish(session, "event-id", "enqueue_electoral_log_event", new byte[0]));
    verify(transaction).setRollbackOnly();
  }
}
