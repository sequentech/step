// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import com.google.auto.service.AutoService;
import java.util.List;
import java.util.Optional;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.*;
import org.keycloak.authentication.authenticators.resetcred.AbstractSetRequiredActionAuthenticator;
import org.keycloak.models.UserModel;
import org.keycloak.provider.ProviderConfigProperty;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialModel;

/*
 * Resets Message OTP credentials for the given user
 */
@JBossLog
@AutoService(AuthenticatorFactory.class)
public class ResetMessageOTP extends AbstractSetRequiredActionAuthenticator {
  public static final String PROVIDER_ID = "reset-message-otp";

  @Override
  public void authenticate(AuthenticationFlowContext context) {
    log.info("authenticate()");
    UserModel user = context.getUser();
    if (user == null) {
      log.info("authenticate(): user is null, return");
      return;
    }

    if (context.getExecution().isRequired()
        || (context.getExecution().isConditional() && configuredFor(context))) {
      Optional<String> credentialId =
          user.credentialManager()
              .getStoredCredentialsStream()
              .filter(
                  credential -> {
                    boolean result = credential.getType().equals(MessageOTPCredentialModel.TYPE);
                    log.infov(
                        "authenticate(): credential.getType()={0}, MessageOTPCredentialModel.TYPE={1}, result={2}",
                        credential.getType(), MessageOTPCredentialModel.TYPE, result);
                    return result;
                  })
              .map(credential -> credential.getId())
              .findFirst();

      log.infov("authenticate(): credentialId.isPresent()={0}", credentialId.isPresent());
      if (credentialId.isPresent()) {
        log.infov("authenticate(): removing credentialId={0}", credentialId);
        user.credentialManager().removeStoredCredentialById(credentialId.get());
      }
    }
    context.success();
  }

  protected boolean configuredFor(AuthenticationFlowContext context) {
    return true;
  }

  @Override
  public boolean isConfigurable() {
    return false;
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    return List.of();
  }

  @Override
  public String getDisplayType() {
    return "Reset Message OTP";
  }

  @Override
  public String getHelpText() {
    return "Removes the Message OTP credentials for the user.";
  }

  @Override
  public String getId() {
    return PROVIDER_ID;
  }
}
