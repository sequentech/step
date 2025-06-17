// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.gateway;

import com.google.auto.service.AutoService;
import org.keycloak.Config;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;

@AutoService(SmsSenderProviderFactory.class)
public class AwsSmsSenderProviderFactory implements SmsSenderProviderFactory {
  private String senderId;
  private String roleArn;
  private String sessionName;

  @Override
  public SmsSenderProvider create(KeycloakSession session) {
    return new AwsSmsSenderProvider(senderId, roleArn, sessionName);
  }

  @Override
  public void init(Config.Scope config) {
    senderId = config.get("senderId");
    roleArn = config.get("roleArn");
    sessionName = config.get("sessionName", "AwsSmsSenderSession");
  }

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public void close() {}

  @Override
  public String getId() {
    return "aws";
  }
}
