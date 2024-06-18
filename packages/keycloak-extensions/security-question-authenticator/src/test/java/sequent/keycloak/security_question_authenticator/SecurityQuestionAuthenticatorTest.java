package sequent.keycloak.security_question_authenticator;
/* 
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.http.HttpRequest;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.mockito.InjectMocks;
import org.mockito.Mock;
import org.mockito.junit.jupiter.MockitoExtension;

import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.Response;

import java.util.HashMap;
import java.util.Map;
import java.util.Optional;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.*;

@ExtendWith(MockitoExtension.class)
public class SecurityQuestionAuthenticatorTest {

    @Mock
    private AuthenticationFlowContext context;

    @Mock
    private AuthenticatorConfigModel config;

    @Mock
    private MultivaluedMap<String, String> formData;

    @Mock
    private UserModel user;

    @Mock
    private RealmModel realm;

    @InjectMocks
    private SecurityQuestionAuthenticator authenticator;

    @BeforeEach
    public void setUp() {
        HttpRequest httpRequest = mock(HttpRequest.class);
        when(context.getHttpRequest()).thenReturn(httpRequest);
        when(httpRequest.getDecodedFormParameters()).thenReturn(formData);
        when(context.getUser()).thenReturn(user);
        when(context.getRealm()).thenReturn(realm);
        when(context.getExecution()).thenReturn(mock(AuthenticationExecutionModel.class));

        // Mock the Utils.getConfig to return the mocked config
        //mockStatic(Utils.class);
        
        when(Utils.getConfig(any(RealmModel.class))).thenReturn(Optional.of(config));
    }

    @Test
    public void testAuthenticate() {
        Response response = mock(Response.class);
        LoginFormsProvider formsProvider = mock(LoginFormsProvider.class);
        when(context.form()).thenReturn(formsProvider);
        when(formsProvider.createForm(Utils.SECURITY_QUESTION_FORM)).thenReturn(response);

        // Create a spy for AuthenticatorConfigModel (assuming it's not final)
        AuthenticatorConfigModel configModel = new AuthenticatorConfigModel();
        AuthenticatorConfigModel spyConfig = spy(configModel);

        // Stub any necessary behavior on the spyConfig
        when(context.getAuthenticatorConfig()).thenReturn(spyConfig);
        authenticator.authenticate(context);

        verify(context).challenge(response);
    }

    @Test
    public void testActionInvalidCredentials() {
        when(formData.getFirst(Utils.FORM_SECURITY_ANSWER_FIELD)).thenReturn("wrongAnswer");

        Map<String, String> configMap = new HashMap<>();
        configMap.put(Utils.NUM_LAST_CHARS, "5");
        configMap.put(Utils.USER_ATTRIBUTE, "securityAnswer");
        when(config.getConfig()).thenReturn(configMap);

        when(user.getFirstAttribute("securityAnswer")).thenReturn("correctAnswer");

        authenticator.action(context);

        verify(context).failureChallenge(eq(AuthenticationFlowError.INVALID_CREDENTIALS), any(Response.class));
    }

    @Test
    public void testActionValidCredentials() {
        when(formData.getFirst(Utils.FORM_SECURITY_ANSWER_FIELD)).thenReturn("Answer123");

        Map<String, String> configMap = new HashMap<>();
        configMap.put(Utils.NUM_LAST_CHARS, "3");
        configMap.put(Utils.USER_ATTRIBUTE, "securityAnswer");
        when(config.getConfig()).thenReturn(configMap);

        when(user.getFirstAttribute("securityAnswer")).thenReturn("Question123");

        authenticator.action(context);

        verify(context).success();
    }

    @Test
    public void testValidateAnswer() {
        when(formData.getFirst(Utils.FORM_SECURITY_ANSWER_FIELD)).thenReturn("Answer123");

        Map<String, String> configMap = new HashMap<>();
        configMap.put(Utils.NUM_LAST_CHARS, "3");
        configMap.put(Utils.USER_ATTRIBUTE, "securityAnswer");
        when(config.getConfig()).thenReturn(configMap);

        when(user.getFirstAttribute("securityAnswer")).thenReturn("Question123");

        boolean isValid = authenticator.validateAnswer(context);

        assertTrue(isValid);
    }

    @Test
    public void testValidateAnswerInvalid() {
        when(formData.getFirst(Utils.FORM_SECURITY_ANSWER_FIELD)).thenReturn("wrongAnswer");

        Map<String, String> configMap = new HashMap<>();
        configMap.put(Utils.NUM_LAST_CHARS, "3");
        configMap.put(Utils.USER_ATTRIBUTE, "securityAnswer");
        when(config.getConfig()).thenReturn(configMap);

        when(user.getFirstAttribute("securityAnswer")).thenReturn("correctAnswer");

        boolean isValid = authenticator.validateAnswer(context);

        assertFalse(isValid);
    }
}


*/