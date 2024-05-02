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
 */
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
