// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import jakarta.persistence.Column;
import jakarta.persistence.Entity;
import jakarta.persistence.Id;
import jakarta.persistence.Lob;
import jakarta.persistence.NamedQuery;
import jakarta.persistence.Table;
import java.util.Base64;

@Entity
@Table(name = "SEQUENT_AUDIT_OUTBOX")
@NamedQuery(
    name = AuditOutboxEntity.FIND_READY,
    query =
        "SELECT event FROM AuditOutboxEntity event "
            + "WHERE event.availableAt <= :now "
            + "AND (event.claimedUntil IS NULL OR event.claimedUntil <= :now) "
            + "ORDER BY event.createdAt")
class AuditOutboxEntity {

  static final String FIND_READY = "AuditOutboxEntity.findReady";

  @Id
  @Column(name = "ID", length = 36, nullable = false)
  private String id;

  @Column(name = "QUEUE_NAME", length = 255, nullable = false)
  private String queueName;

  @Column(name = "TASK_NAME", length = 255, nullable = false)
  private String taskName;

  @Lob
  @Column(name = "PAYLOAD", nullable = false)
  private String encodedBody;

  @Column(name = "CREATED_AT", nullable = false)
  private long createdAt;

  @Column(name = "AVAILABLE_AT", nullable = false)
  private long availableAt;

  @Column(name = "CLAIMED_UNTIL")
  private Long claimedUntil;

  @Column(name = "CLAIM_TOKEN", length = 36)
  private String claimToken;

  @Column(name = "ATTEMPTS", nullable = false)
  private int attempts;

  @Column(name = "LAST_ERROR", length = 2048)
  private String lastError;

  protected AuditOutboxEntity() {}

  AuditOutboxEntity(RabbitMqOutboxMessage message, long now) {
    id = message.id();
    queueName = message.queueName();
    taskName = message.taskName();
    encodedBody = Base64.getEncoder().encodeToString(message.body());
    createdAt = now;
    availableAt = now;
    attempts = 0;
  }

  RabbitMqOutboxMessage claim(long leaseUntil, String token) {
    claimedUntil = leaseUntil;
    claimToken = token;
    attempts++;
    return new RabbitMqOutboxMessage(
        id, queueName, taskName, Base64.getDecoder().decode(encodedBody), attempts, claimToken);
  }

  String getId() {
    return id;
  }

  String getQueueName() {
    return queueName;
  }

  String getTaskName() {
    return taskName;
  }

  byte[] getBody() {
    return Base64.getDecoder().decode(encodedBody);
  }
}
