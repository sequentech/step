// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only


/* 
package sequent.keycloak.inetum_authenticator;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.FormContext;
import org.keycloak.models.AuthenticationSessionModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.userprofile.UserProfile;
import org.keycloak.userprofile.UserProfileContext;
import org.keycloak.userprofile.UserProfileProvider;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;

import java.util.List;
import java.util.Map;

public class UtilsTest {

    private FormContext formContext;
    private AuthenticationFlowContext authFlowContext;
    private AuthenticationSessionModel authSessionModel;
    private KeycloakSession keycloakSession;
    private UserProfileProvider userProfileProvider;
    private UserProfile userProfile;
    private RealmModel realmModel;
    private UserModel userModel;

    @BeforeEach
    public void setup() {
        formContext = mock(FormContext.class);
        authFlowContext = mock(AuthenticationFlowContext.class);
        authSessionModel = mock(AuthenticationSessionModel.class);
        keycloakSession = mock(KeycloakSession.class);
        userProfileProvider = mock(UserProfileProvider.class);
        userProfile = mock(UserProfile.class);
        realmModel = mock(RealmModel.class);
        userModel = mock(UserModel.class);

        when(formContext.getHttpRequest().getDecodedFormParameters()).thenReturn(new MultivaluedHashMap<>());
        when(formContext.getAuthenticationSession()).thenReturn(authSessionModel);
        when(authFlowContext.getSession()).thenReturn(keycloakSession);
        when(authFlowContext.getAuthenticationSession()).thenReturn(authSessionModel);
        when(authFlowContext.getRealm()).thenReturn(realmModel);
        when(keycloakSession.getProvider(UserProfileProvider.class)).thenReturn(userProfileProvider);
        when(userProfileProvider.create(any(UserProfileContext.class), any(MultivaluedMap.class))).thenReturn(userProfile);
        when(userProfile.create()).thenReturn(userModel);
    }

    @Test
    public void testStoreUserDataInAuthSessionNotes() {
        MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
        formData.putSingle("email", "test@example.com");
        formData.putSingle("username", "testuser");

        when(formContext.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        Utils.storeUserDataInAuthSessionNotes(formContext);

        verify(authSessionModel).setAuthNote("keyUserdata", "email;username;");
        verify(authSessionModel).setAuthNote("email", "test@example.com");
        verify(authSessionModel).setAuthNote("username", "testuser");
    }

    @Test
    public void testCreateUserFromAuthSessionNotes() {
        when(authSessionModel.getAuthNote(Utils.KEYS_USERDATA)).thenReturn("email;username;");
        when(authSessionModel.getAuthNote("email")).thenReturn("test@example.com");
        when(authSessionModel.getAuthNote("username")).thenReturn("testuser");
        when(realmModel.isRegistrationEmailAsUsername()).thenReturn(true);

        Utils.createUserFromAuthSessionNotes(authFlowContext);

        verify(userProfile).create();
        verify(userModel).setEnabled(true);
        verify(authFlowContext).setUser(userModel);
        verify(authSessionModel).setClientNote(OIDCLoginProtocol.LOGIN_HINT_PARAM, "test@example.com");
    }

    @Test
    public void testProcessStringTemplate() throws Exception {
        String template = "Hello ${name}!";
        Map<String, Object> data = Map.of("name", "World");

        String result = Utils.processStringTemplate(data, template);
        
        assertEquals("Hello World!", result);
    }

    @Test
    public void testSerializeUserdataKeys() {
        List<String> keys = List.of("email", "username");
        String result = Utils.serializeUserdataKeys(keys);
        assertEquals("email;username;", result);
    }

    @Test
    public void testDeserializeUserdataKeys() {
        String serializedKeys = "email;username;";
        List<String> result = Utils.deserializeUserdataKeys(serializedKeys);
        assertEquals(List.of("email", "username"), result);
    }
}


*/