// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.credential;

import java.io.IOException;
import java.util.Optional;
import org.jboss.logging.Logger;
import org.keycloak.common.util.ObjectUtil;
import org.keycloak.common.util.Time;
import org.keycloak.credential.*;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserCredentialModel;
import org.keycloak.models.UserModel;
import org.keycloak.util.JsonSerialization;
import sequent.keycloak.authenticator.MessageOTPAuthenticatorFactory;

/**
 * 证书使用 CredentialValidator 来认证，例如 password 证书 使用登录认证，本例中使用 phone OTP 认证 //not have credential ,
 * SmsOtpMfaAuthenticator setRequiredActions will add ConfigSmsOtpRequiredAction to add an OPT
 * credential -> OTP
 */
public class MessageOTPCredentialProvider
    implements CredentialProvider<MessageOTPCredentialModel>, CredentialInputValidator {

  private static final Logger logger = Logger.getLogger(MessageOTPCredentialProvider.class);
  private final KeycloakSession session;

  public MessageOTPCredentialProvider(KeycloakSession session) {
    this.session = session;
  }

  @Override
  public boolean supportsCredentialType(String credentialType) {
    return getType().equals(credentialType);
  }

  @Override
  public boolean isConfiguredFor(RealmModel realm, UserModel user, String credentialType) {
    logger.info("isConfiguredFor(): " + credentialType);
    if (!supportsCredentialType(credentialType)) {
      logger.info("isConfiguredFor(): !supportsCredentialType");
      return false;
    }
    boolean ret =
        user.credentialManager()
            .getStoredCredentialsByTypeStream(credentialType)
            .findAny()
            .isPresent();
    logger.info("isConfiguredFor(): ret=" + ret);
    return ret;
  }

  @Override
  public boolean isValid(RealmModel realm, UserModel user, CredentialInput input) {
    logger.info("isValid");

    if (!(input instanceof UserCredentialModel)) {
      return false;
    }
    if (!input.getType().equals(getType())) {
      return false;
    }

    if (ObjectUtil.isBlank(input.getCredentialId())) {
      logger.debugf(
          "CredentialId is null when validating credential of user %s", user.getUsername());
      return false;
    }

    var invalid =
        Optional.ofNullable(
                user.credentialManager().getStoredCredentialById(input.getCredentialId()))
            .map(
                credentialModel -> {
                  try {
                    return JsonSerialization.readValue(
                        credentialModel.getCredentialData(),
                        MessageOTPCredentialModel.MessageOTPCredentialData.class);
                  } catch (IOException error) {
                    throw new IllegalArgumentException(error);
                  }
                })
            .map(MessageOTPCredentialModel.MessageOTPCredentialData::isSecretInvalid)
            .filter(invalidSecret -> invalidSecret)
            .orElse(false);

    return !invalid;
  }

  @Override
  public String getType() {
    return MessageOTPCredentialModel.TYPE;
  }

  @Override
  public CredentialModel createCredential(
      RealmModel realm, UserModel user, MessageOTPCredentialModel credential) {
    logger.info("createCredential()");
    if (credential.getCreatedDate() == null) {
      credential.setCreatedDate(Time.currentTimeMillis());
    }
    return user.credentialManager().createStoredCredential(credential);
  }

  @Override
  public boolean deleteCredential(RealmModel realm, UserModel user, String credentialId) {
    logger.info("deleteCredential()");
    return user.credentialManager().removeStoredCredentialById(credentialId);
  }

  @Override
  public MessageOTPCredentialModel getCredentialFromModel(CredentialModel credentialModel) {
    return MessageOTPCredentialModel.createFromCredentialModel(credentialModel);
  }

  @Override
  public CredentialTypeMetadata getCredentialTypeMetadata(
      CredentialTypeMetadataContext credentialTypeMetadataContext) {
    return CredentialTypeMetadata.builder()
        .type(getType())
        .helpText("SMS/Email OTP Credential Type")
        .category(CredentialTypeMetadata.Category.TWO_FACTOR)
        .displayName(MessageOTPCredentialProviderFactory.PROVIDER_ID)
        .createAction(MessageOTPAuthenticatorFactory.PROVIDER_ID)
        .removeable(true)
        .build(session);
  }
}
