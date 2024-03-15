// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2021,2022 Inventage AG
//
// SPDX-License-Identifier: AGPL-3.0-only

// Partially based in: https://github.dev/inventage/keycloak-custom/tree/tutorial-passkey/extensions/extension-passkey/src/main/java/com/inventage/keycloak/registration/Utils.java

package sequent.keycloak.inetum_authenticator;

import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.FormContext;
import org.keycloak.authentication.forms.RegistrationPage;
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

import java.util.Collection;
import java.util.Collections;
import java.util.List;
import java.util.Optional;

@UtilityClass
@JBossLog
public class Utils {
	final public String USER_DATA_ATTRIBUTE = "user-data-attr";
	final public String USER_STATUS_ATTRIBUTE = "user-status-attr";
	final public String SDK_ATTRIBUTE = "sdk";
	final public String API_KEY_ATTRIBUTE = "api-key";
	final public String APP_ID_ATTRIBUTE = "app-id";
	final public String CLIENT_ID_ATTRIBUTE = "client-id";
	final public String ENV_CONFIG_ATTRIBUTE = "env-config";
    final public String INETUM_FORM = "inetum-authenticator.ftl";

    private static final String KEYS_USERDATA = "keyUserdata";
    private static final String UP_REGISTER_ATTRIBUTE = "UP_REGISTER";
    private static final String KEYS_USERDATA_SEPARATOR = ";";
    private static final List<String> DEFAULT_KEYS_USERDATA = List.of(UserModel.FIRST_NAME, UserModel.LAST_NAME, UserModel.EMAIL, UserModel.USERNAME);

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
						.equals(InetumAuthenticatorFactory.PROVIDER_ID)
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
     * We store the user data entered in the registration form in the session notes.
     * This information will later be retrieved to create a user account.
     */
    static void storeUserDataInAuthSessionNotes(FormContext context)
	{
		log.info("storeUserDataInAuthSessionNotes: start");
        MultivaluedMap<String, String> formData = context
			.getHttpRequest()
			.getDecodedFormParameters();
        AuthenticationSessionModel sessionModel = context.getAuthenticationSession();

        // We store each key
        String keys = Utils.serializeUserdataKeys(formData.keySet());

		log.info("storeUserDataInAuthSessionNotes: setAuthNote(" + Utils.KEYS_USERDATA + ", " + keys + ")");
        sessionModel.setAuthNote(Utils.KEYS_USERDATA, keys);

        formData.forEach((key, value) -> {
			log.info("storeUserDataInAuthSessionNotes: setAuthNote(" + key + ", " + formData.getFirst(key) + ")");
            sessionModel.setAuthNote(key, formData.getFirst(key));
        });
    }

    /**
     * We retrieve the user data stored in the session notes and create a new user in this realm.
     */
    static void createUserFromAuthSessionNotes(AuthenticationFlowContext context) {
        MultivaluedMap<String, String> formData = context
			.getHttpRequest()
			.getDecodedFormParameters();
        MultivaluedMap<String, String> userAttributes = new MultivaluedHashMap<>();

        AuthenticationSessionModel authenticationSession = context
			.getAuthenticationSession();
        List<String> keysUserdata = Utils
			.deserializeUserdataKeys(
				authenticationSession.getAuthNote(Utils.KEYS_USERDATA)
			);

        //keys userdata is transmitted from the UserCreationPasskeyAction class.
        if (keysUserdata != null) {
            for (String key : keysUserdata) {
                String value = authenticationSession.getAuthNote(key);
                if (value != null) {
                    userAttributes.add(key, value);
                }
            }
        } // In case that another custom FormAction than UserCreationPasskey is used.
        else {
            for (String key : DEFAULT_KEYS_USERDATA) {
                String value = authenticationSession.getAuthNote(key);
                if (value != null) {
                    userAttributes.add(key, value);
                }
            }
        }

        String email = formData.getFirst(UserModel.EMAIL);
        String username = formData.getFirst(UserModel.USERNAME);

        if (context.getRealm().isRegistrationEmailAsUsername()) {
            username = email;
        }

        context
			.getEvent()
			.detail(Details.USERNAME, username)
			.detail(Details.REGISTER_METHOD, "form")
			.detail(Details.EMAIL, email);

        KeycloakSession session = context.getSession();

        UserProfileProvider profileProvider = session
			.getProvider(UserProfileProvider.class);
        UserProfile profile = profileProvider
			.create(UserProfileContext.REGISTRATION, userAttributes);
        UserModel user = profile.create();

        user.setEnabled(true);
        context.setUser(user);

        context
			.getAuthenticationSession()
			.setClientNote(OIDCLoginProtocol.LOGIN_HINT_PARAM, username);

        context.getEvent().user(user);
        context.getEvent().success();
        context.newEvent().event(EventType.LOGIN);
        context.getEvent().client(context.getAuthenticationSession().getClient().getClientId())
                .detail(Details.REDIRECT_URI, context.getAuthenticationSession().getRedirectUri())
                .detail(Details.AUTH_METHOD, context.getAuthenticationSession().getProtocol());
        String authType = context.getAuthenticationSession().getAuthNote(Details.AUTH_TYPE);
        if (authType != null) {
            context.getEvent().detail(Details.AUTH_TYPE, authType);
        }
    }

    private static String serializeUserdataKeys(Collection<String> keys, String separator) {
        final StringBuilder key = new StringBuilder();
        keys.forEach((s -> key.append(s + separator)));
        return key.toString();
    }

    private static String serializeUserdataKeys(Collection<String> keys) {
        return serializeUserdataKeys(keys, KEYS_USERDATA_SEPARATOR);
    }

    private static List<String> deserializeUserdataKeys(String key, String separator) {
        if (key == null) {
            return Collections.emptyList();
        }
        return List.of(key.split(separator));
    }

    private static List<String> deserializeUserdataKeys(String key) {
        return deserializeUserdataKeys(key, KEYS_USERDATA_SEPARATOR);
    }

}
