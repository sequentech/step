// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import com.google.auto.service.AutoService;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.authentication.authenticators.x509.X509ClientCertificateAuthenticatorFactory;
import org.keycloak.models.KeycloakSession;

/** Factory for {@link X509UserResolutionAuthenticator}. */
@AutoService(AuthenticatorFactory.class)
public class X509UserResolutionAuthenticatorFactory
    extends X509ClientCertificateAuthenticatorFactory {

  public static final String PROVIDER_ID = "x509-user-resolution";

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  @Override
  public String getDisplayType() {
    return "X509 Certificate Authentication with User Resolution";
  }

  @Override
  public Authenticator create(KeycloakSession session) {
    return new X509UserResolutionAuthenticator();
  }
}
