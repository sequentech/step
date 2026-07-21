// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import java.util.Optional;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;

interface AuditOutboxStore {

  void enqueue(KeycloakSession session, RabbitMqOutboxMessage message);

  Optional<RabbitMqOutboxMessage> claimNext(
      KeycloakSessionFactory sessionFactory, long now, long leaseMillis);

  void markDelivered(KeycloakSessionFactory sessionFactory, String id, String claimToken);

  void markFailed(
      KeycloakSessionFactory sessionFactory,
      String id,
      String claimToken,
      long availableAt,
      String error);
}
