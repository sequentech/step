// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.adyen;

import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.RealmModel;

import lombok.experimental.UtilityClass;
import lombok.extern.jbosslog.JBossLog;

import java.util.Optional;

@UtilityClass
@JBossLog
public class Utils {
	final public String USER_STATUS_ATTRIBUTE = "user-status-attr";
	final public String API_KEY_ATTRIBUTE = "api-key";
	final public String MERCHANT_ACCOUNT_ATTRIBUTE = "merchant-account";
	final public String CLIENT_KEY_ATTRIBUTE = "client-key";
	final public String AMOUNT_ATTRIBUTE = "amount";
	final public String ENVIRONMENT_ATTRIBUTE = "environment";
	final public String CURRENCY_ATTRIBUTE = "currency";
    final public String ADYEN_FORM = "adyen-authenticator.ftl";
    final public String ADYEN_ERROR = "adyen-error.ftl";
    final public String SESSION_ID = "session_id";
    final public String SESSION_STATUS = "session_status";
    final public String STATUS_SUCCESS = "SUCCESS";
    final public String STATUS_CREATED = "CREATED";

	Optional<AuthenticatorConfigModel> getConfig(RealmModel realm)
	{
		// Using streams to find the first matching configuration
		// NOTE: We're assuming there's only one instance in this realm of this
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
						.equals(AdyenAuthenticator.PROVIDER_ID)
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
