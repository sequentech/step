// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import com.rabbitmq.client.AMQP;
import com.rabbitmq.client.ConnectionFactory;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;

class RabbitMqEventPublisher implements AutoCloseable {

  private static final String DEFAULT_QUEUE_NAME = "electoral_log_event_queue";
  private static final String TASK_HEADER = "task";

  private final String queueName;
  private final AuditOutboxStore outboxStore;
  private final RabbitMqOutboxWorker worker;

  static RabbitMqEventPublisher fromEnvironment() {
    String envSlug = System.getenv("ENV_SLUG");
    String baseQueueName =
        Optional.ofNullable(System.getenv("ELECTORAL_LOG_QUEUE")).orElse(DEFAULT_QUEUE_NAME).trim();
    String queueName =
        envSlug == null || envSlug.trim().isEmpty()
            ? baseQueueName
            : envSlug.trim() + "_" + baseQueueName;
    AuditOutboxStore outboxStore = new JpaAuditOutboxStore();
    RabbitMqOutboxWorker worker =
        new RabbitMqOutboxWorker(System.getenv("AMQP_ADDR"), outboxStore, ConnectionFactory::new);
    return new RabbitMqEventPublisher(queueName, outboxStore, worker);
  }

  RabbitMqEventPublisher(
      String queueName, AuditOutboxStore outboxStore, RabbitMqOutboxWorker worker) {
    this.queueName = Objects.requireNonNull(queueName, "queueName");
    this.outboxStore = Objects.requireNonNull(outboxStore, "outboxStore");
    this.worker = Objects.requireNonNull(worker, "worker");
  }

  void publish(KeycloakSession session, AMQP.BasicProperties properties, byte[] body) {
    String correlationId = requireValue(properties.getCorrelationId(), "correlation ID");
    Map<String, Object> headers = properties.getHeaders();
    Object taskHeader = headers == null ? null : headers.get(TASK_HEADER);
    String taskName = requireValue(taskHeader == null ? null : taskHeader.toString(), "task name");
    outboxStore.enqueue(
        session, new RabbitMqOutboxMessage(correlationId, queueName, taskName, body, 0, null));
  }

  void start(KeycloakSessionFactory sessionFactory) {
    worker.start(sessionFactory);
  }

  @Override
  public void close() {
    worker.close();
  }

  private static String requireValue(String value, String name) {
    if (value == null || value.isBlank()) {
      throw new AuditEventPersistenceException("Audit event " + name + " is missing");
    }
    return value;
  }
}
