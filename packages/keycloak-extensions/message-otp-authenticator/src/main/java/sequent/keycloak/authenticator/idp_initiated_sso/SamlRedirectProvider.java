package sequent.keycloak.authenticator.idp_initiated_sso;

import org.jboss.logging.Logger;
import org.keycloak.models.KeycloakSession;
import org.keycloak.protocol.oidc.utils.RedirectUtils;
import org.keycloak.models.RealmModel;
import org.keycloak.models.ClientModel;

import jakarta.ws.rs.*;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import java.net.URI;
import java.net.URISyntaxException;


public class SamlRedirectProvider implements org.keycloak.services.resource.RealmResourceProvider {

    private static final Logger logger = Logger.getLogger(SamlRedirectProvider.class);
    private final KeycloakSession session;

    public SamlRedirectProvider(KeycloakSession session) {
        logger.infof("Created redirect provider");

        this.session = session;
    }

    @Override
    public Object getResource() {
        return this;
    }

    /**
     * This method handles the POST request to /redirect-me
     * It consumes a standard HTML form submission.
     */
    @POST
    @Path("redirect")
    @Consumes(MediaType.APPLICATION_FORM_URLENCODED)
    @Produces(MediaType.APPLICATION_JSON)
    public Response handleRedirect(@FormParam("RelayState") String relayState) {

        // Log the incoming request, just like the Node.js server
        logger.infof("Received POST with RelayState: %s", relayState);

        // Check if the RelayState was provided
        if (relayState == null || relayState.isEmpty()) {
            logger.warn("RelayState is missing from the request body.");
            String jsonError = "{\"error\": \"RelayState is missing from the request body.\"}";
            return Response
                    .status(Response.Status.BAD_REQUEST)
                    .entity(jsonError)
                    .build();
        }

        // Get the client config to verify redirect uri's
        RealmModel realm = session.getContext().getRealm();
        logger.infof("Current realm: %s", realm.getName());
        ClientModel vpssoClient = realm.getClientByClientId("vp-sso");
        logger.infof("Has vp-sso client? %b", vpssoClient != null);

        // Returns null if either the client is not set or the redirect uri is invalid.
        String verifiedUrl = RedirectUtils.verifyRedirectUri(session, relayState, vpssoClient);

        // Check if the RelayState was provided
        if (verifiedUrl == null || verifiedUrl.isEmpty()) {
            logger.warn("RelayState has an invalid redirect uri or vp-sso saml client is not set.");
            String jsonError = "{\"error\": \"RelayState has an invalid redirect uri or vp-sso saml client is not set.\"}";
            return Response
                    .status(Response.Status.BAD_REQUEST)
                    .entity(jsonError)
                    .build();
        }

        // If it exists redirect to it
        try {
            URI location = new URI(verifiedUrl);
            return Response
                    .status(Response.Status.FOUND) // 302 Redirect
                    .location(location)
                    .build();
        } catch (URISyntaxException e) {
            logger.error("Invalid URI syntax in RelayState", e);
            String jsonError = "{\"error\": \"Invalid RelayState URL format.\"}";
            return Response
                    .status(Response.Status.BAD_REQUEST)
                    .entity(jsonError)
                    .build();
        }
    }

    @Override
    public void close() {
        // No-op
    }
}
