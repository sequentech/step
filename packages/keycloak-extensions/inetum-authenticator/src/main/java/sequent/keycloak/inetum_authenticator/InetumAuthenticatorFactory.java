package sequent.keycloak.inetum_authenticator;

import com.google.auto.service.AutoService;
import org.keycloak.Config;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.provider.ProviderConfigProperty;

import java.util.List;

@AutoService(AuthenticatorFactory.class)
public class InetumAuthenticatorFactory
	implements AuthenticatorFactory
{
	public static final String PROVIDER_ID = "inetum-authenticator";

	private static final InetumAuthenticator SINGLETON = 
        new InetumAuthenticator();

	@Override
	public String getId() {
		return PROVIDER_ID;
	}

	@Override
	public String getDisplayType() {
		return "Inetum Authentication";
	}

	@Override
	public String getHelpText() {
		return "Validates the User using Inetum Platform.";
	}

	@Override
	public String getReferenceCategory() {
		return "External Authenticator";
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
				Utils.USER_DATA_ATTRIBUTE,
				"User Data Attribute",
				"The name of the user data attribute to check against.",
				ProviderConfigProperty.STRING_TYPE,
				"sequent.read-only.id-card-number"
			),
			new ProviderConfigProperty(
				Utils.USER_STATUS_ATTRIBUTE,
				"User Status Attribute",
				"The name of the user validation status attribute.",
				ProviderConfigProperty.STRING_TYPE,
				"sequent.read-only.id-card-number-validated"
			),
			new ProviderConfigProperty(
				Utils.SDK_ATTRIBUTE,
				"Configuration for the SDK",
				"-",
				ProviderConfigProperty.STRING_TYPE,
				"{}"
			),
			new ProviderConfigProperty(
				Utils.ENV_CONFIG_ATTRIBUTE,
				"Configuration for the env_config",
				"-",
				ProviderConfigProperty.STRING_TYPE,
				"{}"
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
}
