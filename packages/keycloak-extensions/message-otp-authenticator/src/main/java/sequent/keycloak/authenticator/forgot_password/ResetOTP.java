// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import com.google.auto.service.AutoService;
import java.util.List;
import java.util.stream.Collectors;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.authentication.CredentialValidator;
import org.keycloak.authentication.authenticators.resetcred.*;
import org.keycloak.credential.CredentialModel;
import org.keycloak.credential.CredentialProvider;
import org.keycloak.credential.OTPCredentialProvider;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.credential.OTPCredentialModel;
import org.keycloak.provider.ProviderConfigProperty;

/**
 * This is similar to the stock Reset OTP, but with some changes: - configuredFor() returns always
 * true. This allows us to put it as required in a flow without cancelling the flow if the user
 * doesn't have an OTP. - It is not configurable, it just deletes all the otp credentials - It
 * doesn't add a ResetOTP RequiredAction. In our case that is performed by the MFA MethodSelector.
 */
@JBossLog
@AutoService(AuthenticatorFactory.class)
public class ResetOTP extends AbstractSetRequiredActionAuthenticator
    implements CredentialValidator<OTPCredentialProvider> {
  public static final String PROVIDER_ID = "reset-otp-sequent";

  @Override
  public void authenticate(AuthenticationFlowContext context) {
    log.info("authenticate()");
    List<CredentialModel> otpCredentialModelList =
        context
            .getUser()
            .credentialManager()
            .getStoredCredentialsByTypeStream(OTPCredentialModel.TYPE)
            .collect(Collectors.toList());

    log.info("authenticate(): REMOVE_ALL");

    otpCredentialModelList.forEach(
        otpCredentialModel -> {
          log.info("authenticate(): REMOVE_ALL: removing otpCredentialModel.getId()");

          context
              .getUser()
              .credentialManager()
              .removeStoredCredentialById(otpCredentialModel.getId());
        });

    context.success();
  }

  @Override
  public void action(AuthenticationFlowContext context) {}

  @Override
  public boolean isConfigurable() {
    return false;
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    return List.of();
  }

  @Override
  public OTPCredentialProvider getCredentialProvider(KeycloakSession session) {
    return (OTPCredentialProvider) session.getProvider(CredentialProvider.class, "keycloak-otp");
  }

  @Override
  public boolean configuredFor(KeycloakSession session, RealmModel realm, UserModel user) {
    return true;
  }

  @Override
  public String getDisplayType() {
    return "Reset OTP - Sequent";
  }

  @Override
  public String getHelpText() {
    return "Removes all existing OTP configurations.";
  }

  @Override
  public String getId() {
    return PROVIDER_ID;
  }
}
