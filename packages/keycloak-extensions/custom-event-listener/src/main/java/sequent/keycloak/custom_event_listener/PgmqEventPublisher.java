// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import com.fasterxml.jackson.databind.ObjectMapper;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.Arrays;
import java.util.Base64;
import java.util.HexFormat;
import java.util.Map;
import java.util.Optional;
import org.keycloak.connections.jpa.JpaConnectionProvider;
import org.keycloak.models.KeycloakSession;

final class PgmqEventPublisher {
  private final String queueName;
  private final ObjectMapper mapper = new ObjectMapper();

  static PgmqEventPublisher fromEnvironment() {
    String slug = System.getenv("ENV_SLUG");
    if (slug == null || slug.isBlank()) {
      throw new IllegalStateException("ENV_SLUG is required for PGMQ electoral logging");
    }
    String queue =
        Optional.ofNullable(System.getenv("ELECTORAL_LOG_QUEUE"))
            .orElse("electoral_log_event_queue")
            .trim();
    return new PgmqEventPublisher(slug + "_" + queue);
  }

  PgmqEventPublisher(String logicalQueue) {
    try {
      byte[] hash =
          MessageDigest.getInstance("SHA-256")
              .digest(logicalQueue.getBytes(StandardCharsets.UTF_8));
      queueName = "step_" + HexFormat.of().formatHex(hash).substring(0, 40);
    } catch (NoSuchAlgorithmException exception) {
      throw new IllegalStateException("SHA-256 is unavailable", exception);
    }
  }

  void initialize(KeycloakSession session) {
    session
        .getProvider(JpaConnectionProvider.class)
        .getEntityManager()
        .createNativeQuery("SELECT 1 FROM pgmq.create(:queue)")
        .setParameter("queue", queueName)
        .getSingleResult();
  }

  void publish(KeycloakSession session, String taskId, String taskName, byte[] body) {
    try {
      Map<String, Object> headers =
          Map.of("id", taskId, "task", taskName, "timelimit", Arrays.asList(null, null));
      Map<String, Object> properties =
          Map.of("correlation_id", taskId, "delivery_tag", taskId, "body_encoding", "base64");
      String payload =
          mapper.writeValueAsString(
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
      // Commit and rollback follow the Keycloak request transaction, including audit failures.
      session
          .getProvider(JpaConnectionProvider.class)
          .getEntityManager()
          .createNativeQuery("SELECT pgmq.send(:queue, CAST(:payload AS jsonb))")
          .setParameter("queue", queueName)
          .setParameter("payload", payload)
          .getSingleResult();
    } catch (Exception exception) {
      session.getTransactionManager().setRollbackOnly();
      throw new IllegalStateException("Unable to enqueue electoral audit event", exception);
    }
  }
}
