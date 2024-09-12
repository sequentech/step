// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import java.util.Optional;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.RealmModel;

public class Utils {
  static Optional<AuthenticatorConfigModel> getConfig(RealmModel realm, String providerId) {
    // Using streams to find the first matching configuration
    // TODO: We're assuming there's only one instance in this realm of this
    // authenticator
    return realm
        .getAuthenticationFlowsStream()
        .flatMap(flow -> realm.getAuthenticationExecutionsStream(flow.getId()))
        .filter(
            model -> {
              return model.getAuthenticator() != null
                  && model.getAuthenticator().equals(providerId);
            })
        .map(model -> realm.getAuthenticatorConfigById(model.getAuthenticatorConfig()))
        .findFirst();
  }
}
