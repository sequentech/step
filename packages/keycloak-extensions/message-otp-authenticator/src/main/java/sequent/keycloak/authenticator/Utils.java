package sequent.keycloak.authenticator;

import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.RealmModel;

import lombok.experimental.UtilityClass;
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
