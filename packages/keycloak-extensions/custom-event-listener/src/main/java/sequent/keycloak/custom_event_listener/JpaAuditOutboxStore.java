// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import jakarta.persistence.EntityManager;
import jakarta.persistence.LockModeType;
import java.util.List;
import java.util.Optional;
import java.util.UUID;
import java.util.function.LongSupplier;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.connections.jpa.JpaConnectionProvider;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.utils.KeycloakModelUtils;

@JBossLog
class JpaAuditOutboxStore implements AuditOutboxStore {

  private static final int MAX_ERROR_LENGTH = 2048;

  private final LongSupplier clock;

  JpaAuditOutboxStore() {
    this(System::currentTimeMillis);
  }

  JpaAuditOutboxStore(LongSupplier clock) {
    this.clock = clock;
  }

  @Override
  public void enqueue(KeycloakSession session, RabbitMqOutboxMessage message) {
    try {
      EntityManager entityManager = getEntityManager(session);
      entityManager.persist(new AuditOutboxEntity(message, clock.getAsLong()));
      entityManager.flush();
    } catch (RuntimeException exception) {
      throw new AuditEventPersistenceException(
          "Failed to durably store audit event " + message.id(), exception);
    }
  }

  @Override
  public Optional<RabbitMqOutboxMessage> claimNext(
      KeycloakSessionFactory sessionFactory, long now, long leaseMillis) {
    return KeycloakModelUtils.runJobInTransactionWithResult(
        sessionFactory, session -> claimNext(session, now, leaseMillis));
  }

  Optional<RabbitMqOutboxMessage> claimNext(KeycloakSession session, long now, long leaseMillis) {
    List<AuditOutboxEntity> readyEvents =
        getEntityManager(session)
            .createNamedQuery(AuditOutboxEntity.FIND_READY, AuditOutboxEntity.class)
            .setParameter("now", now)
            .setLockMode(LockModeType.PESSIMISTIC_WRITE)
            .setHint("jakarta.persistence.lock.timeout", 0)
            .setMaxResults(1)
            .getResultList();
    if (readyEvents.isEmpty()) {
      return Optional.empty();
    }

    AuditOutboxEntity event = readyEvents.get(0);
    RabbitMqOutboxMessage claimed = event.claim(now + leaseMillis, UUID.randomUUID().toString());
    getEntityManager(session).flush();
    return Optional.of(claimed);
  }

  @Override
  public void markDelivered(KeycloakSessionFactory sessionFactory, String id, String claimToken) {
    KeycloakModelUtils.runJobInTransaction(
        sessionFactory,
        session -> {
          int deleted =
              getEntityManager(session)
                  .createQuery(
                      "DELETE FROM AuditOutboxEntity event "
                          + "WHERE event.id = :id AND event.claimToken = :claimToken")
                  .setParameter("id", id)
                  .setParameter("claimToken", claimToken)
                  .executeUpdate();
          if (deleted == 0) {
            log.warnv(
                "Confirmed audit event {0} was not deleted because its claim expired; it may be delivered again",
                id);
          }
        });
  }

  @Override
  public void markFailed(
      KeycloakSessionFactory sessionFactory,
      String id,
      String claimToken,
      long availableAt,
      String error) {
    KeycloakModelUtils.runJobInTransaction(
        sessionFactory,
        session -> {
          int updated =
              getEntityManager(session)
                  .createQuery(
                      "UPDATE AuditOutboxEntity event "
                          + "SET event.availableAt = :availableAt, "
                          + "event.claimedUntil = NULL, event.claimToken = NULL, "
                          + "event.lastError = :lastError "
                          + "WHERE event.id = :id AND event.claimToken = :claimToken")
                  .setParameter("availableAt", availableAt)
                  .setParameter("lastError", truncate(error))
                  .setParameter("id", id)
                  .setParameter("claimToken", claimToken)
                  .executeUpdate();
          if (updated == 0) {
            log.warnv("Could not reschedule audit event {0} because its claim expired", id);
          }
        });
  }

  private static EntityManager getEntityManager(KeycloakSession session) {
    JpaConnectionProvider provider = session.getProvider(JpaConnectionProvider.class);
    if (provider == null) {
      throw new AuditEventPersistenceException("Keycloak JPA connection provider is unavailable");
    }
    return provider.getEntityManager();
  }

  private static String truncate(String error) {
    if (error == null) {
      return "Unknown RabbitMQ delivery error";
    }
    return error.length() <= MAX_ERROR_LENGTH ? error : error.substring(0, MAX_ERROR_LENGTH);
  }
}
