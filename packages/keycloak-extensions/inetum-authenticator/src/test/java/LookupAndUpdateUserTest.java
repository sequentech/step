
// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.inetum_authenticator;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;

import java.util.HashMap;
import java.util.List;
import java.util.Map;

import static org.mockito.Mockito.*;

class LookupAndUpdateUserTest {

    private LookupAndUpdateUser authenticator;
    private AuthenticationFlowContext context;
    private AuthenticatorConfigModel configModel;
    private KeycloakSession session;
    private RealmModel realm;
    private AuthenticationSessionModel authSession;
    private UserModel user;

    @BeforeEach
    void setUp() {
        authenticator = new LookupAndUpdateUser();
        context = mock(AuthenticationFlowContext.class);
        configModel = mock(AuthenticatorConfigModel.class);
        session = mock(KeycloakSession.class);
        realm = mock(RealmModel.class);
        authSession = mock(AuthenticationSessionModel.class);
        user = mock(UserModel.class);

        when(context.getAuthenticatorConfig()).thenReturn(configModel);
        when(context.getSession()).thenReturn(session);
        when(context.getRealm()).thenReturn(realm);
        when(context.getAuthenticationSession()).thenReturn(authSession);
    }

    @Test
    void testAuthenticate_UserNotFound() {
        Map<String, String> configMap = new HashMap<>();
        configMap.put(LookupAndUpdateUser.SEARCH_ATTRIBUTES, "email");
        when(configModel.getConfig()).thenReturn(configMap);
        when(authSession.getAuthNote("email")).thenReturn("nonexistent@example.com");
        when(session.users().searchForUserByUserAttributeStream(realm, "email", "nonexistent@example.com"))
            .thenReturn(Stream.empty());

        authenticator.authenticate(context);

        verify(context).attempted();
    }

    @Test
    void testAuthenticate_UserFoundButHasCredentials() {
        Map<String, String> configMap = new HashMap<>();
        configMap.put(LookupAndUpdateUser.SEARCH_ATTRIBUTES, "email");
        when(configModel.getConfig()).thenReturn(configMap);
        when(authSession.getAuthNote("email")).thenReturn("existent@example.com");
        when(session.users().searchForUserByUserAttributeStream(realm, "email", "existent@example.com"))
            .thenReturn(Stream.of(user));
        when(user.credentialManager().getStoredCredentialsStream()).thenReturn(Stream.of(mock(UserCredentialModel.class)));

        authenticator.authenticate(context);

        verify(context).attempted();
    }

    @Test
    void testAuthenticate_UserFoundAndNoCredentials() {
        Map<String, String> configMap = new HashMap<>();
        configMap.put(LookupAndUpdateUser.SEARCH_ATTRIBUTES, "email");
        configMap.put(LookupAndUpdateUser.UNSET_ATTRIBUTES, "phoneNumber");
        configMap.put(LookupAndUpdateUser.UPDATE_ATTRIBUTES, "username,email");
        when(configModel.getConfig()).thenReturn(configMap);
        when(authSession.getAuthNote("email")).thenReturn("existent@example.com");
        when(authSession.getAuthNote("username")).thenReturn("newusername");
        when(session.users().searchForUserByUserAttributeStream(realm, "email", "existent@example.com"))
            .thenReturn(Stream.of(user));
        when(user.credentialManager().getStoredCredentialsStream()).thenReturn(Stream.empty());
        when(user.getAttributes()).thenReturn(Map.of());

        authenticator.authenticate(context);

        verify(user).setUsername("newusername");
        verify(user).setEmail("existent@example.com");
        verify(context).setUser(user);
        verify(context).success();
    }
}
