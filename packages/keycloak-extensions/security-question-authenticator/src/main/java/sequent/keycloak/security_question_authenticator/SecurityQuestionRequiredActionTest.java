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
import org.keycloak.authentication.RequiredActionContext;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;

import java.util.HashMap;
import java.util.Map;

public class SecurityQuestionRequiredActionTest {

    private SecurityQuestionRequiredAction requiredAction;
    private RequiredActionContext context;
    private AuthenticatorConfigModel config;
    private UserModel user;

    @BeforeEach
    public void setUp() {
        requiredAction = new SecurityQuestionRequiredAction();
        context = mock(RequiredActionContext.class);
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
    public void testEvaluateTriggers() {
        when(context.getAuthenticationSession().getAuthNote(Utils.ALREADY_EXECUTED_SECURITY_QUESTION)).thenReturn(null);
        requiredAction.evaluateTriggers(context);
        verify(context.getAuthenticationSession()).addRequiredAction(SecurityQuestionRequiredAction.PROVIDER_ID);
    }

    @Test
    public void testRequiredActionChallenge() {
        requiredAction.requiredActionChallenge(context);
        verify(context).challenge(any(Response.class));
    }

    @Test
    public void testProcessAction_ValidAnswer() {
        MultivaluedMap<String, String> formData = mock(MultivaluedMap.class);
        when(formData.getFirst(Utils.FORM_SECURITY_ANSWER_FIELD)).thenReturn("123");
        when(context.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        requiredAction.processAction(context);
        verify(context).success();
    }

    @Test
    public void testProcessAction_InvalidAnswer() {
        MultivaluedMap<String, String> formData = mock(MultivaluedMap.class);
        when(formData.getFirst(Utils.FORM_SECURITY_ANSWER_FIELD)).thenReturn("456");
        when(context.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        requiredAction.processAction(context);
        verify(context).challenge(any(Response.class));
    }
}
*/