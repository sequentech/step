package sequent.keycloak.authenticator.forgot_password;

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
	public final String EMAIL_USER_ATTRIBUTE = "emailUserAttribute";
    public final String EMAIL_ADDRESS_FIELD = "sequent.read-only.email-address";
    public final String ATTEMPTED_EMAIL = "ATTEMPTED_EMAIL";

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
