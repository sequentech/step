// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.aws_ses;

import sequent.keycloak.aws_ses.AwsSesEmailSenderProvider;
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
        return new AwsSesEmailSenderProvider();
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
