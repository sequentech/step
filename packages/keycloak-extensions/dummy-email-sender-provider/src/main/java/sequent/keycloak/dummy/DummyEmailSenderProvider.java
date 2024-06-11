// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.dummy;

import org.keycloak.email.EmailSenderProvider;
import org.keycloak.email.EmailException;
import lombok.extern.jbosslog.JBossLog;
import java.util.Map;
 
/*
 * Dummy email sender provider that just prints emails into the standard output
 * log.
 
@JBossLog
public class DummyEmailSenderProvider implements EmailSenderProvider {
 
    public DummyEmailSenderProvider() {

    }

    @Override
    public void send(
        Map<String, String> config,
        String address,
        String subject,
        String textBody,
        String htmlBody
    ) throws EmailException
    {
        log.infov(
            "**Sending dummy email**:\n\t- subject={0}\n\t- address={1}\n\t- textBody={2}\n\t- htmlBody={3}",
            subject,
            address,
            textBody,
            htmlBody
        );
    }

    @Override
    public void close() {
    }
}
*/

@JBossLog
public class DummyEmailSenderProvider implements EmailSenderProvider {

    // Temporary variable for testing purposes
    private String temporaryValue;

    public DummyEmailSenderProvider() {
        // Initialize the temporary value to a default or null
        this.temporaryValue = null;
    }

    // Setter method to set the temporary value
    public void setTemporaryValue(String temporaryValue) {
        this.temporaryValue = temporaryValue;
    }

    @Override
    public void send(
            Map<String, String> config,
            String address,
            String subject,
            String textBody,
            String htmlBody
    ) throws EmailException {
        // Use the temporary value in your method logic
        log.infov(
                "**Sending dummy email**:\n\t- subject={0}\n\t- address={1}\n\t- textBody={2}\n\t- htmlBody={3}\n\t- temporaryValue={4}",
                subject,
                address,
                textBody,
                htmlBody,
                temporaryValue
        );
    }

    @Override
    public void close() {
    }
}
