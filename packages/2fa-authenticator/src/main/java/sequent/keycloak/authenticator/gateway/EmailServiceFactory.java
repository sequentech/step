package sequent.keycloak.authenticator.gateway;

import sequent.keycloak.authenticator.OTPConstants;
import lombok.extern.slf4j.Slf4j;

import java.util.Map;

@Slf4j
public class EmailServiceFactory {
	public static EmailService get(Map<String, String> config) {
		if (Boolean
			.parseBoolean(
				config.getOrDefault(OTPConstants.SIMULATION_MODE, "false")
			)
		) {
			return (emailAddress, title, body, htmlBody) ->
				log.warn(
					String.format(
						"***** SIMULATION MODE ***** Would send the following message: emailAddress=`%s`, title=`%s`, body=`%s`, htmlBody=`%s`",
						emailAddress,
                        title,
                        body,
                        htmlBody
					)
				);
		} else {
			return new SmtpEmailService(config);
		}
	}

}
