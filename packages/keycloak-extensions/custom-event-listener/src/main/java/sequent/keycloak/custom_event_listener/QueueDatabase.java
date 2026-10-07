// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.custom_event_listener;

import io.agroal.api.AgroalDataSource;
import io.agroal.api.configuration.AgroalConnectionPoolConfiguration.ConnectionValidator;
import io.agroal.api.configuration.supplier.AgroalDataSourceConfigurationSupplier;
import io.agroal.api.security.NamePrincipal;
import io.agroal.api.security.SimplePassword;
import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.time.Duration;

/** The environment's task-queue database, with a connection pool of its own. */
final class QueueDatabase implements QueueSender {
  private static final String SEND = "SELECT pgmq.send(?, CAST(? AS jsonb))";
  private static final String INSTALLED_ENVIRONMENT =
      "SELECT environment FROM step_queue.installation";

  private final AgroalDataSource dataSource;
  private final String environment;
  private volatile boolean verified;

  private QueueDatabase(AgroalDataSource dataSource, String environment) {
    this.dataSource = dataSource;
    this.environment = environment;
  }

  static QueueDatabase connect(QueueDatabaseSettings settings) {
    AgroalDataSourceConfigurationSupplier configuration =
        new AgroalDataSourceConfigurationSupplier()
            .connectionPoolConfiguration(
                pool ->
                    pool.maxSize(settings.poolSize())
                        .acquisitionTimeout(Duration.ofSeconds(5))
                        // A connection the server closed while idle is replaced before use,
                        // instead of failing the request that would use it.
                        .validateOnBorrow(true)
                        .validationTimeout(Duration.ofMinutes(1))
                        .connectionValidator(ConnectionValidator.defaultValidatorWithTimeout(5))
                        .reapTimeout(Duration.ofMinutes(5))
                        .maxLifetime(Duration.ofMinutes(30))
                        .connectionFactoryConfiguration(
                            factory ->
                                factory
                                    .connectionProviderClassName("org.postgresql.Driver")
                                    .jdbcUrl(settings.jdbcUrl())
                                    .loginTimeout(Duration.ofSeconds(5))
                                    .autoCommit(true)
                                    .principal(new NamePrincipal(settings.user()))
                                    .credential(new SimplePassword(settings.password()))));
    try {
      return new QueueDatabase(AgroalDataSource.from(configuration), settings.environment());
    } catch (SQLException exception) {
      throw new IllegalStateException(
          "Unable to create the task-queue connection pool for " + settings, exception);
    }
  }

  @Override
  public void send(String queue, String payload) throws SQLException {
    try (Connection connection = dataSource.getConnection()) {
      verify(connection);
      try (PreparedStatement statement = connection.prepareStatement(SEND)) {
        statement.setString(1, queue);
        statement.setString(2, payload);
        statement.executeQuery().close();
      }
    }
  }

  @Override
  public void verify() throws SQLException {
    try (Connection connection = dataSource.getConnection()) {
      verify(connection);
    }
  }

  /** Refuse a database that is not set up or belongs to another environment. */
  private void verify(Connection connection) throws SQLException {
    if (verified) {
      return;
    }
    try (PreparedStatement statement = connection.prepareStatement(INSTALLED_ENVIRONMENT);
        ResultSet result = statement.executeQuery()) {
      String installed = result.next() ? result.getString(1) : null;
      if (installed == null) {
        throw new SQLException("The task-queue database is not set up");
      }
      if (!installed.equals(environment)) {
        throw new SQLException(
            "The task-queue database belongs to environment " + installed + ", not " + environment);
      }
    }
    verified = true;
  }

  @Override
  public void close() {
    dataSource.close();
  }
}
