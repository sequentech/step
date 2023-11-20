package sequent.keycloak.authenticator;

import sequent.keycloak.authenticator.credential.MessageOTPCredentialModel;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialProvider;
import org.jboss.logging.Logger;
import jakarta.ws.rs.core.Response;
import org.keycloak.authentication.InitiatedActionSupport;
import org.keycloak.authentication.RequiredActionContext;
import org.keycloak.authentication.RequiredActionProvider;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;

import java.util.function.Consumer;
import java.util.Optional;

public class ResetMessageOTPRequiredAction implements RequiredActionProvider {
	private static final Logger logger = Logger
		.getLogger(ResetMessageOTPRequiredAction.class);

	public static final String PROVIDER_ID = "message-otp-ra";

    public static final String IS_SETUP_FIELD = "is-setup";
    private static final String FTL_RESET_MESSAGE_OTP = "reset-message-otp.ftl";


	public MessageOTPCredentialProvider getCredentialProvider(
		KeycloakSession session
	) {
		logger.info("getCredentialProvider()");
		return new MessageOTPCredentialProvider(session);
		// TODO: doesn't work - why?
		// return (MessageOTPCredentialProvider) session
		// 	.getProvider(
		// 		CredentialProvider.class,
		// 		MessageOTPCredentialProviderFactory.PROVIDER_ID
		// 	);
	}

	@Override
	public InitiatedActionSupport initiatedActionSupport() {
		return InitiatedActionSupport.SUPPORTED;
	}

	@Override
	public void evaluateTriggers(RequiredActionContext context) {
	}

	@Override
	public void requiredActionChallenge(RequiredActionContext context) {
		context.challenge(createForm(context, null));
	}

	@Override
	public void processAction(RequiredActionContext context) {
		logger.info("action() called");
		
		UserModel user = context.getUser();
		String enteredCode = context
			.getHttpRequest()
			.getDecodedFormParameters()
			.getFirst(Utils.CODE);

		AuthenticationSessionModel authSession = context
			.getAuthenticationSession();
		String code = authSession.getAuthNote(Utils.CODE);
		String ttl = authSession.getAuthNote(Utils.CODE_TTL);

		if (code == null || ttl == null) {
			context.failure();
			return;
		}

		boolean isValid = enteredCode.equals(code);
		if (isValid) {
			if (Long.parseLong(ttl) < System.currentTimeMillis()) {
				// expired
				context.challenge(
					createForm(
						context,
						form -> form
							.setError("messageOtpAuthCodeExpired")
							.createErrorPage(Response.Status.BAD_REQUEST)
					)
				);
				return;
			}
		} else {
			// invalid
			context.challenge(
				createForm(
					context,
					form -> form
						.setError("messageOtpAuthCodeInvalid")
						.createErrorPage(Response.Status.BAD_REQUEST)
				)
			);
			return;
		}

		// Generate a MessageOTP credential for the user and remove the required
		// action
		MessageOTPCredentialProvider credentialProvider = getCredentialProvider(
			context.getSession()
		);
        credentialProvider
			.createCredential(
				context.getRealm(),
				context.getUser(),
				MessageOTPCredentialModel.create(/* isSetup= */ true)
			);

		user.removeRequiredAction(PROVIDER_ID);
		context.getAuthenticationSession().removeRequiredAction(PROVIDER_ID);

		context.success();
	}

	@Override
	public void close() {
	}

	private Response createForm(
        RequiredActionContext context,
        Consumer<LoginFormsProvider> formConsumer
    ) {
        Optional<AuthenticatorConfigModel> config = Utils
            .getConfig(context.getRealm());
		KeycloakSession session = context.getSession();
		UserModel user = context.getUser();
		AuthenticationSessionModel authSession = context
			.getAuthenticationSession();
        try {
            Utils.sendCode(
                config.get(),
                session,
                user,
                authSession
            );
        } catch (Exception error) {
			context.failure();
        }

		LoginFormsProvider form = context.form();
		form.setAttribute("realm", context.getRealm());

		if (formConsumer != null) {
			formConsumer.accept(form);
		}

		return form.createForm(FTL_RESET_MESSAGE_OTP);
	}
}
