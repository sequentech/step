// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.aws_ses;

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
import org.keycloak.email.EmailException;
import org.mockito.ArgumentCaptor;
import software.amazon.awssdk.awscore.exception.AwsErrorDetails;
import software.amazon.awssdk.services.ses.SesClient;
import software.amazon.awssdk.services.ses.model.SendEmailRequest;
import software.amazon.awssdk.services.ses.model.SesException;

class EmailSecretLoggingTest {
  @Test
  void providerFailuresDoNotExposeEchoedBodiesThroughLogsOrExceptions() {
    SesClient client = mock(SesClient.class);
    AwsSesEmailSenderProvider provider = new AwsSesEmailSenderProvider(client);
    Logger logger = Logger.getLogger(AwsSesEmailSenderProvider.class.getName());
    Handler handler = mock(Handler.class);
    logger.addHandler(handler);
    try {
      for (RuntimeException failure :
          List.of(
              SesException.builder()
                  .statusCode(400)
                  .awsErrorDetails(
                      AwsErrorDetails.builder().errorMessage("OTP synthetic-body-secret").build())
                  .build(),
              new IllegalStateException("synthetic-body-secret"))) {
        doThrow(failure).when(client).sendEmail(any(SendEmailRequest.class));
        EmailException error =
            assertThrows(
                EmailException.class,
                () ->
                    provider.send(
                        Map.of("from", "sender@example.com"),
                        "voter@example.com",
                        "subject",
                        "body",
                        "<p>body</p>"));
        assertFalse(error.getMessage().contains("synthetic-body-secret"));
        assertNull(error.getCause());
      }
      ArgumentCaptor<LogRecord> records = ArgumentCaptor.forClass(LogRecord.class);
      verify(handler, atLeastOnce()).publish(records.capture());
      for (LogRecord record : records.getAllValues()) {
        assertFalse(new SimpleFormatter().format(record).contains("synthetic-body-secret"));
      }
    } finally {
      logger.removeHandler(handler);
    }
  }

  @Test
  void sendsWithoutLoggingSubjectOrMessageBodies() throws Exception {
    SesClient client = mock(SesClient.class);
    AwsSesEmailSenderProvider provider = new AwsSesEmailSenderProvider(client);
    Logger logger = Logger.getLogger(AwsSesEmailSenderProvider.class.getName());
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
      ArgumentCaptor<SendEmailRequest> request = ArgumentCaptor.forClass(SendEmailRequest.class);
      verify(client).sendEmail(request.capture());
      assertEquals("OTP 482619", request.getValue().message().body().text().data());
      assertEquals(
          "<a href=\"https://auth.example/?key=synthetic-token\">Login</a>",
          request.getValue().message().body().html().data());
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
