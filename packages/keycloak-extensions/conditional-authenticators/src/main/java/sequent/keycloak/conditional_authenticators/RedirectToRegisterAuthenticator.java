// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.Response;
import jakarta.ws.rs.core.UriBuilder;
import java.net.URI;
import java.util.Collections;
import java.util.List;
import org.jboss.logging.Logger;
import org.keycloak.models.Constants;
import org.keycloak.models.ClientModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.AuthenticationProcessor;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.authentication.ConfigurableAuthenticatorFactory;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.services.Urls;

/**
 * Authenticator that transparently redirects users from login to registration
 * flow. This authenticator intercepts the login process and redirects the user
 * to the registration page instead, making it appear seamless to the user.
 */
public class RedirectToRegisterAuthenticator implements Authenticator {

  public static final String PROVIDER_ID = "redirect-to-register";
  private static final Logger log = Logger.getLogger(RedirectToRegisterAuthenticator.class);


    /**
     * Prepare base uri builder for later use
     *
     * @return base uri builder
     */
    protected UriBuilder prepareBaseUriBuilder(final AuthenticationFlowContext context) {
        final String requestURI = context.getUriInfo().getBaseUri().getPath();
        final UriBuilder uriBuilder = UriBuilder.fromUri(requestURI);
        final ClientModel client = context.getAuthenticationSession().getClient();
        final AuthenticationSessionModel authenticationSession = context.getAuthenticationSession();

        if (client != null) {
            uriBuilder.queryParam(Constants.CLIENT_ID, client.getClientId());
        }
        if (authenticationSession != null) {
            uriBuilder.queryParam(Constants.TAB_ID, authenticationSession.getTabId());
        }
        return uriBuilder;
    }

  public void authenticate(final AuthenticationFlowContext context) {
    try {
      // Get the UriBuilder from the current request. This is crucial as it
      // preserves all existing query parameters like client_id, state, scope,
      // and most importantly, the 'execution' ID which maintains the
      // authentication flow state.
      final UriBuilder builder = prepareBaseUriBuilder(context);
      final URI baseUri = builder.build();

      // The name of the current realm.
      final String realmName = context.getRealm().getName();

      final URI registrationUri = Urls.realmRegisterPage(baseUri, realmName);

      // Perform the redirect using a 302 Found response.
      final Response response = Response
        .status(Response.Status.FOUND)
        .location(registrationUri).build();

      context.challenge(response);
    } catch (Exception e) {
      log.error("Failed to create redirect to registration", e);
      context.cancelLogin();
    }
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
  public boolean configuredFor(
      final KeycloakSession session, final RealmModel realm, final UserModel user) {
    return true;
  }

  @Override
  public void setRequiredActions(
      final KeycloakSession session, final RealmModel realm, final UserModel user) {
    // No required actions
  }

  @Override
  public void close() {
    // Nothing to close
  }

  /** Factory class for the RedirectToRegisterAuthenticator */
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
      return new AuthenticationExecutionModel.Requirement[] {
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
