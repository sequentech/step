// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.custom_event_listener;

import java.sql.SQLException;

/** Sends task messages to a PGMQ queue. */
interface QueueSender extends AutoCloseable {
  void send(String queue, String payload) throws SQLException;

  /** Fails if the task-queue database cannot be used by this environment. */
  default void verify() throws SQLException {}

  @Override
  default void close() {}
}
