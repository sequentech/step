package sequent.keycloak.security_question_authenticator;
/* 
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.*;

import org.jboss.resteasy.specimpl.MultivaluedMapImpl;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.keycloak.authentication.InitiatedActionSupport;
import org.keycloak.authentication.RequiredActionContext;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.sessions.CommonClientSessionModel;
import org.keycloak.sessions.CommonClientSessionModel.ExecutionStatus;
import org.keycloak.sessions.CommonClientSessionModel;

import org.mockito.InjectMocks;
import org.mockito.Mock;
import org.mockito.junit.jupiter.MockitoExtension;

import jakarta.ws.rs.core.Response;

import java.util.Collections;
import java.util.Map;
import java.util.Optional;
import java.util.stream.Stream;

@ExtendWith(MockitoExtension.class)
public class SecurityQuestionRequiredActionTest {

    @InjectMocks
    private SecurityQuestionRequiredAction requiredAction;

    @Mock
    private RequiredActionContext context;

    @Mock
    private UserModel mockUser;

    @Mock
    private AuthenticationSessionModel mockAuthSession;

    @Mock
    private LoginFormsProvider formsProvider;

    @BeforeEach
    void setUp() {
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
        when(context.form()).thenReturn(formsProvider);
        when(formsProvider.createForm(any())).thenReturn(mockResponse);

        requiredAction.requiredActionChallenge(context);

        verify(context).challenge(mockResponse);
    }

    @Test
    void testProcessAction_ValidAnswer() {
        // Setup mock behavior
        when(mockAuthSession.getRequiredActions()).thenReturn(Collections.singleton(SecurityQuestionRequiredAction.PROVIDER_ID));
        
        CommonClientSessionModel.ExecutionStatus executionStatus = mock(CommonClientSessionModel.ExecutionStatus.class);
        when(mockAuthSession.getExecutionStatus()).thenReturn(executionStatus);

         

        when(executionStatus.isRequired()).thenReturn(true);
        when(executionStatus.isConditional()).thenReturn(false);
        when(executionStatus.isAlternative()).thenReturn(false);
    
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
        //ExecutionStatus executionStatus = mock(ExecutionStatus.class); 
        CommonClientSessionModel.ExecutionStatus executionStatus = mock(CommonClientSessionModel.ExecutionStatus.class);
        
        when(mockAuthSession.getExecutionStatus()).thenReturn(executionStatus);
    
        // Stub methods on executionStatus
        when(executionStatus.isRequired()).thenReturn(true);
        when(executionStatus.isConditional()).thenReturn(false);
        when(executionStatus.isAlternative()).thenReturn(false);
    
        // Mock form data
        MultivaluedMapImpl<String, String> formData = new MultivaluedMapImpl<>();
        formData.add(Utils.FORM_SECURITY_ANSWER_FIELD, "invalidAnswer");
        when(context.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);
    
        requiredAction.processAction(context);
    
        verify(context).challenge(any());
    }
    
}

*/



