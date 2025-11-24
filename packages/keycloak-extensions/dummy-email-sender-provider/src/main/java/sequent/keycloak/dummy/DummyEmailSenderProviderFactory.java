// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.dummy;

import com.google.auto.service.AutoService;
import org.keycloak.Config;
import org.keycloak.email.EmailSenderProvider;
import org.keycloak.email.EmailSenderProviderFactory;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;

/**
 * @author <a href="mailto:edu@sequentech.io">Eduardo Robles</a>
 */
@AutoService(EmailSenderProviderFactory.class)
public class DummyEmailSenderProviderFactory implements EmailSenderProviderFactory {

  @Override
  public EmailSenderProvider create(KeycloakSession session) {
    return new DummyEmailSenderProvider();
  }

  @Override
  public void init(Config.Scope config) {}

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public void close() {}

  @Override
  public String getId() {
    return "dummy";
  }
}
