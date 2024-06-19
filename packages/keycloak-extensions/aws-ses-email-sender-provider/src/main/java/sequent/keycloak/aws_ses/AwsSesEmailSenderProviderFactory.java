// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.aws_ses;

import sequent.keycloak.aws_ses.AwsSesEmailSenderProvider; // sending email using AWS
import software.amazon.awssdk.services.ses.SesClient;      // sending Mock email  

import org.keycloak.email.EmailSenderProviderFactory;
import org.keycloak.email.EmailSenderProvider;
import org.keycloak.Config;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import com.google.auto.service.AutoService;
 
 /**
  * @author <a href="mailto:edu@sequentech.io">Eduardo Robles</a>
  */
@AutoService(EmailSenderProviderFactory.class)
public class AwsSesEmailSenderProviderFactory
    implements EmailSenderProviderFactory
{

    @Override
    public EmailSenderProvider create(KeycloakSession session) {
        /* 
        // Uncomment for emailing directly using AWS
        return new AwsSesEmailSenderProvider();
        */
        // For Mock Ses email sending test, Comment if to send email directly using AWS
        SesClient sesClient = SesClient.builder().build();
        return new AwsSesEmailSenderProvider(sesClient);
    }

    @Override
    public void init(Config.Scope config) {
    }

    @Override
    public void postInit(KeycloakSessionFactory factory) {
    }

    @Override
    public void close() {
    }

    @Override
    public String getId() {
        return "aws-ses";
    }
}
