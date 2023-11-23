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
	public final String EMAIL_USER_ATTRIBUTE = "emailUserAttribute";
	public final String SIMULATION_MODE = "simulationMode";
	public final boolean SIMULATION_MODE_DEFAULT = true;
    public final String EMAIL_ADDRESS_FIELD = "sequent.read-only.email-address";
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

	String getEmail(AuthenticatorConfigModel config, UserModel user)
	{
		log.infov("getEmail()");
		if (config == null) {
			log.infov("getEmail(): NULL config={0}", config);
			return user.getFirstAttribute(
				Utils.EMAIL_ADDRESS_FIELD
			);
		}

		Map<String, String> mapConfig = config.getConfig();
		if (
			mapConfig == null ||
			!mapConfig.containsKey(Utils.EMAIL_USER_ATTRIBUTE)
		) {
			log.infov("getEmail(): NullOrNotFound mapConfig={0}", mapConfig);
			return user.getFirstAttribute(
				Utils.EMAIL_ADDRESS_FIELD
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

	int getPasswordLength(AuthenticatorConfigModel config, UserModel user)
	{
		log.infov("getPasswordLength()");
		if (config == null) {
			log.infov("getPasswordLength(): NULL config={0}", config);
			return Integer.parseInt(Utils.PASSWORD_LENGTH_DEFAULT);
		}

		Map<String, String> mapConfig = config.getConfig();
		if (
			mapConfig == null ||
			!mapConfig.containsKey(Utils.PASSWORD_LENGTH) ||
			mapConfig.get(Utils.PASSWORD_LENGTH).strip().length() == 0
		) {
			log.infov("getPasswordLength(): NullOrNotFound mapConfig={0}", mapConfig);
			return Integer.parseInt(Utils.PASSWORD_LENGTH_DEFAULT);
		}
		return Integer.parseInt(mapConfig.get(Utils.PASSWORD_LENGTH));
	}

	String getPasswordChars(AuthenticatorConfigModel config, UserModel user)
	{
		log.infov("getPasswordChars()");
		if (config == null) {
			log.infov("getPasswordChars(): NULL config={0}", config);
			return Utils.PASSWORD_CHARS_DEFAULT;
		}

		Map<String, String> mapConfig = config.getConfig();
		if (
			mapConfig == null ||
			!mapConfig.containsKey(Utils.PASSWORD_CHARS) ||
			mapConfig.get(Utils.PASSWORD_CHARS).strip().length() == 0
		) {
			log.infov("getPasswordChars(): NullOrNotFound mapConfig={0}", mapConfig);
			return Utils.PASSWORD_CHARS_DEFAULT;
		}
		return mapConfig.get(Utils.PASSWORD_CHARS);
	}

	int getPasswordExpirationSeconds(AuthenticatorConfigModel config, UserModel user)
	{
		log.infov("getPasswordExpiration()");
		if (config == null) {
			log.infov("getPasswordExpiration(): NULL config={0}", config);
			return Integer.parseInt(Utils.PASSWORD_EXPIRATION_SECONDS_DEFAULT);
		}

		Map<String, String> mapConfig = config.getConfig();
		if (
			mapConfig == null ||
			!mapConfig.containsKey(Utils.PASSWORD_EXPIRATION_SECONDS) ||
			mapConfig.get(Utils.PASSWORD_EXPIRATION_SECONDS).strip().length() == 0
		) {
			log.infov("getPasswordExpiration(): NullOrNotFound mapConfig={0}", mapConfig);
			return Integer.parseInt(Utils.PASSWORD_EXPIRATION_SECONDS_DEFAULT);
		}
		return Integer.parseInt(mapConfig.get(Utils.PASSWORD_EXPIRATION_SECONDS));
	}

	String getPasswordExpirationUserAttribute(
		AuthenticatorConfigModel config, UserModel user
	) {
		log.infov("getPasswordExpirationUserAttribute()");
		if (config == null) {
			log.infov("getPasswordExpirationUserAttribute(): NULL config={0}", config);
			return Utils.PASSWORD_EXPIRATION_USER_ATTRIBUTE_DEFAULT;
		}

		Map<String, String> mapConfig = config.getConfig();
		if (
			mapConfig == null ||
			!mapConfig.containsKey(Utils.PASSWORD_EXPIRATION_USER_ATTRIBUTE) ||
			mapConfig.get(Utils.PASSWORD_EXPIRATION_USER_ATTRIBUTE).strip().length() == 0
		) {
			log.infov("getPasswordExpirationUserAttribute(): NullOrNotFound mapConfig={0}", mapConfig);
			return Utils.PASSWORD_EXPIRATION_USER_ATTRIBUTE_DEFAULT;
		}
		return mapConfig.get(Utils.PASSWORD_EXPIRATION_USER_ATTRIBUTE);
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
        String temporaryPassword,
		boolean simulationMode
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
