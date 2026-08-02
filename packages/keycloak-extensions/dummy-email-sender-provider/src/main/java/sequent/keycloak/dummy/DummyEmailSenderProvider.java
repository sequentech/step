// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.dummy;

import jakarta.mail.internet.InternetAddress;
import java.util.Map;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.email.EmailException;
import org.keycloak.email.EmailSenderProvider;

@JBossLog
public class DummyEmailSenderProvider implements EmailSenderProvider {

  // Temporary variable for testing purposes
  private String temporaryValue;

  public DummyEmailSenderProvider() {
    // Initialize the temporary value to a default or null
    this.temporaryValue = null;
  }

  @Override
  public void send(
      Map<String, String> config, String address, String subject, String textBody, String htmlBody)
      throws EmailException {
    log.infov(
        "**Sending dummy email**:\n\t- subject={0}\n\t- address={1}\n\t- textBody={2}\n\t- htmlBody={3}",
        subject, address, textBody, htmlBody);
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
  public void validate(Map<String, String> config) throws EmailException {
    String from = config.get("from");
    String fromDisplayName = config.get("fromDisplayName");

    try {
      // Validate that the 'from' address is present and properly formatted
      if (from == null || from.trim().isEmpty()) {
        throw new EmailException("Missing 'from' email address in configuration", null);
      }

      // Validate that the from address is properly formatted
      toInternetAddress(from, fromDisplayName);

      log.infov("Dummy email configuration validated successfully - from: {0}", from);
    } catch (EmailException e) {
      throw e;
    } catch (Exception e) {
      throw new EmailException("Invalid email configuration: " + e.getMessage(), e);
    }
  }

  @Override
  public void close() {}
}
