// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.custom_event_listener;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.ArgumentMatchers.*;
import static org.mockito.Mockito.*;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.sql.SQLException;
import java.util.Base64;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakTransaction;
import org.keycloak.models.KeycloakTransactionManager;
import org.mockito.ArgumentCaptor;

class PgmqEventPublisherTest {
  private static final String TASK = "enqueue_electoral_log_event";
  private static final byte[] BODY = "[[],{\"input\":{}},{}]".getBytes(StandardCharsets.UTF_8);

  private final ObjectMapper mapper = new ObjectMapper();
  private KeycloakSession session;
  private KeycloakTransactionManager transaction;
  private QueueSender sender;

  @BeforeEach
  void setUp() {
    session = mock(KeycloakSession.class);
    transaction = mock(KeycloakTransactionManager.class);
    sender = mock(QueueSender.class);
    when(session.getTransactionManager()).thenReturn(transaction);
    when(transaction.isActive()).thenReturn(true);
  }

  private KeycloakTransaction enlisted(PublishFailurePolicy policy) {
    new PgmqEventPublisher(sender, policy).publish(session, "event-id", TASK, BODY);
    ArgumentCaptor<KeycloakTransaction> enqueue =
        ArgumentCaptor.forClass(KeycloakTransaction.class);
    switch (policy) {
      case FAIL_REQUEST -> {
        verify(transaction).enlistPrepare(enqueue.capture());
        verify(transaction, never()).enlistAfterCompletion(any());
      }
      case LOG_AND_CONTINUE -> {
        verify(transaction).enlistAfterCompletion(enqueue.capture());
        verify(transaction, never()).enlistPrepare(any());
      }
    }
    return enqueue.getValue();
  }

  @Test
  void failRequestEnqueuesBeforeKeycloakCommits() throws Exception {
    KeycloakTransaction enqueue = enlisted(PublishFailurePolicy.FAIL_REQUEST);
    verifyNoInteractions(sender);
    enqueue.commit();
    ArgumentCaptor<String> payload = ArgumentCaptor.forClass(String.class);
    verify(sender).send(eq(PgmqEventPublisher.QUEUE), payload.capture());
    JsonNode envelope = mapper.readTree(payload.getValue());
    assertArrayEquals(BODY, Base64.getDecoder().decode(envelope.get("body").asText()));
    assertEquals("event-id", envelope.at("/headers/id").asText());
    assertEquals(TASK, envelope.at("/headers/task").asText());
  }

  @Test
  void failRequestFailsTheRequestWhenTheEventCannotBeEnqueued() throws Exception {
    KeycloakTransaction enqueue = enlisted(PublishFailurePolicy.FAIL_REQUEST);
    doThrow(new SQLException("queue database unavailable")).when(sender).send(any(), any());
    assertThrows(IllegalStateException.class, enqueue::commit);
  }

  @Test
  void logAndContinueEnqueuesAfterKeycloakCommitsAndOnlyLogsFailures() throws Exception {
    KeycloakTransaction enqueue = enlisted(PublishFailurePolicy.LOG_AND_CONTINUE);
    verifyNoInteractions(sender);
    doThrow(new SQLException("queue database unavailable")).when(sender).send(any(), any());
    assertDoesNotThrow(enqueue::commit);
    verify(sender).send(eq(PgmqEventPublisher.QUEUE), anyString());
  }

  @Test
  void rolledBackRequestsEnqueueNothing() {
    for (PublishFailurePolicy policy : PublishFailurePolicy.values()) {
      setUp();
      enlisted(policy).rollback();
      verifyNoInteractions(sender);
    }
  }

  @Test
  void eventsOutsideATransactionAreEnqueuedImmediately() throws Exception {
    when(transaction.isActive()).thenReturn(false);
    new PgmqEventPublisher(sender, PublishFailurePolicy.FAIL_REQUEST)
        .publish(session, "event-id", TASK, BODY);
    verify(sender).send(eq(PgmqEventPublisher.QUEUE), anyString());
    verify(transaction, never()).enlistPrepare(any());
  }

  /** The fixture is also decoded by Windmill's tests, so both sides agree on the format. */
  @Test
  void envelopeMatchesTheContractFixture() throws Exception {
    JsonNode expected;
    try (InputStream fixture =
        getClass().getResourceAsStream("/electoral-log-event-envelope.json")) {
      expected = mapper.readTree(fixture);
    }
    String id = expected.at("/headers/id").asText();
    byte[] body =
        mapper.writeValueAsBytes(
            CustomEventListenerProvider.taskMessage(
                "6f1c3a6e-2a61-4d6b-9a52-3f0f8a3c2b10",
                "LOGIN",
                null,
                "0b9f6c5e-7d3a-4a1e-8c2f-5e4d3c2b1a09",
                "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                "voter@example.com"));
    ObjectNode actual =
        (ObjectNode)
            mapper.readTree(
                new PgmqEventPublisher(sender, PublishFailurePolicy.FAIL_REQUEST)
                    .envelope(id, TASK, body));

    assertEquals(decodedBody(expected), decodedBody(actual));
    ((ObjectNode) expected).remove("body");
    actual.remove("body");
    assertEquals(expected, actual);
  }

  private JsonNode decodedBody(JsonNode envelope) throws Exception {
    return mapper.readTree(Base64.getDecoder().decode(envelope.get("body").asText()));
  }
}
