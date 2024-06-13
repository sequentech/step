 
package sequent.keycloak.inetum_authenticator;


/*   CODE 1 --------------------------------
*          A. BUILD SUCCESS - Only if single UNIT Test File
*          B. BUILD FAILED - if another Unit Test File exists  
* /


import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.authentication.AuthenticationFlowException;
import org.keycloak.authentication.ValidationContext;
import org.keycloak.events.Errors;
import org.keycloak.events.EventType;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.*;
import org.keycloak.models.utils.FormMessage;
import org.mockito.InjectMocks;
import org.mockito.Mock;
import org.mockito.junit.jupiter.MockitoExtension;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;

import java.util.ArrayList;
import java.util.List;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.mockito.Mockito.*;

@ExtendWith(MockitoExtension.class)
public class DeferredRegistrationUserCreationTest {

    @Mock
    private KeycloakSession session;

    @Mock
    private RealmModel realm;

    @Mock
    private UserModel user;

    @Mock
    private ValidationContext validationContext;

    @Mock
    private LoginFormsProvider loginFormsProvider;

    @InjectMocks
    private DeferredRegistrationUserCreation formAction;

    @BeforeEach
    public void setUp() {
        formAction = new DeferredRegistrationUserCreation();
    }

    @Test
    public void testValidate_Success() {
        MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
        formData.add("email", "test@example.com");
        formData.add("username", "testuser");
        formData.add("firstName", "Test");
        formData.add("lastName", "User");
        when(validationContext.getRealm()).thenReturn(realm);
        when(validationContext.getEvent()).thenReturn(mock(Event.class));
        when(session.getProvider(UserProfileProvider.class)).thenReturn(mock(UserProfileProvider.class));
        when(session.getProvider(PasswordPolicyManagerProvider.class)).thenReturn(mock(PasswordPolicyManagerProvider.class));
        when(validationContext.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        formAction.validate(validationContext);

        verify(validationContext).success();
        verify(session.getProvider(UserProfileProvider.class)).create(any(), eq(formData));
    }

    @Test
    public void testValidate_InvalidEmail() {
        MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
        formData.add("email", "invalidemail");
        when(validationContext.getRealm()).thenReturn(realm);
        when(validationContext.getEvent()).thenReturn(mock(Event.class));
        when(session.getProvider(UserProfileProvider.class)).thenReturn(mock(UserProfileProvider.class));
        when(validationContext.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        formAction.validate(validationContext);

        verify(validationContext).error(Errors.INVALID_EMAIL);
        verify(validationContext).validationError(eq(formData), any(List.class));
    }

    @Test
    public void testValidate_PasswordMismatch() {
        MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
        formData.add("email", "test@example.com");
        formData.add("username", "testuser");
        formData.add("firstName", "Test");
        formData.add("lastName", "User");
        formData.add("password", "password123");
        formData.add("password-confirm", "password456");
        when(validationContext.getRealm()).thenReturn(realm);
        when(validationContext.getEvent()).thenReturn(mock(Event.class));
        when(session.getProvider(UserProfileProvider.class)).thenReturn(mock(UserProfileProvider.class));
        when(session.getProvider(PasswordPolicyManagerProvider.class)).thenReturn(mock(PasswordPolicyManagerProvider.class));
        when(validationContext.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        formAction.validate(validationContext);

        verify(validationContext).error(Errors.INVALID_REGISTRATION);
        verify(validationContext).validationError(eq(formData), any(List.class));
    }

    @Test
    public void testValidate_BlankPassword() {
        MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
        formData.add("email", "test@example.com");
        formData.add("username", "testuser");
        formData.add("firstName", "Test");
        formData.add("lastName", "User");
        formData.add("password", "");
        when(validationContext.getRealm()).thenReturn(realm);
        when(validationContext.getEvent()).thenReturn(mock(Event.class));
        when(session.getProvider(UserProfileProvider.class)).thenReturn(mock(UserProfileProvider.class));
        when(session.getProvider(PasswordPolicyManagerProvider.class)).thenReturn(mock(PasswordPolicyManagerProvider.class));
        when(validationContext.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        formAction.validate(validationContext);

        verify(validationContext).error(Errors.INVALID_REGISTRATION);
        verify(validationContext).validationError(eq(formData), any(List.class));
    }

    @Test
    public void testSuccess() {
        formAction.success(validationContext);

        verify(validationContext).success();
    }
}
*/  
 

/*    CODE 2 --------------------------------------------------------------------------------
*       Combined Unit Test Approach 
*          A. BUILD FAILED - Separate Combination File, calling 2 separate Unit Test Files
* /

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.authentication.AuthenticationFlowException;
import org.keycloak.authentication.ValidationContext;
import org.keycloak.events.Errors;
import org.keycloak.events.EventType;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.*;
import org.mockito.InjectMocks;
import org.mockito.Mock;
import org.mockito.junit.jupiter.MockitoExtension;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;

import java.util.List;

import static org.mockito.Mockito.*;

@ExtendWith(MockitoExtension.class)
public class DeferredRegistrationUserCreationTest {

    @Mock
    private KeycloakSession session;

    @Mock
    private RealmModel realm;

    @Mock
    private UserModel user;

    @Mock
    private ValidationContext validationContext;

    @Mock
    private LoginFormsProvider loginFormsProvider;

    @InjectMocks
    private DeferredRegistrationUserCreation formAction;

    @BeforeEach
    public void setUp() {
        formAction = new DeferredRegistrationUserCreation();
    }

    @Test
    public void testValidate_Success() {
        MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
        formData.add("email", "test@example.com");
        formData.add("username", "testuser");
        formData.add("firstName", "Test");
        formData.add("lastName", "User");
        when(validationContext.getRealm()).thenReturn(realm);
        when(validationContext.getEvent()).thenReturn(mock(Event.class));
        when(session.getProvider(UserProfileProvider.class)).thenReturn(mock(UserProfileProvider.class));
        when(session.getProvider(PasswordPolicyManagerProvider.class)).thenReturn(mock(PasswordPolicyManagerProvider.class));
        when(validationContext.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        formAction.validate(validationContext);

        verify(validationContext).success();
        verify(session.getProvider(UserProfileProvider.class)).create(any(), eq(formData));
    }

    @Test
    public void testValidate_InvalidEmail() {
        MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
        formData.add("email", "invalidemail");
        when(validationContext.getRealm()).thenReturn(realm);
        when(validationContext.getEvent()).thenReturn(mock(Event.class));
        when(session.getProvider(UserProfileProvider.class)).thenReturn(mock(UserProfileProvider.class));
        when(validationContext.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        formAction.validate(validationContext);

        verify(validationContext).error(Errors.INVALID_EMAIL);
        verify(validationContext).validationError(eq(formData), any(List.class));
    }

    @Test
    public void testValidate_PasswordMismatch() {
        MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
        formData.add("email", "test@example.com");
        formData.add("username", "testuser");
        formData.add("firstName", "Test");
        formData.add("lastName", "User");
        formData.add("password", "password123");
        formData.add("password-confirm", "password456");
        when(validationContext.getRealm()).thenReturn(realm);
        when(validationContext.getEvent()).thenReturn(mock(Event.class));
        when(session.getProvider(UserProfileProvider.class)).thenReturn(mock(UserProfileProvider.class));
        when(session.getProvider(PasswordPolicyManagerProvider.class)).thenReturn(mock(PasswordPolicyManagerProvider.class));
        when(validationContext.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        formAction.validate(validationContext);

        verify(validationContext).error(Errors.INVALID_REGISTRATION);
        verify(validationContext).validationError(eq(formData), any(List.class));
    }

    @Test
    public void testValidate_BlankPassword() {
        MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
        formData.add("email", "test@example.com");
        formData.add("username", "testuser");
        formData.add("firstName", "Test");
        formData.add("lastName", "User");
        formData.add("password", "");
        when(validationContext.getRealm()).thenReturn(realm);
        when(validationContext.getEvent()).thenReturn(mock(Event.class));
        when(session.getProvider(UserProfileProvider.class)).thenReturn(mock(UserProfileProvider.class));
        when(session.getProvider(PasswordPolicyManagerProvider.class)).thenReturn(mock(PasswordPolicyManagerProvider.class));
        when(validationContext.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        formAction.validate(validationContext);

        verify(validationContext).error(Errors.INVALID_REGISTRATION);
        verify(validationContext).validationError(eq(formData), any(List.class));
    }

    @Test
    public void testSuccess() {
        formAction.success(validationContext);

        verify(validationContext).success();
    }
}
*/


 /*    CODE 3 --------------------------------------------------------------------------------
*         Combined Unit Test Approach  - CLASSPATH Issues
*          A. BUILD SUCCESS - Single Combination Test File, Integrating Multiple Tests Test Files
*/

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.FormContext;
import org.keycloak.authentication.ValidationContext;
import org.keycloak.events.Errors;
import org.keycloak.models.*;
import org.keycloak.policy.PasswordPolicyManagerProvider;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.userprofile.UserProfile;
import org.keycloak.userprofile.UserProfileProvider;
import org.mockito.InjectMocks;
import org.mockito.Mock;
import org.mockito.junit.jupiter.MockitoExtension;
import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;

import java.util.Collections;
import java.util.List;

import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.*;
import static org.junit.jupiter.api.Assertions.*;

@ExtendWith(MockitoExtension.class)
public class CombinedTests {

    // Mocks and InjectMocks for DeferredRegistrationUserCreation
    @Mock
    private KeycloakSession session;

    @Mock
    private RealmModel realm;

    @Mock
    private ValidationContext validationContext;

    @InjectMocks
    private DeferredRegistrationUserCreation formAction;

    // Mocks for InetumAuthenticator (no @InjectMocks as it's a factory)
    @Mock
    private AuthenticationFlowContext authenticationFlowContext;

    @Mock
    private UserModel userModel;

    @BeforeEach
    public void setUp() {
        formAction = new DeferredRegistrationUserCreation();
    }

    // Tests for DeferredRegistrationUserCreation.java

    @Test
    public void testValidate_Success() {
        MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
        formData.add("email", "test@example.com");
        formData.add("username", "testuser");
        formData.add("firstName", "Test");
        formData.add("lastName", "User");

        when(validationContext.getRealm()).thenReturn(realm);
        when(validationContext.getEvent()).thenReturn(mock(Event.class));
        when(session.getProvider(UserProfileProvider.class)).thenReturn(mock(UserProfileProvider.class));
        when(session.getProvider(PasswordPolicyManagerProvider.class)).thenReturn(mock(PasswordPolicyManagerProvider.class));
        when(validationContext.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        formAction.validate(validationContext);

        verify(validationContext).success();
        verify(session.getProvider(UserProfileProvider.class)).create(any(), eq(formData));
    }

    @Test
    public void testValidate_InvalidEmail() {
        MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
        formData.add("email", "invalidemail");

        when(validationContext.getRealm()).thenReturn(realm);
        when(validationContext.getEvent()).thenReturn(mock(Event.class));
        when(session.getProvider(UserProfileProvider.class)).thenReturn(mock(UserProfileProvider.class));
        when(validationContext.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        formAction.validate(validationContext);

        verify(validationContext).error(Errors.INVALID_EMAIL);
        verify(validationContext).validationError(eq(formData), any(List.class));
    }

    // Tests for InetumAuthenticator.java

    @Test
    public void testInetumAuthenticatorAuthenticate_Success() {
        // Mock configuration and setup for InetumAuthenticator
        AuthenticatorConfigModel config = mock(AuthenticatorConfigModel.class);
        when(authenticationFlowContext.getAuthenticatorConfig()).thenReturn(config);
        when(authenticationFlowContext.getUser()).thenReturn(userModel);

        // Mock user status attribute
        when(userModel.getFirstAttribute(anyString())).thenReturn("TRUE");

        // Invoke authenticate method
        InetumAuthenticator inetumAuthenticator = new InetumAuthenticator();
        inetumAuthenticator.authenticate(authenticationFlowContext);

        // Verify that context.success() is called
        verify(authenticationFlowContext).success();
    }

    @Test
    public void testInetumAuthenticatorAuthenticate_Failure() {
        // Mock configuration and setup for InetumAuthenticator
        AuthenticatorConfigModel config = mock(AuthenticatorConfigModel.class);
        when(authenticationFlowContext.getAuthenticatorConfig()).thenReturn(config);
        when(authenticationFlowContext.getUser()).thenReturn(userModel);

        // Mock user status attribute
        when(userModel.getFirstAttribute(anyString())).thenReturn(null);

        // Invoke authenticate method
        InetumAuthenticator inetumAuthenticator = new InetumAuthenticator();
        inetumAuthenticator.authenticate(authenticationFlowContext);

        // Verify that context.challenge() is called
        verify(authenticationFlowContext).challenge(any());
    }

    // Tests for LookupAndUpdateUser.java

    @Test
    public void testLookupAndUpdateUser_Authenticate_Success() {
        AuthenticationSessionModel authenticationSession = mock(AuthenticationSessionModel.class);
        when(authenticationFlowContext.getAuthenticationSession()).thenReturn(authenticationSession);
        when(authenticationFlowContext.getSession()).thenReturn(session);
        when(authenticationFlowContext.getRealm()).thenReturn(realm);

        MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
        formData.add("username", "testuser");
        when(authenticationSession.getAuthNote(anyString())).thenReturn("testuser");
        when(authenticationSession.getAuthNoteNames()).thenReturn(Collections.singletonList("username"));
        when(session.users().searchForUserByUserAttributeStream(eq(realm), anyString(), anyString())).thenReturn(java.util.stream.Stream.of(userModel));

        LookupAndUpdateUser lookupAndUpdateUser = new LookupAndUpdateUser();
        lookupAndUpdateUser.authenticate(authenticationFlowContext);

        verify(authenticationFlowContext).success();
    }

    // Tests for Utils.java 

    @Test
    public void testUtils_StoreUserDataInAuthSessionNotes() {
        FormContext formContext = mock(FormContext.class);
        MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
        formData.add("email", "test@example.com");
        formData.add("username", "testuser");
        when(formContext.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        AuthenticationSessionModel sessionModel = mock(AuthenticationSessionModel.class);
        when(formContext.getAuthenticationSession()).thenReturn(sessionModel);

        Utils.storeUserDataInAuthSessionNotes(formContext);

        verify(sessionModel).setAuthNote(Utils.KEYS_USERDATA, Utils.serializeUserdataKeys(formData.keySet()));
        formData.forEach((key, value) -> {
            verify(sessionModel).setAuthNote(key, formData.getFirst(key));
        });
    }

    @Test
    public void testUtils_CreateUserFromAuthSessionNotes() {
        AuthenticationFlowContext context = mock(AuthenticationFlowContext.class);
        MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
        formData.add(UserModel.EMAIL, "test@example.com");
        formData.add(UserModel.USERNAME, "testuser");
        when(context.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        AuthenticationSessionModel authenticationSession = mock(AuthenticationSessionModel.class);
        when(context.getAuthenticationSession()).thenReturn(authenticationSession);
        when(context.getRealm()).thenReturn(realm);
        UserProfileProvider profileProvider = mock(UserProfileProvider.class);
        when(session.getProvider(UserProfileProvider.class)).thenReturn(profileProvider);
        UserProfile profile = mock(UserProfile.class);
        when(profileProvider.create(any())).thenReturn(profile);
        UserModel user = mock(UserModel.class);
        when(profile.create()).thenReturn(user);

        Utils.createUserFromAuthSessionNotes(context);

        verify(user).setEnabled(true);
        verify(context).setUser(user);
    }

}
 