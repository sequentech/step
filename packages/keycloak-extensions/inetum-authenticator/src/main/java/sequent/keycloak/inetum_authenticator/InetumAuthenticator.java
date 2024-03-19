package sequent.keycloak.inetum_authenticator;

import lombok.extern.jbosslog.JBossLog;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.Config;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.broker.provider.util.SimpleHttp;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.Response;

import java.io.IOException;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

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
		try {
			Map<String, String> transactionData = newTransaction(configMap, context);
			Response challenge = getBaseForm(context)
				.setAttribute("user_id", transactionData.get("user_id"))
				.setAttribute("token_dob", transactionData.get("token_dob"))
				.createForm(Utils.INETUM_FORM);
			context.challenge(challenge);
		} catch (IOException error) {
			context.failure(AuthenticationFlowError.INTERNAL_ERROR);
			context.attempted();
			Response challenge = getBaseForm(context)
				.setAttribute("error", "internalInetumError")
				.createForm(Utils.INETUM_ERROR);
			context.challenge(challenge);
		}
    }

	protected SimpleHttp.Response doPost(
		Map<String, String> configMap,
		AuthenticationFlowContext context,
		Object payload,
		String uriPath
	) throws IOException {
		String url = configMap.get(Utils.BASE_URL_ATTRIBUTE) + uriPath;
		String authorization = "Bearer " + configMap.get(Utils.API_KEY_ATTRIBUTE);
		log.info("doPost: url=" + url);
		log.info("doPost: authorization=" + authorization);
		log.info("doPost: payload=" + payload);

		SimpleHttp.Response response = SimpleHttp
			.doPost( url, context.getSession())
			.header("Content-Type", "application/json")
			.header("Authorization", authorization)
			.json(payload)
			.asResponse();
		return response;
	}

	protected Map<String, String> newTransaction(
		Map<String, String> configMap,
		AuthenticationFlowContext context
	) throws IOException
	{
		try {
			ObjectMapper objectMapper = new ObjectMapper();
			ObjectNode payloadNode = objectMapper.createObjectNode();

			payloadNode.put("wFtype_Facial", true);
			payloadNode.put("wFtype_OCR", true);
			payloadNode.put("wFtype_Video", false);
			payloadNode.put("wFtype_Anti_Spoofing", false);
			payloadNode.put("wFtype_Sign", false);
			payloadNode.put("wFtype_VerifAvan", false);
			payloadNode.put("wFtype_UECertificate", false);
			payloadNode.put("docID", "");
			payloadNode.put("name", "");
			payloadNode.put("lastname1", "");
			payloadNode.put("lastname2", "");
			payloadNode.put("country", "");
			payloadNode.put("mobilePhone", "");
			payloadNode.put("eMail", "");
			payloadNode.put("priority", 3);
			payloadNode.put("maxRetries", 3);
			payloadNode.put("maxProcessTime", 30);
			payloadNode.put("application", "sequent-keycloak");
			payloadNode.put("clienteID", configMap.get(Utils.CLIENT_ID_ATTRIBUTE));
			String payload = objectMapper.writeValueAsString(payloadNode);

			SimpleHttp.Response response = doPost(
				configMap,
				context,
				payloadNode,
				"/transaction/new"
			);

			if (response.getStatus() != 200) {
				log.error("Error calling transaction/new, status = " + response.getStatus());
				log.error("Error calling transaction/new, response.asString() = " + response.asString());
				throw new IOException(
					"Error calling transaction/new, status = " + response.getStatus()
				);
			}

			JsonNode responseContent = response.asJson().get("response");
			Map<String, String> output = new HashMap<String, String>();
			output.put("token_dob", responseContent.get("tokenDob").asText());
			output.put("user_id", responseContent.get("userID").asText());
			return output;
		} catch (IOException error) {
			log.error("Error calling transaction/new", error);
			throw error;
		}
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
				"https://des.digitalonboarding.es/dob-api/2.0.0"
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
 