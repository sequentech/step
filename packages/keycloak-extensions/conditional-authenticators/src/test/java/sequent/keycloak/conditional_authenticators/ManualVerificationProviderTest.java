package sequent;

import static org.mockito.Mockito.*;

import java.net.URI;

import org.junit.Test;
import org.junit.jupiter.api.BeforeEach;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.models.KeycloakContext;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakUriInfo;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.representations.AccessToken;
import org.mockito.Mock;
import org.mockito.MockitoAnnotations;
import sequent.keycloak.conditional_authenticators.ManualVerificationProvider;
import sequent.keycloak.conditional_authenticators.ManualVerificationToken;

public class ManualVerificationProviderTest {
    private ManualVerificationProvider provider;
    @Mock
    private KeycloakSession session;
    @Mock 
    private AccessToken accessToken;
    @Mock 
    private RealmModel realmModel;
    @Mock
    private UserModel user;
    @Mock
    private KeycloakContext context;
    @Mock
    private KeycloakUriInfo uriInfo;

    @BeforeEach
    public void setup() {
        MockitoAnnotations.openMocks(this);
        when(context.getRealm()).thenReturn(realmModel);
        
        provider = new ManualVerificationProvider(session);
    }

    @Test
    public void testGenerateTokenLink() {
       String userId = "sampleUserId";
       String redirectUri = "https://example.com/callback";
       String realmBaseUri = "https://example.com/auth/realms/";
       int expirationInSecs = 3600;
       ManualVerificationToken mvToken = new ManualVerificationToken(userId, expirationInSecs, redirectUri);
       String serializedTokenUri = "TestTokenSerializedUri";
       String realmName = "TestRealmName";
       session = mock(KeycloakSession.class);
       context = mock(KeycloakContext.class);
       uriInfo = mock(KeycloakUriInfo.class);
       realmModel = mock(RealmModel.class);
       
       when(session.getContext()).thenReturn(context);
       when(session.getContext().getUri()).thenReturn(uriInfo);
       when(mvToken.serialize(session, realmModel, session.getContext().getUri())).thenReturn(serializedTokenUri);
       when(realmModel.getName()).thenReturn(realmName);

    }

}

