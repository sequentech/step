package sequent.keycloak.conditional_authenticators;


/*              BUILD FAILED - CANNOT FIND JUNIT.JUPITER.API 
 * Unit Concitional AuthNote Authenticator  JC:Ayeng  6132024
 *
 * NOTE : Commented out so dependencies are installed properly for other Unit Testing Files.
 *         UNCOMMENT IF TESTING FOR THIS FILE ONLY, or dependencies will be needed in all extensions
* /


import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.mockito.InjectMocks;
import org.mockito.Mock;
import org.mockito.junit.jupiter.MockitoExtension;

import jakarta.ws.rs.core.Response;
import jakarta.ws.rs.core.UriInfo;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.representations.AccessToken;

import java.net.URI;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.mockito.Mockito.*;

@ExtendWith(MockitoExtension.class)
public class ManualVerificationProviderTest {

    @Mock
    private KeycloakSession session;
    
    @Mock
    private AccessToken token;
    
    @Mock
    private RealmModel realm;
    
    @Mock
    private UserModel user;

    @Mock
    private UriInfo uriInfo;

    @InjectMocks
    private ManualVerificationProvider manualVerificationProvider;

    @BeforeEach
    public void setUp() {
        manualVerificationProvider = new ManualVerificationProvider(session);
    }

    @Test
    public void testPing() {
        Response response = manualVerificationProvider.ping();
        assertEquals(Response.Status.OK.getStatusCode(), response.getStatus());
        Map<String, String> entity = (Map<String, String>) response.getEntity();
        assertEquals("pong", entity.get("answer"));
    }

    @Test
    public void testGenerateLink_UserFound() {
        String userId = "testUserId";
        String redirectUri = "http://redirect.uri";
        when(session.getContext().getRealm()).thenReturn(realm);
        when(realm.getActionTokenGeneratedByAdminLifespan()).thenReturn(600);
        when(session.users().getUserById(realm, userId)).thenReturn(user);
        when(user.getUsername()).thenReturn("testUser");
        when(uriInfo.getBaseUri()).thenReturn(URI.create("http://localhost:8080"));

        Response response = manualVerificationProvider.generateLink(userId, redirectUri);

        assertEquals(Response.Status.OK.getStatusCode(), response.getStatus());
        Map<String, String> entity = (Map<String, String>) response.getEntity();
        assertEquals(1, entity.size());
    }

    @Test
    public void testGenerateLink_UserNotFound() {
        String userId = "nonExistentUserId";
        String redirectUri = "http://redirect.uri";
        when(session.getContext().getRealm()).thenReturn

*/