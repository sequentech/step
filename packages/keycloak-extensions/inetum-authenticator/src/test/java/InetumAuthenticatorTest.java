package sequent.keycloak.inetum_authenticator;

//import javax.ws.rs.core.Response;

import static org.mockito.Mockito.*;
import static org.junit.Assert.*;

import org.junit.Before;
import org.junit.Test;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.*;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.mockito.*;

import java.io.IOException;
import java.util.HashMap;
import java.util.Map;

public class InetumAuthenticatorTest {
    @Mock
    private AuthenticationFlowContext context;

    @Mock
    private AuthenticatorConfigModel configModel;

    @Mock
    private UserModel user;

    @Mock
    private LoginFormsProvider formProvider;

    @Mock
    private AuthenticationSessionModel sessionModel;

    @InjectMocks
    private InetumAuthenticator authenticator;

    @Before
    public void setUp() {
        MockitoAnnotations.initMocks(this);
    }

    @Test
    public void testAuthenticate_UserValidated_Success() {
        Map<String, String> configMap = new HashMap<>();
        configMap.put(Utils.USER_STATUS_ATTRIBUTE, "userStatus");

        when(context.getAuthenticatorConfig()).thenReturn(configModel);
        when(configModel.getConfig()).thenReturn(configMap);
        when(context.getUser()).thenReturn(user);
        when(user.getFirstAttribute("userStatus")).thenReturn("TRUE");

        authenticator.authenticate(context);

        verify(context).success();
    }

    @Test
    public void testAuthenticate_UserNotValidated_RenderForm() throws IOException {
        Map<String, String> configMap = new HashMap<>();
        configMap.put(Utils.USER_STATUS_ATTRIBUTE, "userStatus");

        when(context.getAuthenticatorConfig()).thenReturn(configModel);
        when(configModel.getConfig()).thenReturn(configMap);
        when(context.getUser()).thenReturn(user);
        when(user.getFirstAttribute("userStatus")).thenReturn("FALSE");
        when(context.getAuthenticationSession()).thenReturn(sessionModel);
        when(context.form()).thenReturn(formProvider);
        when(formProvider.setAttribute(anyString(), anyString())).thenReturn(formProvider);

        Map<String, String> transactionData = new HashMap<>();
        transactionData.put(Utils.FTL_TOKEN_DOB, "tokenDob");
        transactionData.put(Utils.FTL_USER_ID, "userId");

        InetumAuthenticator spyAuthenticator = Mockito.spy(authenticator);
        doReturn(transactionData).when(spyAuthenticator).newTransaction(anyMap(), any());

        spyAuthenticator.authenticate(context);

        verify(sessionModel).setAuthNote(Utils.FTL_TOKEN_DOB, "tokenDob");
        verify(sessionModel).setAuthNote(Utils.FTL_USER_ID, "userId");
        verify(formProvider).setAttribute(Utils.FTL_USER_ID, "userId");
        verify(formProvider).setAttribute(Utils.FTL_TOKEN_DOB, "tokenDob");
        verify(context).challenge(any(Response.class));
    }

    @Test
    public void testAuthenticate_NewTransactionError_InternalError() throws IOException {
        Map<String, String> configMap = new HashMap<>();
        configMap.put(Utils.USER_STATUS_ATTRIBUTE, "userStatus");

        when(context.getAuthenticatorConfig()).thenReturn(configModel);
        when(configModel.getConfig()).thenReturn(configMap);
        when(context.getUser()).thenReturn(user);
        when(user.getFirstAttribute("userStatus")).thenReturn("FALSE");
        when(context.getAuthenticationSession()).thenReturn(sessionModel);
        when(context.form()).thenReturn(formProvider);
        when(formProvider.setAttribute(anyString(), anyString())).thenReturn(formProvider);

        InetumAuthenticator spyAuthenticator = Mockito.spy(authenticator);
        doThrow(new IOException()).when(spyAuthenticator).newTransaction(anyMap(), any());

        spyAuthenticator.authenticate(context);

        verify(context).failure(AuthenticationFlowError.INTERNAL_ERROR);
        verify(context).attempted();
        verify(formProvider).setAttribute(Utils.FTL_ERROR, Utils.FTL_ERROR_INTERNAL);
        verify(context).challenge(any(Response.class));
    }
}
