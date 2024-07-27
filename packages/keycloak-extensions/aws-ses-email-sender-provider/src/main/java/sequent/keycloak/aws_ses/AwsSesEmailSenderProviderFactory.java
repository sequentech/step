// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.aws_ses;

// sending email using AWS
import com.google.auto.service.AutoService;
import org.keycloak.Config;
import org.keycloak.email.EmailSenderProvider;
import org.keycloak.email.EmailSenderProviderFactory;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import software.amazon.awssdk.services.ses.SesClient; // sending Mock email

/**
 * @author <a href="mailto:edu@sequentech.io">Eduardo Robles</a>
 */
@AutoService(EmailSenderProviderFactory.class)
public class AwsSesEmailSenderProviderFactory implements EmailSenderProviderFactory {

  @Override
  public EmailSenderProvider create(KeycloakSession session) {
    SesClient sesClient = SesClient.builder().build();
    return new AwsSesEmailSenderProvider(sesClient);
  }

  @Override
  public void init(Config.Scope config) {}

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public void close() {}

  @Override
  public String getId() {
    return "aws-ses";
  }
}
