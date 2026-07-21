// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import java.util.Objects;

record RabbitMqOutboxMessage(
    String id, String queueName, String taskName, byte[] body, int attempt, String claimToken) {

  RabbitMqOutboxMessage {
    Objects.requireNonNull(id, "id");
    Objects.requireNonNull(queueName, "queueName");
    Objects.requireNonNull(taskName, "taskName");
    Objects.requireNonNull(body, "body");
    body = body.clone();
  }

  @Override
  public byte[] body() {
    return body.clone();
  }
}
