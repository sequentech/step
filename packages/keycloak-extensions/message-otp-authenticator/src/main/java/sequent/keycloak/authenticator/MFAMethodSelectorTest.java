package sequent.keycloak.authenticator;

/*    BUILD FAILED 
 * Unit Testing MFA Method Selector  JC:Ayeng  6132024
 *
 * NOTE : Commented out so dependencies are installed properly for other Unit Testing Files.
 *         UNCOMMENT IF TESTING FOR THIS FILE ONLY, or dependencies will be needed in all extensions
* /
 

import static org.mockito.Mockito.*;
import static org.junit.jupiter.api.Assertions.*;

import jakarta.ws.rs.core.MultivaluedMap;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.mockito.InjectMocks;
import org.mockito.Mock;
import org.mockito.junit.jupiter.MockitoExtension;
import org.keycloak.authentication.RequiredActionContext;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticationSessionModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;

import java.util.Map;
import java.util.Set;
import java.util.stream.Collectors;
import java.util.stream.Stream;

@ExtendWith(MockitoExtension.class)
public class MFAMethodSelectorTest {

    @InjectMocks
    private MFAMethodSelector selector;

    @Mock
    private RequiredActionContext context;
    
    @Mock
    private UserModel user;
    
    @Mock
    private AuthenticationSessionModel authSession;
    
    @Mock
    private KeycloakSession session;
    
    @Mock
    private RealmModel realm;

    @Mock
    private LoginFormsProvider form;

    @BeforeEach
    public void setUp() {
        when(context.getUser()).thenReturn(user);
        when(context.getAuthenticationSession()).thenReturn(authSession);
        when(context.getSession()).thenReturn(session);
        when(context.getRealm()).thenReturn(realm);
    }

    @Test
    public void testEvaluateTriggers_NoActionRequired() {
        when(authSession.getRequiredActions()).thenReturn(Set.of());

        selector.evaluateTriggers(context);

        verify(authSession, never()).addRequiredAction(MFAMethodSelector.PROVIDER_ID);
    }

    @Test
    public void testEvaluateTriggers_AddsAction() {
        when(authSession.getRequiredActions()).thenReturn(Set.of());
        when(user.credentialManager().getStoredCredentialsByTypeStream(anyString())).thenReturn(Stream.empty());

        selector.evaluateTriggers(context);

        verify(authSession).addRequiredAction(MFAMethodSelector.PROVIDER_ID);
    }

    @Test
    public void testRequiredActionChallenge() {
        when(context.form()).thenReturn(form);
        when(realm.getRequiredActionProvidersStream()).thenReturn(Stream.empty());
        
        selector.requiredActionChallenge(context);
        
        verify(form).setAttribute("realm", realm);
        verify(form).setAttribute("user", user);
        verify(context).challenge(any());
    }

    @Test
    public void testProcessAction() {
        MultivaluedMap<String, String> formData = mock(MultivaluedMap.class);
        when(context.getHttpRequest().getDecodedFormParameters()).thenReturn(formData);
        when(formData.getFirst("requiredActionName")).thenReturn("some-action");

        selector.processAction(context);

        verify(authSession).addRequiredAction("some-action");
        verify(authSession).removeRequiredAction(MFAMethodSelector.PROVIDER_ID);
        verify(context).success();
    }
}
 
*/