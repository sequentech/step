package sequent.keycloak.inetum_authenticator;

import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.RealmModel;
import lombok.experimental.UtilityClass;
import java.util.Optional;

@UtilityClass
public class Utils {
	final public String USER_DATA_ATTRIBUTE = "user-data-attr";
	final public String USER_STATUS_ATTRIBUTE = "user-status-attr";
	final public String SDK_ATTRIBUTE = "sdk";
	final public String ENV_CONFIG_ATTRIBUTE = "env-config";
    final public String INETUM_FORM = "inetum-authenticator.ftl";


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
						.equals(SecurityQuestionAuthenticatorFactory.PROVIDER_ID)
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