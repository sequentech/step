// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.dummy;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.logging.Handler;
import java.util.logging.Level;
import java.util.logging.LogRecord;
import java.util.logging.Logger;
import java.util.logging.SimpleFormatter;
import org.junit.jupiter.api.Test;

class EmailSecretLoggingTest {
  @Test
  void sendsWithoutLoggingSubjectOrMessageBodies() throws Exception {
    DummyEmailSenderProvider provider = new DummyEmailSenderProvider();
    Logger logger = Logger.getLogger(DummyEmailSenderProvider.class.getName());
    Level oldLevel = logger.getLevel();
    boolean oldParents = logger.getUseParentHandlers();
    List<String> logs = new ArrayList<>();
    Handler handler =
        new Handler() {
          @Override
          public void publish(LogRecord record) {
            logs.add(new SimpleFormatter().format(record));
          }

          @Override
          public void flush() {}

          @Override
          public void close() {}
        };
    logger.setLevel(Level.ALL);
    logger.setUseParentHandlers(false);
    logger.addHandler(handler);
    try {
      provider.send(
          Map.of("from", "sender@example.com"),
          "voter@example.com",
          "Secret subject 190283",
          "OTP 482619",
          "<a href=\"https://auth.example/?key=synthetic-token\">Login</a>");

      assertFalse(logs.isEmpty());
      String text = String.join("\n", logs);
      assertFalse(text.contains("482619"));
      assertFalse(text.contains("190283"));
      assertFalse(text.contains("synthetic-token"));
    } finally {
      logger.removeHandler(handler);
      logger.setLevel(oldLevel);
      logger.setUseParentHandlers(oldParents);
    }
  }
}
