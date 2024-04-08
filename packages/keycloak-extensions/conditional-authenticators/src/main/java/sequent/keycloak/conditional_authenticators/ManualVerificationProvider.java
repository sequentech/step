package sequent.keycloak.conditional_authenticators;

import java.util.HashMap;
import java.util.List;
import java.util.Map;

import org.keycloak.authentication.actiontoken.execactions.ExecuteActionsActionToken;
import org.keycloak.authorization.util.Tokens;
import org.keycloak.common.util.Time;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.representations.AccessToken;
import org.keycloak.services.resource.RealmResourceProvider;
import org.keycloak.services.resources.LoginActionsService;

import jakarta.ws.rs.ForbiddenException;
import jakarta.ws.rs.GET;
import jakarta.ws.rs.NotAuthorizedException;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.QueryParam;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import jakarta.ws.rs.core.UriBuilder;
import jakarta.ws.rs.ext.Provider;
import lombok.extern.jbosslog.JBossLog;

/*
 * curl -v http://127.0.0.1:8090/realms/master/manual-verification/ping | jq -C .
 * curl -v http://127.0.0.1:8090/realms/master/manual-verification/generate-link?userName=edulix@nvotes.com | jq -C .
 */
@JBossLog
@Provider
public class ManualVerificationProvider implements RealmResourceProvider {
    static final String CLIENT_ID = "manual-verification";

    protected final KeycloakSession session;
    protected final AccessToken token;


    public ManualVerificationProvider(KeycloakSession session) {
        log.info("ManualVerificationProvider");
        this.session = session;
        this.token = Tokens.getAccessToken(session);
    }

        @GET
    @Path("/ping")
    public Object ping() {
        log.info("ping");
        Map<String, String> response = new HashMap<>();
        response.put("answer", "pong");
        return Response.ok(response).build();
    }

    @GET
    @Path("/generate-link")
    @Produces(MediaType.APPLICATION_JSON)
    public Response generateLink(@QueryParam("userName") String userName) {
        log.info("generateLink");
        // throws some exception if is not an admin
        //checkPermissions();

        String tokenLink = generateTokenLink(userName);
        if (tokenLink != null) {
            Map<String, String> response = new HashMap<>();
            response.put("link", tokenLink);
            return Response.ok(response).build();
        }
        return Response.status(Response.Status.NOT_FOUND).build();
    }

    private String generateTokenLink(String userName)
    {
        log.info("generateTokenLink");
        RealmModel realm = session.getContext().getRealm();
        int lifespan = realm.getActionTokenGeneratedByAdminLifespan();
        int expiration = Time.currentTime() + lifespan;

        UserModel user = session
            .users()
            .getUserByUsername(realm, userName);
        if (user != null)
        {
            // Generate the token
            ExecuteActionsActionToken token = new ExecuteActionsActionToken(
                user.getId(),
                expiration,
                List.of(UserModel.RequiredAction.UPDATE_PASSWORD.name()),
                null,
                CLIENT_ID
            );

            UriBuilder builder = LoginActionsService
                .actionTokenProcessor(session.getContext().getUri());
            builder
                .queryParam(
                    "key",
                    token.serialize(
                        session,
                        realm,
                        session.getContext().getUri()
                    )
                );
            return builder.build(realm.getName()).toString();
        }
        return null;
    }

    /*
     * Throws some exception if the user does not have the admin role.
     */
    private void checkPermissions() {
        log.info("checkPermissions");
        if (token == null) {
            throw new NotAuthorizedException("Bearer");
        }

        if (token.getRealmAccess() == null || !token.getRealmAccess().isUserInRole("admin")) {
            throw new ForbiddenException("User does not have realm admin role");
        }
    }

    @Override
    public Object getResource() {
        log.info("getResource");
        return this;
    }
    
    @Override
    public void close() {
        log.info("close");
    }
}
