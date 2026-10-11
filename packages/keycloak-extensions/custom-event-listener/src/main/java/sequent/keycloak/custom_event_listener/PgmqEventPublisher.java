// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.util.Arrays;
import java.util.Base64;
import java.util.Map;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakTransaction;
import org.keycloak.models.KeycloakTransactionManager;

/**
 * Enqueues electoral-log events in the environment's task-queue database. That database is not
 * Keycloak's, so the event is sent when Keycloak commits the request, as the {@link
 * PublishFailurePolicy} says.
 */
@JBossLog
final class PgmqEventPublisher implements AutoCloseable {
  static final String QUEUE = "electoral_log_event_queue";

  private final QueueSender sender;
  private final PublishFailurePolicy policy;
  private final ObjectMapper mapper = new ObjectMapper();

  PgmqEventPublisher(QueueSender sender, PublishFailurePolicy policy) {
    this.sender = sender;
    this.policy = policy;
  }

  static PgmqEventPublisher fromEnvironment() {
    Map<String, String> environment = System.getenv();
    PublishFailurePolicy policy = PublishFailurePolicy.fromEnvironment(environment);
    QueueDatabase database =
        QueueDatabase.connect(QueueDatabaseSettings.fromEnvironment(environment));
    log.infov("Electoral-log events go to the task-queue database ({0})", policy.value());
    return new PgmqEventPublisher(database, policy);
  }

  /** Log whether events can be enqueued; the database may be set up after Keycloak starts. */
  void checkQueueDatabase() {
    try {
      sender.verify();
    } catch (Exception exception) {
      log.warnv(
          "The task-queue database cannot take electoral-log events yet: {0}",
          exception.getMessage());
    }
  }

  void publish(KeycloakSession session, String taskId, String taskName, byte[] body) {
    String payload;
    try {
      payload = envelope(taskId, taskName, body);
    } catch (JsonProcessingException exception) {
      throw new IllegalStateException("Unable to encode electoral audit event", exception);
    }
    KeycloakTransactionManager transaction = session.getTransactionManager();
    if (!transaction.isActive()) {
      send(taskId, payload);
      return;
    }
    switch (policy) {
      case FAIL_REQUEST -> transaction.enlistPrepare(new Enqueue(taskId, payload));
      case LOG_AND_CONTINUE -> transaction.enlistAfterCompletion(new Enqueue(taskId, payload));
    }
  }

  /** The Celery message that the Rust broker decodes. */
  String envelope(String taskId, String taskName, byte[] body) throws JsonProcessingException {
    Map<String, Object> headers =
        Map.of("id", taskId, "task", taskName, "timelimit", Arrays.asList(null, null));
    Map<String, Object> properties =
        Map.of("correlation_id", taskId, "delivery_tag", taskId, "body_encoding", "base64");
    return mapper.writeValueAsString(
        Map.of(
            "body",
            Base64.getEncoder().encodeToString(body),
            "content-encoding",
            "utf-8",
            "content-type",
            "application/json",
            "headers",
            headers,
            "properties",
            properties));
  }

  private void send(String taskId, String payload) {
    try {
      sender.send(QUEUE, payload);
    } catch (Exception exception) {
      switch (policy) {
        case FAIL_REQUEST ->
            throw new IllegalStateException(
                "Unable to enqueue electoral audit event " + taskId, exception);
        case LOG_AND_CONTINUE ->
            log.errorv(
                exception,
                "Unable to enqueue electoral audit event {0}; it is not in the electoral log."
                    + " Its message, to send to {1}: {2}",
                taskId,
                QUEUE,
                payload);
      }
    }
  }

  @Override
  public void close() {
    sender.close();
  }

  /** Sends the event when Keycloak commits the request; a rolled-back request sends nothing. */
  private final class Enqueue implements KeycloakTransaction {
    private final String taskId;
    private final String payload;
    private boolean active;
    private boolean rollbackOnly;

    private Enqueue(String taskId, String payload) {
      this.taskId = taskId;
      this.payload = payload;
    }

    @Override
    public void begin() {
      active = true;
    }

    @Override
    public void commit() {
      active = false;
      send(taskId, payload);
    }

    @Override
    public void rollback() {
      active = false;
    }

    @Override
    public void setRollbackOnly() {
      rollbackOnly = true;
    }

    @Override
    public boolean getRollbackOnly() {
      return rollbackOnly;
    }

    @Override
    public boolean isActive() {
      return active;
    }
  }
}
