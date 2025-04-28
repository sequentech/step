// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import static java.util.Arrays.asList;

import com.google.auto.service.AutoService;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import org.keycloak.Config;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.provider.ServerInfoAwareProviderFactory;

@AutoService(AuthenticatorFactory.class)
public class MessageOTPAuthenticatorFactory
    implements AuthenticatorFactory, ServerInfoAwareProviderFactory {
  public static final String PROVIDER_ID = "message-otp-authenticator";

  private static AuthenticationExecutionModel.Requirement[] REQUIREMENT_CHOICES = {
    AuthenticationExecutionModel.Requirement.REQUIRED,
    AuthenticationExecutionModel.Requirement.ALTERNATIVE,
    AuthenticationExecutionModel.Requirement.DISABLED
  };

  private static final MessageOTPAuthenticator SINGLETON = new MessageOTPAuthenticator();

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  @Override
  public String getDisplayType() {
    return "OTP - Message via Email/SMS";
  }

  @Override
  public String getHelpText() {
    return "Validates an OTP sent via SMS and/or SMS to the users mobile phone and/or email address.";
  }

  @Override
  public String getReferenceCategory() {
    return "otp";
  }

  @Override
  public boolean isConfigurable() {
    return true;
  }

  @Override
  public boolean isUserSetupAllowed() {
    return true;
  }

  @Override
  public AuthenticationExecutionModel.Requirement[] getRequirementChoices() {
    return REQUIREMENT_CHOICES;
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    ProviderConfigProperty messageCourier =
        new ProviderConfigProperty(
            Utils.MESSAGE_COURIER_ATTRIBUTE,
            "Message Courier",
            "Choose if the message is going to be sent via email, sms or both.",
            ProviderConfigProperty.LIST_TYPE,
            Utils.MessageCourier.BOTH.name());
    messageCourier.setOptions(
        asList(
            Utils.MessageCourier.BOTH.name(),
            Utils.MessageCourier.SMS.name(),
            Utils.MessageCourier.EMAIL.name()));
    return List.of(
        new ProviderConfigProperty(
            Utils.ONE_TIME_LINK,
            "Use OTL instead of OTP",
            "Send One Time Link instead of One Time Password.",
            ProviderConfigProperty.BOOLEAN_TYPE,
            false),
        new ProviderConfigProperty(
            Utils.CODE_LENGTH,
            "Code length",
            "The number of digits of the generated code.",
            ProviderConfigProperty.STRING_TYPE,
            6),
        new ProviderConfigProperty(
            Utils.CODE_TTL,
            "Time-to-live",
            "The time to live in seconds for the code to be valid.",
            ProviderConfigProperty.STRING_TYPE,
            "300"),
        new ProviderConfigProperty(
            Utils.SENDER_ID,
            "SenderId",
            "The SMS sender ID is displayed as the message sender on the receiving device.",
            ProviderConfigProperty.STRING_TYPE,
            "Keycloak"),
        new ProviderConfigProperty(
            Utils.TEL_USER_ATTRIBUTE,
            "Telephone User Attribute",
            "Name of the user attribute used to retrieve the mobile telephone number of the user. Please make sure this is a read-only attribute for security reasons.",
            ProviderConfigProperty.STRING_TYPE,
            MessageOTPAuthenticator.MOBILE_NUMBER_FIELD),
        new ProviderConfigProperty(
            Utils.DEFERRED_USER_ATTRIBUTE,
            "Use Deferred User",
            "If enabled, there won't be a need to have a valid user when using this authenticator",
            ProviderConfigProperty.BOOLEAN_TYPE,
            "false"),
        new ProviderConfigProperty(
            Utils.OTL_RESTORED_AUTH_NOTES_ATTRIBUTE,
            "Comma Separated Names of the Auth Notes to Restore",
            "When loading an OTL, these are the Auth Notes that will be restored from the previous session",
            ProviderConfigProperty.STRING_TYPE,
            ""),
        new ProviderConfigProperty(
            Utils.RESEND_ACTIVATION_TIMER,
            "Seconds to activate resend",
            "Time in seconds the resend code gets re activated",
            ProviderConfigProperty.STRING_TYPE,
            "60"),
        new ProviderConfigProperty(
            Utils.TEST_MODE_ATTRIBUTE,
            "Test Mode",
            "If true, the otp will accept specific code that recive from Test Mode Code field",
            ProviderConfigProperty.BOOLEAN_TYPE,
            "false"),
        new ProviderConfigProperty(
            Utils.TEST_MODE_CODE_ATTRIBUTE,
            "Test Mode Code",
            "Will be used for test mode. code will contain only digit and with the same number of digits as Code length specify",
            ProviderConfigProperty.STRING_TYPE,
            "123456"),
        messageCourier);
  }

  @Override
  public Authenticator create(KeycloakSession session) {
    return SINGLETON;
  }

  @Override
  public void init(Config.Scope config) {}

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public void close() {}

  @Override
  public Map<String, String> getOperationalInfo() {
    Map<String, String> ret = new LinkedHashMap<>();
    ret.put("provider-id", getId());
    ret.put("reference-category", getReferenceCategory());
    return ret;
  }
}
