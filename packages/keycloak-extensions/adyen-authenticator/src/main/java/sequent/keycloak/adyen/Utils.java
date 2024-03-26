// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.adyen;

import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.FormContext;
import org.keycloak.events.Details;
import org.keycloak.events.EventType;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.protocol.oidc.OIDCLoginProtocol;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.userprofile.UserProfile;
import org.keycloak.userprofile.UserProfileContext;
import org.keycloak.userprofile.UserProfileProvider;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import lombok.experimental.UtilityClass;
import lombok.extern.jbosslog.JBossLog;

import java.io.StringWriter;
import java.util.Collection;
import java.util.Collections;
import java.util.List;
import java.util.Optional;
import java.io.Writer;

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
