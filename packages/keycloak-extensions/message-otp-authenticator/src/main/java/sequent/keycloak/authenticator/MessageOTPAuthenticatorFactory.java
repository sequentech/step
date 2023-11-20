package sequent.keycloak.authenticator;

import com.google.auto.service.AutoService;
import org.keycloak.Config;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.provider.ServerInfoAwareProviderFactory;

import java.util.List;
import java.util.Map;
import java.util.LinkedHashMap;

@AutoService(AuthenticatorFactory.class)
public class MessageOTPAuthenticatorFactory 
	implements AuthenticatorFactory, ServerInfoAwareProviderFactory
{

	public static final String PROVIDER_ID = "message-otp-authenticator";

	private static final MessageOTPAuthenticator SINGLETON = new MessageOTPAuthenticator();

	@Override
	public String getId() {
		return PROVIDER_ID;
	}

	@Override
	public String getDisplayType() {
		return "Email/SMS Authentication";
	}

	@Override
	public String getHelpText() {
		return "Validates an OTP sent via SMS and/or SMS to the users mobile phone or email address.";
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

    private static AuthenticationExecutionModel.Requirement[] REQUIREMENT_CHOICES = {
		AuthenticationExecutionModel.Requirement.REQUIRED,
		AuthenticationExecutionModel.Requirement.ALTERNATIVE,
		AuthenticationExecutionModel.Requirement.DISABLED
	};

	@Override
	public AuthenticationExecutionModel.Requirement[] getRequirementChoices() {
		return REQUIREMENT_CHOICES;
	}

	@Override
	public List<ProviderConfigProperty> getConfigProperties() {
		return List.of(
			new ProviderConfigProperty(
				Utils.CODE_LENGTH,
				"Code length",
				"The number of digits of the generated code.",
				ProviderConfigProperty.STRING_TYPE,
				6
			),
			new ProviderConfigProperty(
				Utils.CODE_TTL,
				"Time-to-live",
				"The time to live in seconds for the code to be valid.",
				ProviderConfigProperty.STRING_TYPE,
				"300"
			),
			new ProviderConfigProperty(
				Utils.SENDER_ID,
				"SenderId",
				"The SMS sender ID is displayed as the message sender on the receiving device.", ProviderConfigProperty.STRING_TYPE,
				"Keycloak"
			),
			new ProviderConfigProperty(
				Utils.EMAIL_USER_ATTRIBUTE,
				"Email User Attribute",
				"Name of the user attribute used to retrieve the email address of the user. Please make sure this is a read-only attribute for security reasons.", 
				ProviderConfigProperty.STRING_TYPE,
				MessageOTPAuthenticator.EMAIL_ADDRESS_FIELD
			),
			new ProviderConfigProperty(
				Utils.TEL_USER_ATTRIBUTE,
				"Telephone User Attribute",
				"Name of the user attribute used to retrieve the mobile telephone number of the user. Please make sure this is a read-only attribute for security reasons.", 
				ProviderConfigProperty.STRING_TYPE,
				MessageOTPAuthenticator.MOBILE_NUMBER_FIELD
			),
			new ProviderConfigProperty(
				Utils.SIMULATION_MODE,
				"Simulation mode",
				"In simulation mode, the SMS won't be sent, but printed to the server logs", ProviderConfigProperty.BOOLEAN_TYPE,
				true
			)
		);
	}

	@Override
	public Authenticator create(KeycloakSession session) {
		return SINGLETON;
	}

	@Override
	public void init(Config.Scope config) {
	}

	@Override
	public void postInit(KeycloakSessionFactory factory) {
	}

	@Override
	public void close() {
	}

    @Override
    public Map<String, String> getOperationalInfo() {
        Map<String, String> ret = new LinkedHashMap<>();
        ret.put("provider-id", getId());
		ret.put("reference-category", getReferenceCategory());
        return ret;
    }
}
