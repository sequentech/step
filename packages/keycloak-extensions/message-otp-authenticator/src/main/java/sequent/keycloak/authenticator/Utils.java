package sequent.keycloak.authenticator;

import org.keycloak.common.util.SecretGenerator;
import org.keycloak.email.EmailTemplateProvider;
import org.keycloak.email.EmailException;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.RealmModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.theme.Theme;

import com.google.common.base.Strings;
import com.google.common.collect.ImmutableList;
import com.google.common.collect.Maps;

import lombok.extern.jbosslog.JBossLog;

import lombok.experimental.UtilityClass;
import sequent.keycloak.authenticator.gateway.SmsServiceFactory;

import java.io.IOException;
import java.util.List;
import java.util.Locale;
import java.util.Optional;
import java.util.Map;

@UtilityClass
@JBossLog
public class Utils {
	public final String CODE = "code";
	public final String CODE_LENGTH = "length";
	public final String CODE_TTL = "ttl";
	public final String SENDER_ID = "senderId";
	public final String SIMULATION_MODE = "simulation";
	public final String TEL_USER_ATTRIBUTE = "telUserAttribute";
	public final String EMAIL_USER_ATTRIBUTE = "emailUserAttribute";
	public final String SEND_CODE_EMAIL_SUBJECT = "messageOtp.sendCode.email.subject";
	public final String SEND_CODE_EMAIL_FTL = "send-code-email.ftl";

	/**
	 * Sends code and also sets the auth notes related to the code
	 */
	void sendCode(
		AuthenticatorConfigModel config,
		KeycloakSession session,
		UserModel user,
		AuthenticationSessionModel authSession
	) throws IOException, EmailException
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
			EmailTemplateProvider emailTemplateProvider =
				session.getProvider(EmailTemplateProvider.class);
			RealmModel realm = authSession.getRealm();
			String realmName = getRealmName(realm);
			List<Object> subjAttr = ImmutableList.of(realmName);
			Map<String, Object> bodyAttr = Maps.newHashMap();
			bodyAttr.put("realmName", realmName);
			bodyAttr.put("code", code);
			bodyAttr.put("ttl", Math.floorDiv(ttl, 60));
			emailTemplateProvider
				.setRealm(realm)
				.setUser(user)
				.setAttribute("realmName", realmName)
				.send(
					Utils.SEND_CODE_EMAIL_SUBJECT,
					subjAttr, 
					Utils.SEND_CODE_EMAIL_FTL,
					bodyAttr
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


    public static String getRealmName(RealmModel realm)
    {
        return Strings.isNullOrEmpty(realm.getDisplayName())
            ? realm.getName()
            : realm.getDisplayName();
    }

}
