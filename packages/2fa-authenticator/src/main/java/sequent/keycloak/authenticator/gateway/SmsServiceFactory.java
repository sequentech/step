package sequent.keycloak.authenticator.gateway;

import sequent.keycloak.authenticator.OTPConstants;
import lombok.extern.slf4j.Slf4j;

import java.util.Map;

@Slf4j
public class SmsServiceFactory {
	public static SmsService get(Map<String, String> config) {
		if (Boolean
			.parseBoolean(
				config.getOrDefault(OTPConstants.SIMULATION_MODE, "false")
			)
		) {
			return (phoneNumber, message) ->
				log.warn(
					String.format(
						"***** SIMULATION MODE ***** Would send the following message: phoneNumber=`%s`, message=`%s`",
						phoneNumber,
						message
					)
				);
		} else {
			return new AwsSmsService(config);
		}
	}

}
