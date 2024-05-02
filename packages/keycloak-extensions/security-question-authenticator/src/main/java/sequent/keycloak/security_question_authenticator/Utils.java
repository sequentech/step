// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.security_question_authenticator;

import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.RealmModel;
import lombok.experimental.UtilityClass;
import java.util.Optional;

@UtilityClass
public class Utils {
	final public String USER_ATTRIBUTE = "user-attribute";
	final public String NUM_LAST_CHARS = "num-last-chars";
	final public String FORM_SECURITY_ANSWER_FIELD = "security-answer";
	final public String ALREADY_EXECUTED_SECURITY_QUESTION = "already-executed-security-question";
    final public String SECURITY_QUESTION_FORM = "security-question.ftl";


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