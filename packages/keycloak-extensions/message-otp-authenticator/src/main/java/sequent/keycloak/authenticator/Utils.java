package sequent.keycloak.authenticator;

import org.keycloak.common.util.SecretGenerator;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.RealmModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.theme.Theme;
import lombok.extern.jbosslog.JBossLog;

import lombok.experimental.UtilityClass;
import sequent.keycloak.authenticator.gateway.EmailServiceFactory;
import sequent.keycloak.authenticator.gateway.SmsServiceFactory;

import java.io.IOException;
import java.util.Locale;
import java.util.Optional;
import java.util.Map;

@UtilityClass
@JBossLog
public class Utils {
	public String CODE = "code";
	public String CODE_LENGTH = "length";
	public String CODE_TTL = "ttl";
	public String SENDER_ID = "senderId";
	public String SIMULATION_MODE = "simulation";
	public String TEL_USER_ATTRIBUTE = "telUserAttribute";
	public String EMAIL_USER_ATTRIBUTE = "emailUserAttribute";

	/**
	 * Sends code and also sets the auth notes related to the code
	 */
	void sendCode(
		AuthenticatorConfigModel config,
		KeycloakSession session,
		UserModel user,
		AuthenticationSessionModel authSession
	) throws IOException
	{
		log.info("sendCode()");
		Theme theme = session.theme().getTheme(Theme.Type.LOGIN);
		String mobileNumber = Utils.getMobile(config, user);
		String emailAddress = Utils.getEmail(config, user);

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

		if (mobileNumber != null) {
			String smsAuthText = theme
				.getMessages(locale)
				.getProperty("smsAuthText");
			String smsText = String
				.format(smsAuthText, code, Math.floorDiv(ttl, 60));
			SmsServiceFactory
				.get(config.getConfig())
				.send(mobileNumber, smsText);
		}
		if (emailAddress != null) {
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

	String getMobile(AuthenticatorConfigModel config, UserModel user)
	{
		log.infov("getMobile()");
		if (config == null) {
			log.infov("getMobile(): NULL config={0}", config);
			return user.getFirstAttribute(
				MessageOTPAuthenticator.MOBILE_NUMBER_FIELD
			);
		}

		Map<String, String> mapConfig = config.getConfig();
		if (
			mapConfig == null ||
			!mapConfig.containsKey(Utils.TEL_USER_ATTRIBUTE))
		{
			log.infov("getEmail(): NullOrNotFound mapConfig={0}", mapConfig);
			return user.getFirstAttribute(
				MessageOTPAuthenticator.MOBILE_NUMBER_FIELD
			);
		}
		String telUserAttribute = mapConfig.get(Utils.TEL_USER_ATTRIBUTE);

		String mobile = user.getFirstAttribute(telUserAttribute);
		log.infov(
			"getMobile(): telUserAttribute={0}, mobile={1}",
			telUserAttribute,
			mobile
		);
		return mobile;
	}

	String getEmail(AuthenticatorConfigModel config, UserModel user)
	{
		log.infov("getEmail()");
		if (config == null) {
			log.infov("getEmail(): NULL config={0}", config);
			return user.getFirstAttribute(
				MessageOTPAuthenticator.EMAIL_ADDRESS_FIELD
			);
		}

		Map<String, String> mapConfig = config.getConfig();
		if (
			mapConfig == null ||
			!mapConfig.containsKey(Utils.EMAIL_USER_ATTRIBUTE)
		) {
			log.infov("getEmail(): NullOrNotFound mapConfig={0}", mapConfig);
			return user.getFirstAttribute(
				MessageOTPAuthenticator.EMAIL_ADDRESS_FIELD
			);
		}
		String emailUserAttribute = mapConfig.get(Utils.EMAIL_USER_ATTRIBUTE);

		String email = user.getFirstAttribute(emailUserAttribute);
		log.infov(
			"getEmail(): emailUserAttribute={0}, email={1}",
			emailUserAttribute,
			email
		);
		return email;
	}

	Optional<AuthenticatorConfigModel> getConfig(RealmModel realm)
	{
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
					model
						.getAuthenticator()
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

	/**
	 * We use constant time comparison for security reasons, to avoid timing
	 * attacks
	 */
	boolean constantTimeIsEqual(byte[] digesta, byte[] digestb)
	{
		if (digesta.length != digestb.length) {
			return false;
		}
	
		int result = 0;
		// time-constant comparison
		for (int i = 0; i < digesta.length; i++) {
			result |= digesta[i] ^ digestb[i];
		}
		return result == 0;
	}
}
