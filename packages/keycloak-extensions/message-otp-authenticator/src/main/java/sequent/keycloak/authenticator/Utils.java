package sequent.keycloak.authenticator;

import org.keycloak.common.util.SecretGenerator;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.RealmModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.theme.Theme;

import lombok.experimental.UtilityClass;
import sequent.keycloak.authenticator.gateway.EmailServiceFactory;
import sequent.keycloak.authenticator.gateway.SmsServiceFactory;

import java.io.IOException;
import java.util.Locale;
import java.util.Optional;

@UtilityClass
public class Utils {
	public String CODE = "code";
	public String CODE_LENGTH = "length";
	public String CODE_TTL = "ttl";
	public String SENDER_ID = "senderId";
	public String SIMULATION_MODE = "simulation";
	public String TEL_USER_ATTRIBUTE = "telUserAttribute";
	public String EMAIL_USER_ATTRIBUTE = "emailUserAttribute";

	void sendCode(
		AuthenticatorConfigModel config,
		KeycloakSession session,
		UserModel user,
		AuthenticationSessionModel authSession
	) throws IOException
	{
		Theme theme = session.theme().getTheme(Theme.Type.LOGIN);
		String telUserAttribute = config
			.getConfig()
			.get(Utils.TEL_USER_ATTRIBUTE);
		String mobileNumber = user.getFirstAttribute(telUserAttribute);

		String emailUserAttribute = config
			.getConfig()
			.get(Utils.EMAIL_USER_ATTRIBUTE);
		String emailAddress = user.getFirstAttribute(emailUserAttribute);

		int length = Integer.parseInt(
			config.getConfig().get(Utils.CODE_LENGTH)
		);
		int ttl = Integer.parseInt(
			config.getConfig().get(Utils.CODE_TTL)
		);

		String code = SecretGenerator
			.getInstance()
			.randomString(length, SecretGenerator.DIGITS);
		authSession.setAuthNote(Utils.CODE, code);
		authSession.setAuthNote(
			Utils.CODE_TTL,
			Long.toString(System.currentTimeMillis() + (ttl * 1000L))
		);

		Locale locale = session.getContext().resolveLocale(user);
		String smsAuthText = theme
			.getMessages(locale)
			.getProperty("smsAuthText");
		String smsText = String
			.format(smsAuthText, code, Math.floorDiv(ttl, 60));

		String emailAuthTitle = theme
			.getMessages(locale)
			.getProperty("emailAuthTitle");
		String emailTitle = String
			.format(emailAuthTitle, code, Math.floorDiv(ttl, 60));

		String emailAuthBody = theme
			.getMessages(locale)
			.getProperty("emailAuthBody");
		String emailBody = String
			.format(emailAuthBody, code, Math.floorDiv(ttl, 60));

		String emailAuthHtmlBody = theme
			.getMessages(locale)
			.getProperty("emailAuthHtmlBody");
		String emailHtmlBody = String
			.format(emailAuthHtmlBody, code, Math.floorDiv(ttl, 60));

		if (mobileNumber != null) {
			SmsServiceFactory
				.get(config.getConfig())
				.send(mobileNumber, smsText);
		}
		if (emailAddress != null) {
			EmailServiceFactory
				.get(config.getConfig())
				.send(
					emailAddress,
					emailTitle,
					emailBody,
					emailHtmlBody
				);
		}
	}

	Optional<AuthenticatorConfigModel> getConfig(RealmModel realm) {
		// Using streams to find the first matching configuration
		// TODO: We're assuming there's only one instance in this realm of this 
		// authenticator
		Optional<AuthenticatorConfigModel> configOptional = realm
			.getAuthenticationFlowsStream()
			.flatMap(flow ->
				realm.getAuthenticationExecutionsStream(flow.getId())
			)
			.filter(model -> {
				boolean ret = (
					model.getAuthenticator() != null &&
					model.getAuthenticator()
						.equals(MessageOTPAuthenticatorFactory.PROVIDER_ID)
				);
				return ret;
			})
			.map(model ->
				realm.getAuthenticatorConfigById(model.getAuthenticatorConfig())
			)
			.findFirst();
		return configOptional;
	}
}
