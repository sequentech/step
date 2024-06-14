package sequent.keycloak.security_question_authenticator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import org.jboss.resteasy.specimpl.MultivaluedMapImpl;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.InitiatedActionSupport;
import org.keycloak.authentication.RequiredActionContext;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;

import javax.ws.rs.core.Response;
import java.util.Collections;
import java.util.Optional;
import java.util.stream.Stream;

public class SecurityQuestionRequiredActionTest {

    private SecurityQuestionRequiredAction requiredAction;
    private RequiredActionContext context;
    private UserModel mockUser;
    private AuthenticationSessionModel mockAuthSession;

    @BeforeEach
    void setUp() {
        requiredAction = new SecurityQuestionRequiredAction();
        context = mock(RequiredActionContext.class);
        mockUser = mock(UserModel.class);
        mockAuthSession = mock(AuthenticationSessionModel.class);

        when(context.getUser()).thenReturn(mockUser);
        when(context.getAuthenticationSession()).thenReturn(mockAuthSession);
        when(Utils.getConfig(any())).thenReturn(Optional.of(mock(AuthenticatorConfigModel.class)));
    }

    @Test
    void testInitiatedActionSupport() {
        InitiatedActionSupport support = requiredAction.initiatedActionSupport();
        assertEquals(InitiatedActionSupport.SUPPORTED, support);
    }

    @Test
    void testEvaluateTriggers_AddsRequiredAction() {
        // Setup mock behavior
        when(mockAuthSession.getRequiredActions()).thenReturn(Collections.emptySet());
        when(mockUser.credentialManager().getStoredCredentialsByTypeStream(any())).thenReturn(Stream.empty());
        when(mockUser.getRequiredActionsStream()).thenReturn(Stream.of("action1"));

        requiredAction.evaluateTriggers(context);

        verify(mockAuthSession).addRequiredAction(SecurityQuestionRequiredAction.PROVIDER_ID);
    }

    @Test
    void testRequiredActionChallenge() {
        Response mockResponse = mock(Response.class);
        when(context.form()).thenReturn(new LoginFormsProviderImpl());
        when(context.form().createForm(any())).thenReturn(mockResponse);

        requiredAction.requiredActionChallenge(context);

        verify(context).challenge(mockResponse);
    }

    @Test
    void testProcessAction_ValidAnswer() {
        // Setup mock behavior
        when(mockAuthSession.getRequiredActions()).thenReturn(Collections.singleton(SecurityQuestionRequiredAction.PROVIDER_ID));
        when(mockAuthSession.getExecution()).thenReturn(mock(AuthenticationExecutionModel.class));
        when(mockAuthSession.getExecution().isRequired()).thenReturn(true);
        when(mockAuthSession.getExecution().isConditional()).thenReturn(false);
        when(mockAuthSession.getExecution().isAlternative()).thenReturn(false);

        // Mock form data
        MultivaluedMapImpl<String, String> formData = new MultivaluedMapImpl<>();
        formData.add(Utils.FORM_SECURITY_ANSWER_FIELD, "validAnswer");
        when(context.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        requiredAction.processAction(context);

        verify(context).success();
    }

    @Test
    void testProcessAction_InvalidAnswer() {
        // Setup mock behavior
        when(mockAuthSession.getRequiredActions()).thenReturn(Collections.singleton(SecurityQuestionRequiredAction.PROVIDER_ID));
        when(mockAuthSession.getExecution()).thenReturn(mock(AuthenticationExecutionModel.class));
        when(mockAuthSession.getExecution().isRequired()).thenReturn(true);
        when(mockAuthSession.getExecution().isConditional()).thenReturn(false);
        when(mockAuthSession.getExecution().isAlternative()).thenReturn(false);

        // Mock form data
        MultivaluedMapImpl<String, String> formData = new MultivaluedMapImpl<>();
        formData.add(Utils.FORM_SECURITY_ANSWER_FIELD, "invalidAnswer");
        when(context.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);

        requiredAction.processAction(context);

        verify(context).challenge(any());
    }
}
