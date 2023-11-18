package sequent.keycloak.authenticator;

import sequent.keycloak.authenticator.gateway.SmsServiceFactory;
import sequent.keycloak.authenticator.gateway.EmailServiceFactory;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialProvider;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialProviderFactory;
import jakarta.ws.rs.core.Response;
import org.jboss.logging.Logger;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.CredentialValidator;
import org.keycloak.common.util.SecretGenerator;
import org.keycloak.credential.CredentialProvider;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.theme.Theme;

import java.util.Locale;
import java.util.Optional;

/**
 * @author Niko Köbler, https://www.n-k.de, @sequent
 */
public class MessageOTPAuthenticator implements Authenticator, CredentialValidator<MessageOTPCredentialProvider> {

	private static final Logger logger = Logger
		.getLogger(MessageOTPAuthenticator.class);
	public static final String MOBILE_NUMBER_FIELD = "read-only.mobile-number";
	public static final String EMAIL_ADDRESS_FIELD = "read-only.email-address";
	private static final String TPL_CODE = "login-message-otp.ftl";

	@Override
	public MessageOTPCredentialProvider getCredentialProvider(
		KeycloakSession session
	) {
		return (MessageOTPCredentialProvider) session
			.getProvider(
				CredentialProvider.class,
				MessageOTPCredentialProviderFactory.PROVIDER_ID
			);
	}

	@Override
	public void authenticate(AuthenticationFlowContext context) {
		logger.info("authenticate() called");
		AuthenticatorConfigModel config = context.getAuthenticatorConfig();
		KeycloakSession session = context.getSession();
		UserModel user = context.getUser();
		AuthenticationSessionModel authSession = context
			.getAuthenticationSession();

		try {
			Utils.sendCode(
				config,
				session,
				user,
				authSession
			);
			context
				.challenge(
					context.form().setAttribute("realm", context.getRealm()
				)
				.createForm(TPL_CODE));
		} catch (Exception error) {
			context.failureChallenge(
				AuthenticationFlowError.INTERNAL_ERROR,
				context
					.form()
					.setError("smsAuthSmsNotSent", error.getMessage())
					.createErrorPage(Response.Status.INTERNAL_SERVER_ERROR)
			);
		}
	}

	@Override
	public void action(AuthenticationFlowContext context) {
		logger.info("action() called");
		String enteredCode = context
			.getHttpRequest()
			.getDecodedFormParameters()
			.getFirst(Utils.CODE);

		AuthenticationSessionModel authSession = context
			.getAuthenticationSession();
		String code = authSession.getAuthNote(Utils.CODE);
		String ttl = authSession.getAuthNote(Utils.CODE_TTL);

		if (code == null || ttl == null) {
			context.failureChallenge(
				AuthenticationFlowError.INTERNAL_ERROR,
				context
					.form()
					.createErrorPage(Response.Status.INTERNAL_SERVER_ERROR)
			);
			return;
		}

		boolean isValid = enteredCode.equals(code);
		if (isValid) {
			if (Long.parseLong(ttl) < System.currentTimeMillis()) {
				// expired
				context.failureChallenge(
					AuthenticationFlowError.EXPIRED_CODE,
					context
						.form()
						.setError("messageOtpAuthCodeExpired")
						.createErrorPage(Response.Status.BAD_REQUEST)
				);
			} else {
				// valid
				context.success();
			}
		} else {
			// invalid
			AuthenticationExecutionModel execution = context.getExecution();
			if (execution.isRequired()) {
				context.failureChallenge(
					AuthenticationFlowError.INVALID_CREDENTIALS,
					context
						.form()
						.setAttribute("realm", context.getRealm())
						.setError("messageOtpAuthCodeInvalid")
						.createForm(TPL_CODE)
				);
			} else if (execution.isConditional() || execution.isAlternative()) {
				context.attempted();
			}
		}
	}

	@Override
	public boolean requiresUser() {
		logger.info("requiresUser() called");
		return true;
	}

	@Override
	public boolean configuredFor(
		KeycloakSession session,
		RealmModel realm,
		UserModel user
	) {
		logger.info("configuredFor() called");
		MessageOTPCredentialProvider provider = getCredentialProvider(session);
		if (
			provider == null ||
			!provider.isConfiguredFor(realm, user, getType(session))
		) {
			return false;
		}

		Optional<AuthenticatorConfigModel> config = Utils
			.getConfig(realm);

		// If no configuration is found, fall back to default behavior
	 	if (!config.isPresent()) {
			return user.getFirstAttribute(MOBILE_NUMBER_FIELD) != null;
		}
	
		String telUserAttribute = config
			.get()
			.getConfig()
			.get(Utils.TEL_USER_ATTRIBUTE);
		return user.getFirstAttribute(telUserAttribute) != null;
	}

	@Override
	public void setRequiredActions(
		KeycloakSession session,
		RealmModel realm,
		UserModel user
	) {
		logger.info("setRequiredActions() called");
		// this will only work if you have the required action from here
		// configured:
		// https://github.com/dasniko/keycloak-extensions-demo/tree/main/requiredaction
		//TODO:user.addRequiredAction(OTPMethodSelector.PROVIDER_ID);
	}

	@Override
	public void close() {
	}

}
