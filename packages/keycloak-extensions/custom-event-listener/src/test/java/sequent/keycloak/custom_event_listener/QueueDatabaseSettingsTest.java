// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.custom_event_listener;

import static org.junit.jupiter.api.Assertions.*;

import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.Test;

class QueueDatabaseSettingsTest {
  private static Map<String, String> environment() {
    Map<String, String> environment = new HashMap<>();
    environment.put("QUEUE_DB__HOST", "postgres");
    environment.put("QUEUE_DB__DBNAME", "dev_queues");
    environment.put("QUEUE_DB__USER", "queue_producer");
    environment.put("QUEUE_DB__PASSWORD", "secret-password");
    environment.put("ENV_SLUG", "dev");
    return environment;
  }

  @Test
  void defaultsApplyToOptionalSettings() {
    QueueDatabaseSettings settings = QueueDatabaseSettings.fromEnvironment(environment());
    assertEquals(QueueDatabaseSettings.DEFAULT_PORT, settings.port());
    assertEquals(QueueDatabaseSettings.DEFAULT_POOL_SIZE, settings.poolSize());
    assertEquals("dev", settings.environment());
    assertEquals(
        "jdbc:postgresql://postgres:5432/dev_queues?ApplicationName=keycloak-electoral-log"
            + "&options=-c+statement_timeout%3D5s",
        settings.jdbcUrl());
    assertFalse(settings.toString().contains("secret-password"));
  }

  @Test
  void tlsSettingsReachTheJdbcUrl() {
    Map<String, String> environment = environment();
    environment.put("QUEUE_DB__PORT", "6432");
    environment.put("QUEUE_DB__SSL_MODE", "Require");
    environment.put("QUEUE_DB_CA_PATH", "/etc/ssl/rds.pem");
    environment.put("QUEUE_DB__POOL__MAX_SIZE", "4");
    QueueDatabaseSettings settings = QueueDatabaseSettings.fromEnvironment(environment);
    assertEquals(4, settings.poolSize());
    assertTrue(settings.jdbcUrl().startsWith("jdbc:postgresql://postgres:6432/dev_queues?"));
    assertTrue(settings.jdbcUrl().endsWith("&sslmode=require&sslrootcert=%2Fetc%2Fssl%2Frds.pem"));
  }

  @Test
  void missingOrInvalidSettingsAreRejected() {
    for (String required :
        new String[] {
          "QUEUE_DB__HOST", "QUEUE_DB__DBNAME", "QUEUE_DB__USER", "QUEUE_DB__PASSWORD", "ENV_SLUG"
        }) {
      Map<String, String> environment = environment();
      environment.remove(required);
      IllegalStateException error =
          assertThrows(
              IllegalStateException.class,
              () -> QueueDatabaseSettings.fromEnvironment(environment));
      assertTrue(error.getMessage().contains(required));
    }
    for (Map.Entry<String, String> invalid :
        Map.of(
                "QUEUE_DB__PORT", "0",
                "QUEUE_DB__POOL__MAX_SIZE", "many",
                "QUEUE_DB__SSL_MODE", "verify-full")
            .entrySet()) {
      Map<String, String> environment = environment();
      environment.put(invalid.getKey(), invalid.getValue());
      assertThrows(
          IllegalStateException.class, () -> QueueDatabaseSettings.fromEnvironment(environment));
    }
  }
}
