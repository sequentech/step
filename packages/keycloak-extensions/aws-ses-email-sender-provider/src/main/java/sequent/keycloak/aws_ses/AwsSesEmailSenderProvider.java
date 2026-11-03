// SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
// SPDX-FileCopyrightText: 2020 Niko Köbler (MIT)
//
// SPDX-License-Identifier: AGPL-3.0-only
// Inspired on https://github.com/dasniko/keycloak-aws-ses-email-provider
package sequent.keycloak.aws_ses;

import jakarta.mail.internet.InternetAddress;
import java.util.Collections;
import java.util.Map;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.email.EmailException;
import org.keycloak.email.EmailSenderProvider;
import software.amazon.awssdk.services.ses.SesClient;
import software.amazon.awssdk.services.ses.model.Body;
import software.amazon.awssdk.services.ses.model.Content;
import software.amazon.awssdk.services.ses.model.Destination;
import software.amazon.awssdk.services.ses.model.Message;
import software.amazon.awssdk.services.ses.model.SendEmailRequest;
import software.amazon.awssdk.services.ses.model.SesException;

/*
 * AwsSes email sender provider that uses AWS Simple Email Service to send emails.
 */

@JBossLog
public class AwsSesEmailSenderProvider implements EmailSenderProvider {

  private final SesClient sesClient;

  public AwsSesEmailSenderProvider(SesClient sesClient) {
    this.sesClient = sesClient;
  }

  @Override
  public void send(
      Map<String, String> config, String address, String subject, String textBody, String htmlBody)
      throws EmailException {
    String from = config.get("from");
    String fromDisplayName = config.get("fromDisplayName");
    String replyTo = config.get("replyTo");
    String replyToDisplayName = config.get("replyToDisplayName");

    log.infov("Sending AWS SES email to {0}, from={1}", address, from);
    try {
      if (from == null || from.isEmpty()) {
        throw new Exception("Missing 'from' email address.");
      }
      SendEmailRequest.Builder request =
          SendEmailRequest.builder()
              .source(toInternetAddress(from, fromDisplayName).toString())
              .destination(Destination.builder().toAddresses(address).build())
              .message(
                  Message.builder()
                      .subject(Content.builder().data(subject).charset("UTF-8").build())
                      .body(
                          Body.builder()
                              .text(Content.builder().data(textBody).charset("UTF-8").build())
                              .html(Content.builder().data(htmlBody).charset("UTF-8").build())
                              .build())
                      .build());

      if (replyTo != null && !replyTo.isEmpty()) {
        request.replyToAddresses(
            Collections.singletonList(toInternetAddress(replyTo, replyToDisplayName).toString()));
      }

      sesClient.sendEmail(request.build());
      log.infov("Email sent to {0} via AWS SES", address);
    } catch (SesException error) {
      log.errorf("AWS SES delivery failed (status %d)", error.statusCode());
      throw new EmailException("SES: Failed to send email via AWS SES");
    } catch (Exception error) {
      String category = failureCategory(error);
      log.errorf("Failed to send email via AWS SES (%s)", category);
      throw new EmailException("Exception: Failed to send email via AWS SES (" + category + ")");
    }
  }

  // Class names only: exception messages can echo addresses or provider responses.
  static String failureCategory(Throwable error) {
    String category = error.getClass().getSimpleName();
    Throwable cause = error.getCause();
    return cause == null ? category : category + " caused by " + cause.getClass().getSimpleName();
  }

  public InternetAddress toInternetAddress(String email, String displayName) throws Exception {
    if (email == null || "".equals(email.trim())) {
      throw new EmailException("Invalid email address", null);
    }
    if (displayName == null || "".equals(displayName.trim())) {
      return new InternetAddress(email);
    }
    return new InternetAddress(email, displayName, "UTF-8");
  }

  @Override
  public void close() {
    // Properly close the SES client
    if (sesClient != null) {
      sesClient.close();
    }
  }
}
