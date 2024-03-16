package sequent.keycloak.inetum_authenticator;

import sequent.keycloak.inetum_authenticator.Utils;
import org.apache.commons.lang3.StringUtils;
import org.keycloak.http.HttpCookie;
import org.keycloak.http.HttpResponse;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.Config;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.authentication.CredentialValidator;
import org.keycloak.authentication.RequiredActionFactory;
import org.keycloak.authentication.RequiredActionProvider;
import org.keycloak.credential.CredentialProvider;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserCredentialModel;
import org.keycloak.models.UserModel;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.sessions.AuthenticationSessionModel;

import com.google.auto.service.AutoService;

import jakarta.ws.rs.core.Cookie;
import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.Response;
import java.net.URI;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.security.MessageDigest;

@JBossLog
@AutoService(AuthenticatorFactory.class)
public class InetumAuthenticator implements Authenticator, AuthenticatorFactory
{
    public static final String PROVIDER_ID = "inetum-authenticator";
	private static final InetumAuthenticator SINGLETON = 
        new InetumAuthenticator();

    @Override
    public void authenticate(AuthenticationFlowContext context)
    {
        // Authentication is successful if the user already has the user's 
        // validation status attribute set to true, otherwise initiate a new 
        // flow and show form
        log.info("authenticate()");
        AuthenticatorConfigModel config = Utils
            .getConfig(context.getRealm())
            .get();
        Map<String, String> configMap = config.getConfig();
        UserModel user = context.getUser();

        if (user != null)
        {
            String statusAttributeName = configMap.get(Utils.USER_STATUS_ATTRIBUTE);
            String statusAttributeValue = user.getFirstAttribute(statusAttributeName);
            log.info("checking statusAttributeValue=" + statusAttributeValue);
            boolean validated = (
                statusAttributeValue != null && statusAttributeValue.equals("TRUE")
            );

            log.info("validated=" + validated);
            if (validated)
            {
                log.info("validated IS TRUE, pass");
                context.success();
                return;
            }
        }

        log.info("validated is NOT TRUE, rendering the form");
        Response challenge = getBaseForm(context)
            .createForm(Utils.INETUM_FORM);
        context.challenge(challenge);
    }
 
    @Override
    public void action(AuthenticationFlowContext context)
    {
        log.info("action()");
        boolean validated = validateAnswer(context);
        if (!validated)
        {
			// invalid
			AuthenticationExecutionModel execution = context.getExecution();
			if (execution.isRequired())
            {
				context.failureChallenge(
					AuthenticationFlowError.INVALID_CREDENTIALS,
					getBaseForm(context)
						.setError("authInvalid")
						.createForm(Utils.INETUM_FORM)
				);
			} else if (execution.isConditional() || execution.isAlternative())
            {
				context.attempted();
			}
        } else {
            // valid
            context.success();
        }
    }
 
    protected boolean validateAnswer(AuthenticationFlowContext context)
    {
        return true;
    }

    protected LoginFormsProvider getBaseForm(AuthenticationFlowContext context)
    {
        AuthenticatorConfigModel config = Utils
            .getConfig(context.getRealm())
            .get();
        Map<String, String> configMap = config.getConfig();
        return context
            .form()
            .setAttribute("realm", context.getRealm())
            .setAttribute("api_key", configMap.get(Utils.API_KEY_ATTRIBUTE))
            .setAttribute("app_id", configMap.get(Utils.APP_ID_ATTRIBUTE))
            .setAttribute("client_id", configMap.get(Utils.CLIENT_ID_ATTRIBUTE))
            .setAttribute("base_url", configMap.get(Utils.BASE_URL_ATTRIBUTE));
    }
 
    @Override
    public boolean requiresUser() {
        return false;
    }
 
     @Override
     public boolean configuredFor(
        KeycloakSession session,
        RealmModel realm,
        UserModel user
    ) {
        return false;
    }
 
     @Override
     public void setRequiredActions(
        KeycloakSession session,
        RealmModel realm,
        UserModel user
    ) {
    }

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
				Utils.API_KEY_ATTRIBUTE,
				"API KEY",
				"-",
				ProviderConfigProperty.STRING_TYPE,
				""
			),
			new ProviderConfigProperty(
				Utils.APP_ID_ATTRIBUTE,
				"APP ID",
				"-",
				ProviderConfigProperty.STRING_TYPE,
				""
			),
			new ProviderConfigProperty(
				Utils.CLIENT_ID_ATTRIBUTE,
				"CLIENT ID",
				"-",
				ProviderConfigProperty.STRING_TYPE,
				""
			),
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
			),
			new ProviderConfigProperty(
				Utils.BASE_URL_ATTRIBUTE,
				"Base URL for Inetum API",
				"-",
				ProviderConfigProperty.STRING_TYPE,
				"https://des.digitalonboarding.es/"
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
 