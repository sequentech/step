package sequent.keycloak.adyen;

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

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.Response;

import com.adyen.Client;
import com.adyen.service.checkout.PaymentsApi;
import com.adyen.model.checkout.Amount;
import com.adyen.model.checkout.CreateCheckoutSessionRequest;
import com.adyen.model.checkout.CreateCheckoutSessionResponse;
import com.adyen.model.checkout.Name;
import com.adyen.enums.Environment;
import com.adyen.service.exception.ApiException;

import java.io.IOException;
import java.util.List;
import java.util.Map;
import java.util.UUID;

 
@JBossLog
@AutoService(AuthenticatorFactory.class)
public class AdyenAuthenticator implements Authenticator, AuthenticatorFactory
{
    public static final String PROVIDER_ID = "adyen-authenticator";
	private static final AdyenAuthenticator SINGLETON = 
        new AdyenAuthenticator();

	protected Environment getEnvironment(Map<String, String> configMap)
	{
		if (configMap.get(Utils.ENVIRONMENT_ATTRIBUTE).equals("LIVE")) {
			return Environment.LIVE;
		} else {
			return Environment.TEST;
		}
	}

	protected Client getClient(Map<String, String> configMap)
		throws IOException, ApiException
	{
		return new Client(
			configMap.get(Utils.API_KEY_ATTRIBUTE),
			getEnvironment(configMap)
		);
	}

	/**
	 * Returns the payment reference for this user. If it is not found, 
	 * generates a new payment reference and saves it to the user attribute.
	 */
	protected String getPaymentReference(AuthenticationFlowContext context)
	{
		UserModel user = context.getUser();
		if (user == null) {
			return UUID.randomUUID().toString();
		}
		return user.getEmail();
	}

	/**
	 * The shopper's country code. This is used to filter the list of available
	 * payment methods to your shopper.
	 */
	protected String getCountryCode()
	{
		// TODO
		return "US";
	}

	/**
	 * Save the sessionData and sessionId onto the user attribute.
	 */
	protected void saveSession(
		AuthenticationFlowContext context,
		CreateCheckoutSessionResponse session
	) {
		log.info("saveSession()");
		UserModel user = context.getUser();
		ObjectMapper mapper = new ObjectMapper();

        AuthenticatorConfigModel config = Utils
            .getConfig(context.getRealm())
            .get();
        Map<String, String> configMap = config.getConfig();

		// Constructing the JSON object to store session data
		ObjectNode sessionInfo = mapper.createObjectNode();
		sessionInfo.put(Utils.SESSION_ID, session.getId());
		sessionInfo.put(Utils.SESSION_STATUS, Utils.STATUS_CREATED);

		saveUserSession(user, configMap, sessionInfo);
	}

	/*
	 * Creates a new checkout session 
	 */
	protected CreateCheckoutSessionResponse newSession(
		Client client,
		Map<String, String> configMap,
		AuthenticationFlowContext context
	)
		throws IOException, ApiException
	{
		log.info("newSession()");
		UserModel user = context.getUser();
		Amount amount = new Amount()
			.currency(configMap.get(Utils.CURRENCY_ATTRIBUTE))
			.value(Long.valueOf(configMap.get(Utils.AMOUNT_ATTRIBUTE)));
 
		Name userName = new Name()
			.firstName(user.getFirstName())
			.lastName(user.getLastName());
		
		CreateCheckoutSessionRequest createCheckoutSessionRequest = 
			new CreateCheckoutSessionRequest()
			.amount(amount)
			.merchantAccount(configMap.get(Utils.MERCHANT_ACCOUNT_ATTRIBUTE))
			.shopperEmail(user.getEmail())
			.shopperName(userName)
			.shopperReference(user.getId())
			.returnUrl("https://example.com/TODO")
			.reference(getPaymentReference(context))
			.countryCode(getCountryCode());
		 
		PaymentsApi checkoutPaymentsApi = new PaymentsApi(client);
		CreateCheckoutSessionResponse session = 
			checkoutPaymentsApi.sessions(createCheckoutSessionRequest);

		return session;
	}

	/**
	 * Called by authenticate() to render the basic form with the Adyen web
	 * drop-in widget to process the payment.
	 */
	protected void renderBaseForm(
		Map<String, String> configMap,
		AuthenticationFlowContext context
	) {
        log.info("renderBaseForm()");
		UserModel user = context.getUser();
		try {
			Client client = getClient(configMap);
			CreateCheckoutSessionResponse session =
				newSession(client, configMap, context);
			saveSession(context, session);
			Response challenge = getBaseForm(context)
				.setAttribute(
					"adyen_holder_name",
					user.getFirstName() + " " + user.getLastName()
				)
				.setAttribute("adyen_session_data", session.getSessionData())
				.setAttribute("adyen_session_id", session.getId())
				.createForm(Utils.ADYEN_FORM);
			context.challenge(challenge);
		} catch (Exception error) {
			log.error("renderBaseForm(): ERROR: " + error.toString());
			context.failure(AuthenticationFlowError.INTERNAL_ERROR);
			context.attempted();
			Response challenge = getBaseForm(context)
				.setAttribute("adyen_error", "adyen.form.error.generic")
				.createForm(Utils.ADYEN_ERROR);
			context.challenge(challenge);
		}
	}

	protected JsonNode getUserSession(
		UserModel user,
		Map<String, String> configMap
	) {
		log.info("getUserSession: start");
		if (user == null) {
			log.info("getUserSession: 1");
			return null;
		}
		String statusAttributeName = configMap.get(Utils.USER_STATUS_ATTRIBUTE);
		if (statusAttributeName == null) {
			log.info("getUserSession: 2");
			return null;
		}
		String statusStr = user.getFirstAttribute(statusAttributeName);
		if (statusStr == null) {
			log.info("getUserSession: 3");
			return null;
		}
		ObjectMapper mapper = new ObjectMapper();
		try {
			JsonNode status = 
				mapper.readValue(statusStr, JsonNode.class);

			log.info("getUserSession: success");
			return status;
		} catch (Exception _error) {
			log.info("getUserSession: 4");
			return null;
		}
	}

	protected void saveUserSession(
		UserModel user,
		Map<String, String> configMap,
		JsonNode sessionInfo
	) {
		log.info("saveUserSession: start");

		ObjectMapper mapper = new ObjectMapper();
		// Convert the JSON object to a string
		String sessionInfoStr;
		try {
			sessionInfoStr = mapper.writeValueAsString(sessionInfo);
		} catch (JsonProcessingException error) {
			log.error("Error serializing session info", error);
			throw new RuntimeException("Error serializing session info", error);
		}

		String statusAttributeName = configMap.get(Utils.USER_STATUS_ATTRIBUTE);
		if (statusAttributeName == null) {
			throw new RuntimeException("Utils.USER_STATUS_ATTRIBUTE not found");
		}

		try {
			log.info("saving attribute name=" + statusAttributeName + ", value=`" + sessionInfoStr + "`");
			user.setSingleAttribute(statusAttributeName, sessionInfoStr);
		} catch (Exception error) {
			throw new RuntimeException("Error saving session", error);
		}

		// Log the action for debug purposes
		log.info("saveUserSession: Saved session info for user: " + user.getUsername());
	}

	/**
	 * Returns true if the status of the user is that they payment stored in its
	 * attributes is valid.
	 */
	protected boolean hasUserAlreadyPaid(
		AuthenticationFlowContext context,
		Map<String, String> configMap
	) {
		log.info("hasUserAlreadyPaid()");
		UserModel user = context.getUser();
		JsonNode status = getUserSession(user, configMap);
		if (status == null) {
			return false;
		}
		String sessionId = status.get(Utils.SESSION_ID).asText();
		String sessionStatus = status.get(Utils.SESSION_STATUS).asText();

		if (
			sessionId == null || !sessionId.isEmpty() ||
			sessionStatus == null || !sessionStatus.equals("SUCCESS")
		) {
			return false;
		}
		return true;
	}

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

		boolean userHasPaid = hasUserAlreadyPaid(context, configMap);
        log.info("authenticate(): userHasPaid = " + userHasPaid);

		if (userHasPaid) {
			context.success();
			return;
		} else {
			renderBaseForm(configMap, context);
			return;
		}
    }

	protected boolean validatePayment(AuthenticationFlowContext context)
	{

		log.info("action()");
		UserModel user = context.getUser();
		if (user == null) {
			log.info("action(): 1");
			return false;
		}

        AuthenticatorConfigModel config = Utils
            .getConfig(context.getRealm())
            .get();
        Map<String, String> configMap = config.getConfig();
		JsonNode sessionInfo = getUserSession(user, configMap);

		if (sessionInfo == null) {
			log.info("action(): 2");
			return false;
		}

		String sessionId = sessionInfo.get(Utils.SESSION_ID).asText();
		String sessionStatus = sessionInfo.get(Utils.SESSION_STATUS).asText();
		if (
			sessionId == null ||
			sessionStatus == null ||
			!sessionStatus.equals(Utils.STATUS_CREATED)
		) {
			log.info("action(): 3");
			return false;
		}
		ObjectMapper mapper = new ObjectMapper();

		// Constructing the JSON object to store session data
		ObjectNode newSessionInfo = mapper.createObjectNode();
		newSessionInfo.put(Utils.SESSION_ID, sessionId);
		newSessionInfo.put(Utils.SESSION_STATUS, Utils.STATUS_SUCCESS);
		saveUserSession(user, configMap, newSessionInfo);

		return true;
	}

    @Override
    public void action(AuthenticationFlowContext context)
    {
		log.info("action()");
        boolean validated = validatePayment(context);
        if (!validated)
        {
			// invalid
			AuthenticationExecutionModel execution = context.getExecution();
			if (execution.isRequired())
            {
				// TODO: handle error correctly
				context.failureChallenge(
					AuthenticationFlowError.INVALID_CREDENTIALS,
					getBaseForm(context)
						.setAttribute("adyen_error", "adyen.form.error.generic")
						.createForm(Utils.ADYEN_ERROR)
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

    protected LoginFormsProvider getBaseForm(AuthenticationFlowContext context)
    {
        AuthenticatorConfigModel config = Utils
            .getConfig(context.getRealm())
            .get();
        Map<String, String> configMap = config.getConfig();
        return context
            .form()
            .setAttribute("realm", context.getRealm())
            .setAttribute("adyen_client_key", configMap.get(Utils.CLIENT_KEY_ATTRIBUTE))
            .setAttribute("adyen_environment", getEnvironment(configMap));
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
		return "Adyen Authentication";
	}

	@Override
	public String getHelpText() {
		return "Validates that the User has a payment associated.";
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
				"",
				ProviderConfigProperty.STRING_TYPE,
				""
			),
			new ProviderConfigProperty(
				Utils.CLIENT_KEY_ATTRIBUTE,
				"CLIENT KEY",
				"",
				ProviderConfigProperty.STRING_TYPE,
				""
			),
			new ProviderConfigProperty(
				Utils.MERCHANT_ACCOUNT_ATTRIBUTE,
				"MERCHANT ACCOUNT",
				"",
				ProviderConfigProperty.STRING_TYPE,
				""
			),
			new ProviderConfigProperty(
				Utils.ENVIRONMENT_ATTRIBUTE,
				"ENVIRONMENT",
				"",
				ProviderConfigProperty.STRING_TYPE,
				"TEST"
			),
			new ProviderConfigProperty(
				Utils.AMOUNT_ATTRIBUTE,
				"AMOUNT",
				"-",
				ProviderConfigProperty.STRING_TYPE,
				""
			),
			new ProviderConfigProperty(
				Utils.CURRENCY_ATTRIBUTE,
				"CURRENCY",
				"",
				ProviderConfigProperty.STRING_TYPE,
				"USD"
			),
			new ProviderConfigProperty(
				Utils.USER_STATUS_ATTRIBUTE,
				"User Status Attribute",
				"The name of the adyen user status attribute.",
				ProviderConfigProperty.STRING_TYPE,
				"sequent.read-only.adyen-status"
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
