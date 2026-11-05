// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.pin_link;

import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import java.util.UUID;

/**
 * Plain SQL over Keycloak's user tables. The state that decides who wins must be the database row,
 * never a cached user model, because replicas don't share one in time and a restart loses it.
 */
final class PinLinkStore {
  private final String userAttributeTable;
  private final String userEntityTable;

  PinLinkStore(String userAttributeTable, String userEntityTable) {
    this.userAttributeTable = userAttributeTable;
    this.userEntityTable = userEntityTable;
  }

  /** Finds the voter holding a token hash; two voters with one hash count as nobody. */
  Optional<String> findVoterId(Connection connection, String realmId, String name, String hash)
      throws SQLException {
    String sql =
        "SELECT ua.user_id FROM "
            + userAttributeTable
            + " ua JOIN "
            + userEntityTable
            + " u ON u.id = ua.user_id WHERE u.realm_id = ? AND ua.name = ? AND ua.value = ?";
    try (PreparedStatement statement = connection.prepareStatement(sql)) {
      statement.setString(1, realmId);
      statement.setString(2, name);
      statement.setString(3, hash);
      statement.setMaxRows(2);
      try (ResultSet rows = statement.executeQuery()) {
        if (!rows.next()) {
          return Optional.empty();
        }
        String voterId = rows.getString(1);
        return rows.next() ? Optional.empty() : Optional.of(voterId);
      }
    }
  }

  /** Locks the hash row for this transaction; false if another request already deleted it. */
  boolean lockHash(Connection connection, String voterId, String name, String hash)
      throws SQLException {
    String sql =
        "SELECT id FROM "
            + userAttributeTable
            + " WHERE user_id = ? AND name = ? AND value = ? FOR UPDATE";
    try (PreparedStatement statement = connection.prepareStatement(sql)) {
      statement.setString(1, voterId);
      statement.setString(2, name);
      statement.setString(3, hash);
      try (ResultSet rows = statement.executeQuery()) {
        return rows.next();
      }
    }
  }

  List<String> values(Connection connection, String voterId, String name) throws SQLException {
    String sql = "SELECT value FROM " + userAttributeTable + " WHERE user_id = ? AND name = ?";
    try (PreparedStatement statement = connection.prepareStatement(sql)) {
      statement.setString(1, voterId);
      statement.setString(2, name);
      try (ResultSet rows = statement.executeQuery()) {
        List<String> values = new ArrayList<>();
        while (rows.next()) {
          values.add(rows.getString(1));
        }
        return values;
      }
    }
  }

  /** Call with the hash row locked: the lock is what serializes concurrent attempts. */
  void setAttempts(Connection connection, String voterId, String name, int attempts)
      throws SQLException {
    String update =
        "UPDATE " + userAttributeTable + " SET value = ? WHERE user_id = ? AND name = ?";
    try (PreparedStatement statement = connection.prepareStatement(update)) {
      statement.setString(1, String.valueOf(attempts));
      statement.setString(2, voterId);
      statement.setString(3, name);
      if (statement.executeUpdate() > 0) {
        return;
      }
    }
    String insert =
        "INSERT INTO " + userAttributeTable + " (id, user_id, name, value) VALUES (?, ?, ?, ?)";
    try (PreparedStatement statement = connection.prepareStatement(insert)) {
      statement.setString(1, UUID.randomUUID().toString());
      statement.setString(2, voterId);
      statement.setString(3, name);
      statement.setString(4, String.valueOf(attempts));
      statement.executeUpdate();
    }
  }

  /** The consume itself: one statement, and only the request that deleted the row wins. */
  boolean deleteHash(Connection connection, String voterId, String name, String hash)
      throws SQLException {
    String sql =
        "DELETE FROM " + userAttributeTable + " WHERE user_id = ? AND name = ? AND value = ?";
    try (PreparedStatement statement = connection.prepareStatement(sql)) {
      statement.setString(1, voterId);
      statement.setString(2, name);
      statement.setString(3, hash);
      return statement.executeUpdate() == 1;
    }
  }

  void deleteAttribute(Connection connection, String voterId, String name) throws SQLException {
    String sql = "DELETE FROM " + userAttributeTable + " WHERE user_id = ? AND name = ?";
    try (PreparedStatement statement = connection.prepareStatement(sql)) {
      statement.setString(1, voterId);
      statement.setString(2, name);
      statement.executeUpdate();
    }
  }
}
