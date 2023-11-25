package sequent.keycloak.authenticator.forgot_password;

import org.keycloak.email.EmailException;
import org.keycloak.email.EmailTemplateProvider;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;

import com.google.common.base.Strings;
import com.google.common.collect.ImmutableList;
import com.google.common.collect.Maps;

import lombok.extern.jbosslog.JBossLog;

import lombok.experimental.UtilityClass;

import java.util.Optional;
import java.util.List;
import java.util.Map;

@UtilityClass
@JBossLog
public class Utils {
    public final String ATTEMPTED_EMAIL = "ATTEMPTED_EMAIL";
	public final String PASSWORD_CHARS = "passwordChars";
	public final String PASSWORD_CHARS_DEFAULT = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789.-!¡?¿*:;&()=@#$%";
	public final String PASSWORD_LENGTH = "passwordLength";
	public final String PASSWORD_LENGTH_DEFAULT = "12";
	public final String PASSWORD_EXPIRATION_SECONDS = "passwordExpiration";
	public final String PASSWORD_EXPIRATION_SECONDS_DEFAULT = "7200";
	public final String PASSWORD_EXPIRATION_USER_ATTRIBUTE = "passwordExpirationUserAttribute";
	public final String PASSWORD_EXPIRATION_USER_ATTRIBUTE_DEFAULT = "sequent.read-only.expirationDate";
	public final String NEW_PASSWORD_EMAIL_SUBJECT = "newPassword.email.subject";
	public final String NEW_PASSWORD_EMAIL_FTL = "forgot-password-send-new-password.ftl";

	public final String RECAPTCHA_SITE_KEY_ATTRIBUTE = "recaptchaSiteKey";
	public final String RECAPTCHA_SITE_SECRET_ATTRIBUTE = "siteSecret";
	public final String RECAPTCHA_ENABLED = "recaptchaEnabled";
	public final String RECAPTCHA_MIN_SCORE = "recaptchaMinScore";

	public String getString(
		AuthenticatorConfigModel config,
		String configKey
	) {
		return getString(config, configKey, "");
	}

	public String getString(
		AuthenticatorConfigModel config,
		String configKey,
		String defaultValue
	) {
		log.infov(
			"getString(configKey={0}, defaultValue={1})",
			configKey,
			defaultValue
		);
		if (config == null) {
			log.infov("getString(): NULL config={0}", config);
			return defaultValue;
		}

		Map<String, String> mapConfig = config.getConfig();
		if (
			mapConfig == null ||
			!mapConfig.containsKey(configKey) ||
			mapConfig.get(configKey).strip().length() == 0
		) {
			log.infov("getString(): NullOrNotFound mapConfig={0}", mapConfig);
			return defaultValue;
		}
		return mapConfig.get(configKey);
	}

	public int getInt(
		AuthenticatorConfigModel config,
		String configKey,
		String defaultValue
	) {
		log.infov(
			"getInt(configKey={0}, defaultValue={1})",
			configKey,
			defaultValue
		);
		if (config == null) {
			log.infov("getInt(): NULL config={0}", config);
			return Integer.parseInt(defaultValue);
		}

		Map<String, String> mapConfig = config.getConfig();
		if (
			mapConfig == null ||
			!mapConfig.containsKey(configKey) ||
			mapConfig.get(configKey).strip().length() == 0
		) {
			log.infov("getInt(): NullOrNotFound mapConfig={0}", mapConfig);
			return Integer.parseInt(defaultValue);
		}
		return Integer.parseInt(mapConfig.get(configKey));
	}

	public boolean getBoolean(
		AuthenticatorConfigModel config,
		String configKey,
		boolean defaultValue
	) {
		log.infov(
			"getBoolean(configKey={0}, defaultValue={1})",
			configKey,
			defaultValue
		);
		if (config == null) {
			log.infov("getBoolean(): NULL config={0}", config);
			return defaultValue;
		}

		Map<String, String> mapConfig = config.getConfig();
		if (
			mapConfig == null ||
			!mapConfig.containsKey(configKey) ||
			mapConfig.get(configKey).strip().length() == 0
		) {
			log.infov("getBoolean(): NullOrNotFound mapConfig={0}", mapConfig);
			return defaultValue;
		}
		return Boolean.parseBoolean(mapConfig.get(configKey));
	}

	int getPasswordLength(AuthenticatorConfigModel config)
	{
		return getInt(
			config,
			Utils.PASSWORD_LENGTH,
			Utils.PASSWORD_LENGTH_DEFAULT
		);
	}

	String getPasswordChars(AuthenticatorConfigModel config)
	{
		return getString(
			config,
			Utils.PASSWORD_CHARS,
			Utils.PASSWORD_CHARS_DEFAULT
		);
	}

	int getPasswordExpirationSeconds(AuthenticatorConfigModel config)
	{
		return getInt(
			config,
			Utils.PASSWORD_EXPIRATION_SECONDS,
			Utils.PASSWORD_EXPIRATION_SECONDS_DEFAULT
		);
	}

	String getPasswordExpirationUserAttribute(
		AuthenticatorConfigModel config
	) {
		return getString(
			config,
			Utils.PASSWORD_EXPIRATION_USER_ATTRIBUTE,
			Utils.PASSWORD_EXPIRATION_USER_ATTRIBUTE_DEFAULT
		);
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
						.equals(ChooseUser.PROVIDER_ID)
				);
				return ret;
			})
			.map(model ->
				realm.getAuthenticatorConfigById(model.getAuthenticatorConfig())
			)
			.findFirst();
		return configOptional;
	}

    public static String getRealmName(RealmModel realm)
    {
        return Strings.isNullOrEmpty(realm.getDisplayName())
            ? realm.getName()
            : realm.getDisplayName();
    }

    public static void sendNewPasswordNotification(
        KeycloakSession session,
        UserModel user,
        String temporaryPassword
    ) throws EmailException {
		log.infov(
			"sendNewPasswordNotification(): to user with email={0}",
			user.getEmail()
		);
        RealmModel realm = session.getContext().getRealm();
		EmailTemplateProvider emailTemplateProvider =
			session.getProvider(EmailTemplateProvider.class);
		String realmName = getRealmName(realm);
		List<Object> subjAttr = ImmutableList.of(realmName);
		Map<String, Object> bodyAttr = Maps.newHashMap();
		bodyAttr.put("realmName", realmName);
		bodyAttr.put("temporaryPassword", temporaryPassword);
		emailTemplateProvider
			.setRealm(realm)
			.setUser(user)
			.setAttribute("realmName", realmName)
			.send(
				Utils.NEW_PASSWORD_EMAIL_SUBJECT,
				subjAttr,
				Utils.NEW_PASSWORD_EMAIL_FTL,
				bodyAttr
			);
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
