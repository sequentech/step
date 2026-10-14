// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import java.util.ArrayList;
import java.util.List;
import java.util.logging.Handler;
import java.util.logging.Level;
import java.util.logging.LogRecord;
import java.util.logging.Logger;
import java.util.logging.SimpleFormatter;

public final class CapturedLogs implements AutoCloseable {
  private final Logger logger;
  private final Level previousLevel;
  private final boolean previousUseParentHandlers;
  private final List<String> messages = new ArrayList<>();
  private final Handler handler =
      new Handler() {
        private final SimpleFormatter formatter = new SimpleFormatter();

        @Override
        public void publish(LogRecord record) {
          messages.add(formatter.format(record));
        }

        @Override
        public void flush() {}

        @Override
        public void close() {}
      };

  public CapturedLogs(Class<?> type) {
    logger = Logger.getLogger(type.getName());
    previousLevel = logger.getLevel();
    previousUseParentHandlers = logger.getUseParentHandlers();
    logger.setLevel(Level.ALL);
    logger.setUseParentHandlers(false);
    handler.setLevel(Level.ALL);
    logger.addHandler(handler);
  }

  public String text() {
    return String.join("\n", messages);
  }

  @Override
  public void close() {
    logger.removeHandler(handler);
    logger.setLevel(previousLevel);
    logger.setUseParentHandlers(previousUseParentHandlers);
  }
}
