// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.dummy;

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

  @Override
  public void close() {}
}
