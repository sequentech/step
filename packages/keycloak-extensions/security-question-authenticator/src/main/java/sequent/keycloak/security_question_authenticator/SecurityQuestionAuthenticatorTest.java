package sequent.keycloak.security_question_authenticator;

/*
 * Unit Testing Security Question Authenticator  JC:Ayeng  6132024
 *
 * NOTE : Commented out so dependencies are installed properly for other Unit Testing Files.
 *         UNCOMMENT IF TESTING FOR THIS FILE ONLY, or dependencies will be needed in all extensions

import static org.mockito.Mockito.*;
import static org.junit.jupiter.api.Assertions.*;

import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.Response;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;

import java.util.HashMap;
import java.util.Map;

public class SecurityQuestionAuthenticatorTest {

    private SecurityQuestionAuthenticator authenticator;
    private AuthenticationFlowContext context;
    private AuthenticatorConfigModel config;
    private UserModel user;

    @BeforeEach
    public void setUp() {
        authenticator = new SecurityQuestionAuthenticator();
        context = mock(AuthenticationFlowContext.class);
        config = mock(AuthenticatorConfigModel.class);
        user = mock(UserModel.class);

        RealmModel realm = mock(RealmModel.class);
        KeycloakSession session = mock(KeycloakSession.class);
        AuthenticationSessionModel sessionModel = mock(AuthenticationSessionModel.class);

        when(context.getRealm()).thenReturn(realm);
        when(context.getSession()).thenReturn(session);
        when(context.getUser()).thenReturn(user);
        when(context.getAuthenticationSession()).thenReturn(sessionModel);

        Map<String, String> configMap = new HashMap<>();
        configMap.put(Utils.NUM_LAST_CHARS, "3");
        configMap.put(Utils.USER_ATTRIBUTE, "securityAnswer");

        when(config.getConfig()).thenReturn(configMap);
        when(user.getFirstAttribute("securityAnswer")).thenReturn("abc123");
    }

    @Test
    public void testAuthenticate() {
        authenticator.authenticate(context);
        verify(context).challenge(any(Response.class));
    }

    @Test
    public void testAction_ValidAnswer() {
        MultivaluedMap<String, String> formData = mock(MultivaluedMap.class);
        when(formData.getFirst(Utils.FORM_SECURITY_ANSWER_FIELD)).thenReturn("123");
        when(context.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        authenticator.action(context);
        verify(context).success();
    }

    @Test
    public void testAction_InvalidAnswer() {
        MultivaluedMap<String, String> formData = mock(MultivaluedMap.class);
        when(formData.getFirst(Utils.FORM_SECURITY_ANSWER_FIELD)).thenReturn("456");
        when(context.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        authenticator.action(context);
        verify(context).failureChallenge(eq(AuthenticationFlowError.INVALID_CREDENTIALS), any(Response.class));
    }
}

