// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import static java.util.Arrays.asList;

import com.google.auto.service.AutoService;
import java.util.List;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.Config.Scope;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticationExecutionModel.Requirement;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.provider.ProviderConfigProperty;
import sequent.keycloak.authenticator.MessageOTPAuthenticator;
import sequent.keycloak.authenticator.Utils.MessageCourier;

@JBossLog
@AutoService(AuthenticatorFactory.class)
public class ManualVerificationConfigAuthenticator implements Authenticator, AuthenticatorFactory {
  public static final String PROVIDER_ID = "manual-verification-config";

  public static final String MESSAGE_COURIER_ATTRIBUTE = "messageCourierAttribute";
  public static final String TEL_USER_ATTRIBUTE = "telUserAttribute";
  public static final String AUTO_2FA = "auto-2fa";

  @Override
  public void close() {
    // No resources to close
  }

  @Override
  public Authenticator create(KeycloakSession session) {
    return new ManualVerificationConfigAuthenticator();
  }

  @Override
  public void init(Scope config) {}

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  @Override
  public String getDisplayType() {
    return "Manual Verification Config";
  }

  @Override
  public String getReferenceCategory() {
    return "manual-verification";
  }

  @Override
  public boolean isConfigurable() {
    return true;
  }

  private static AuthenticationExecutionModel.Requirement[] REQUIREMENT_CHOICES = {
    AuthenticationExecutionModel.Requirement.REQUIRED,
    AuthenticationExecutionModel.Requirement.DISABLED
  };

  @Override
  public Requirement[] getRequirementChoices() {
    return REQUIREMENT_CHOICES;
  }

  @Override
  public boolean isUserSetupAllowed() {
    return false;
  }

  @Override
  public String getHelpText() {
    return "Sets configuration for Manual Verification.";
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    ProviderConfigProperty messageCourier =
        new ProviderConfigProperty(
            MESSAGE_COURIER_ATTRIBUTE,
            "Confirmation Courier",
            "Send a confirmation notification of manual verification success.",
            ProviderConfigProperty.LIST_TYPE,
            MessageCourier.NONE.name());
    messageCourier.setOptions(
        asList(
            MessageCourier.BOTH.name(),
            MessageCourier.SMS.name(),
            MessageCourier.EMAIL.name(),
            MessageCourier.NONE.name()));

    // Define configuration properties
    return List.of(
        new ProviderConfigProperty(
            TEL_USER_ATTRIBUTE,
            "Telephone User Attribute",
            "Name of the user attribute used to retrieve the mobile telephone number of the user. Please make sure this is a read-only attribute for security reasons.",
            ProviderConfigProperty.STRING_TYPE,
            MessageOTPAuthenticator.MOBILE_NUMBER_FIELD),
        new ProviderConfigProperty(
            AUTO_2FA,
            "Automatic 2FA Email/SMS",
            "If enabled will configure the users 2FA to use the Email or SMS provided during registration.",
            ProviderConfigProperty.BOOLEAN_TYPE,
            false),
        messageCourier);
  }

  @Override
  public void authenticate(AuthenticationFlowContext context) {
    log.info("authenticate(): start");
  }

  @Override
  public void action(AuthenticationFlowContext context) {
    log.info("action(): start");
  }

  @Override
  public boolean requiresUser() {
    // This authenticator does not necessarily require an existing user
    return false;
  }

  @Override
  public boolean configuredFor(KeycloakSession session, RealmModel realm, UserModel user) {
    // Applicable for any user
    return true;
  }

  @Override
  public void setRequiredActions(KeycloakSession session, RealmModel realm, UserModel user) {
    // No additional required actions
  }
}
