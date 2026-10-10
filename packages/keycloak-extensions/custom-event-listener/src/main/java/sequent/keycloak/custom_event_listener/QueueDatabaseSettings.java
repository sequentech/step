// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.custom_event_listener;

import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.util.LinkedHashMap;
import java.util.Locale;
import java.util.Map;
import java.util.Optional;
import java.util.stream.Collectors;

/**
 * The connection to the environment's task-queue database, configured with the same {@code
 * QUEUE_DB__*} variables as Windmill and Harvest.
 */
record QueueDatabaseSettings(
    String host,
    int port,
    String database,
    String user,
    String password,
    Optional<String> sslMode,
    Optional<String> caPath,
    int poolSize,
    String environment) {

  static final int DEFAULT_PORT = 5432;
  static final int DEFAULT_POOL_SIZE = 10;
  private static final String APPLICATION_NAME = "keycloak-electoral-log";
  private static final String STATEMENT_TIMEOUT = "5s";
  // Longer than the statement timeout, so the server normally cancels first; it bounds a send
  // to a database that stopped answering, which would otherwise hold the login.
  private static final String SOCKET_TIMEOUT_SECONDS = "10";
  private static final String LOGIN_TIMEOUT_SECONDS = "5";

  static QueueDatabaseSettings fromEnvironment(Map<String, String> environment) {
    return new QueueDatabaseSettings(
        required(environment, "QUEUE_DB__HOST"),
        positive(environment, "QUEUE_DB__PORT", DEFAULT_PORT),
        required(environment, "QUEUE_DB__DBNAME"),
        required(environment, "QUEUE_DB__USER"),
        required(environment, "QUEUE_DB__PASSWORD"),
        optional(environment, "QUEUE_DB__SSL_MODE").map(QueueDatabaseSettings::sslMode),
        optional(environment, "QUEUE_DB_CA_PATH"),
        positive(environment, "QUEUE_DB__POOL__MAX_SIZE", DEFAULT_POOL_SIZE),
        required(environment, "ENV_SLUG"));
  }

  String jdbcUrl() {
    Map<String, String> parameters = new LinkedHashMap<>();
    parameters.put("ApplicationName", APPLICATION_NAME);
    parameters.put("options", "-c statement_timeout=" + STATEMENT_TIMEOUT);
    parameters.put("socketTimeout", SOCKET_TIMEOUT_SECONDS);
    // In the URL rather than the pool, which would set DriverManager's JVM-wide login timeout.
    parameters.put("loginTimeout", LOGIN_TIMEOUT_SECONDS);
    // pgjdbc verifies the server only in verify-* modes; with a CA, Require verifies the
    // certificate and host name, as Windmill does.
    sslMode.ifPresent(
        mode ->
            parameters.put(
                "sslmode", mode.equals("require") && caPath.isPresent() ? "verify-full" : mode));
    caPath.ifPresent(path -> parameters.put("sslrootcert", path));
    return "jdbc:postgresql://"
        + host
        + ":"
        + port
        + "/"
        + URLEncoder.encode(database, StandardCharsets.UTF_8)
        + "?"
        + parameters.entrySet().stream()
            .map(
                parameter ->
                    parameter.getKey()
                        + "="
                        + URLEncoder.encode(parameter.getValue(), StandardCharsets.UTF_8))
            .collect(Collectors.joining("&"));
  }

  private static String sslMode(String configured) {
    String mode = configured.trim().toLowerCase(Locale.ROOT);
    return switch (mode) {
      case "disable", "prefer", "require" -> mode;
      default ->
          throw new IllegalStateException(
              "QUEUE_DB__SSL_MODE must be Disable, Prefer or Require, got " + configured);
    };
  }

  private static Optional<String> optional(Map<String, String> environment, String name) {
    return Optional.ofNullable(environment.get(name)).map(String::trim).filter(v -> !v.isEmpty());
  }

  private static String required(Map<String, String> environment, String name) {
    return optional(environment, name)
        .orElseThrow(
            () -> new IllegalStateException(name + " is required for electoral-log events"));
  }

  private static int positive(Map<String, String> environment, String name, int fallback) {
    return optional(environment, name)
        .map(
            value -> {
              try {
                int parsed = Integer.parseInt(value);
                if (parsed > 0) {
                  return parsed;
                }
              } catch (NumberFormatException ignored) {
                // Reported below with the variable's name.
              }
              throw new IllegalStateException(name + " must be a positive integer, got " + value);
            })
        .orElse(fallback);
  }

  @Override
  public String toString() {
    return "QueueDatabaseSettings[" + host + ":" + port + "/" + database + " as " + user + "]";
  }
}
