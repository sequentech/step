// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import com.google.auto.service.AutoService;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.authentication.ConfigurableAuthenticatorFactory;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.provider.ProviderConfigProperty;

import jakarta.ws.rs.core.Response;
import jakarta.ws.rs.core.UriBuilder;
import java.net.URI;
import java.util.List;
import java.util.Collections;
import org.jboss.logging.Logger;

/**
 * Authenticator that transparently redirects users from login to registration flow.
 * This authenticator intercepts the login process and redirects the user to the
 * registration page instead, making it appear seamless to the user.
 */
public class RedirectToRegisterAuthenticator implements Authenticator {

    public static final String PROVIDER_ID = "redirect-to-register";
    private static final Logger log = Logger.getLogger(RedirectToRegisterAuthenticator.class);

    @Override
    public void authenticate(final AuthenticationFlowContext context) {
        // Get the base URI and construct the registration URL
        final URI baseUri = context.getUriInfo().getBaseUri();
        final String realm = context.getRealm().getName();

        // Build the registration URL using UriBuilder for safety
        UriBuilder builder = UriBuilder.fromUri(baseUri)
                .path("realms")
                .path(realm)
                .path("login-actions/registration");

        // Optionally, filter query parameters to avoid open redirect issues
        String queryString = context.getUriInfo().getRequestUri().getQuery();
        if (queryString != null && !queryString.isEmpty()) {
            // Only allow safe parameters (example: skip redirect_uri)
            // In production, parse and filter as needed
            if (!queryString.contains("redirect_uri")) {
                builder.replaceQuery(queryString);
            }
        }

        String registrationUrl = builder.build().toString();
        log.debugf("Redirecting to registration URL: %s", registrationUrl);

        // Perform the redirect
        Response response = Response.status(Response.Status.FOUND)
                .location(URI.create(registrationUrl))
                .build();

        context.challenge(response);
    }

    @Override
    public void action(final AuthenticationFlowContext context) {
        context.success();
    }

    @Override
    public boolean requiresUser() {
        return false;
    }

    @Override
    public boolean configuredFor(final KeycloakSession session, final RealmModel realm, final UserModel user) {
        return true;
    }

    @Override
    public void setRequiredActions(final KeycloakSession session, final RealmModel realm, final UserModel user) {
        // No required actions
    }

    @Override
    public void close() {
        // Nothing to close
    }

    /**
     * Factory class for the RedirectToRegisterAuthenticator
     */
    @AutoService(AuthenticatorFactory.class)
    public static class Factory implements AuthenticatorFactory, ConfigurableAuthenticatorFactory {

        @Override
        public String getDisplayType() {
            return "Redirect to Registration";
        }

        @Override
        public String getReferenceCategory() {
            return "redirect";
        }

        @Override
        public boolean isConfigurable() {
            return false;
        }

        @Override
        public AuthenticationExecutionModel.Requirement[] getRequirementChoices() {
            return new AuthenticationExecutionModel.Requirement[]{
                    AuthenticationExecutionModel.Requirement.REQUIRED,
                    AuthenticationExecutionModel.Requirement.DISABLED
            };
        }

        @Override
        public boolean isUserSetupAllowed() {
            return false;
        }

        @Override
        public String getHelpText() {
            return "Transparently redirects users from login to registration flow.";
        }

        @Override
        public List<ProviderConfigProperty> getConfigProperties() {
            return Collections.emptyList();
        }

        @Override
        public Authenticator create(final KeycloakSession session) {
            return new RedirectToRegisterAuthenticator();
        }

        @Override
        public void init(final org.keycloak.Config.Scope config) {
            // No initialization needed
        }

        @Override
        public void postInit(final org.keycloak.models.KeycloakSessionFactory factory) {
            // No post-initialization needed
        }

        @Override
        public void close() {
            // Nothing to close
        }

        @Override
        public String getId() {
            return PROVIDER_ID;
        }
    }
}
