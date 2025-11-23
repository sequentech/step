// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.gateway;

import java.io.IOException;
import java.util.List;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;

@JBossLog
public class DummySmsSenderProvider implements SmsSenderProvider {
  DummySmsSenderProvider() {}

  @Override
  public String send(
      String phoneNumber,
      String messageKey,
      List<String> attributes,
      RealmModel realm,
      UserModel user,
      KeycloakSession session)
      throws IOException {
    log.infov("send(): called");
    return SmsSenderProvider.super.send(phoneNumber, messageKey, attributes, realm, user, session);
  }

  @Override
  public void send(String phoneNumber, String message) throws IOException {
    log.infov("send() small: called");
    log.infov("**Sending dummy sms**:\n\t- phoneNumber={0}\n\t- message={1}", phoneNumber, message);
  }

  @Override
  public void close() {}
}
