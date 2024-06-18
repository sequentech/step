package sequent.keycloak.conditional_authenticators;
/* 
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.actiontoken.ActionTokenContext;
import org.keycloak.events.EventBuilder;
import org.keycloak.events.EventType;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.UserModel;
import org.keycloak.protocol.oidc.OIDCLoginProtocol;
import org.keycloak.sessions.AuthenticationSessionModel;

import jakarta.ws.rs.core.Response;
import jakarta.ws.rs.core.UriInfo;
import sequent.keycloak.conditional_authenticators.ManualVerificationToken;
import sequent.keycloak.conditional_authenticators.ManualVerificationTokenHandler;

import java.util.Collections;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.mockito.ArgumentMatchers.*;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

public class ManualVerificationTokenHandlerTest {

    private ActionTokenContext<ManualVerificationToken> tokenContext;
    private UserModel user;
    private KeycloakSession session;
    private EventBuilder eventBuilder;
    private UriInfo uriInfo;
    private LoginFormsProvider loginFormsProvider;

    private ManualVerificationTokenHandler tokenHandler;

    @BeforeEach
    void setUp() {
        tokenContext = mock(ActionTokenContext.class);
        user = mock(UserModel.class);
        session = mock(KeycloakSession.class);
        eventBuilder = mock(EventBuilder.class);
        uriInfo = mock(UriInfo.class);
        loginFormsProvider = mock(LoginFormsProvider.class);
    
        // Inject session mock into tokenContext or other mocks as necessary
        // Example: 
        // when(tokenContext.getSession()).thenReturn(session);
        when(tokenContext.getEvent()).thenReturn(eventBuilder);
    
        tokenHandler = new ManualVerificationTokenHandler();
    }
    
    
    
    
    

    @Test
    void testHandleToken() {
        // Prepare mock behavior
        ManualVerificationToken token = new ManualVerificationToken();
        token.setRedirectUri("https://example.com");

        // Mock the necessary dependencies
        AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
        when(tokenContext.getAuthenticationSession()).thenReturn(authSession);
        when(tokenContext.getSession()).thenReturn(session);
        when(tokenContext.getEvent()).thenReturn(eventBuilder);
        when(tokenContext.getUriInfo()).thenReturn(uriInfo);
        when(session.getProvider(LoginFormsProvider.class)).thenReturn(loginFormsProvider);

        when(authSession.getAuthenticatedUser()).thenReturn(user);
        when(user.getUsername()).thenReturn("testUser");
        when(user.getFirstAttribute(any())).thenReturn(null);

        // Call method under test
        Response response = tokenHandler.handleToken(token, tokenContext);

        // Verify interactions and assertions
        // Using any() and eq() from Mockito to match arguments
        // Verify and set up your mocks as needed

        assertEquals(Response.Status.SEE_OTHER.getStatusCode(), response.getStatus());
        assertEquals("https://example.com", response.getLocation().toString());
    }
}
*/