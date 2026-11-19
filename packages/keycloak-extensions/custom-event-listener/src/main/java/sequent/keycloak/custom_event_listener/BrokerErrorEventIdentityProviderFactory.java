// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.custom_event_listener;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.Response;
import org.keycloak.broker.oidc.OIDCIdentityProvider;
import org.keycloak.broker.oidc.OIDCIdentityProviderConfig;
import org.keycloak.broker.oidc.OIDCIdentityProviderFactory;
import org.keycloak.broker.provider.IdentityProviderFactory;
import org.keycloak.broker.provider.UserAuthenticationIdentityProvider;
import org.keycloak.events.Details;
import org.keycloak.events.Errors;
import org.keycloak.events.EventBuilder;
import org.keycloak.events.EventType;
import org.keycloak.models.IdentityProviderModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;

/**
 * Replaces the built-in "oidc" identity provider (same id: Keycloak prefers a factory outside its
 * own packages over the built-in one) so that an error return from the external authentication
 * server ({@code error=access_denied} or any other error) emits an {@code
 * IDENTITY_PROVIDER_LOGIN_ERROR} event. Keycloak itself emits no event on that path, so the failed
 * sign-in would reach neither the failure counts nor the electoral log.
 */
@AutoService(IdentityProviderFactory.class)
public class BrokerErrorEventIdentityProviderFactory extends OIDCIdentityProviderFactory {

  @Override
  public OIDCIdentityProvider create(KeycloakSession session, IdentityProviderModel model) {
    return new Provider(session, new OIDCIdentityProviderConfig(model));
  }

  static class Provider extends OIDCIdentityProvider {
    Provider(KeycloakSession session, OIDCIdentityProviderConfig config) {
      super(session, config);
    }

    @Override
    public Object callback(
        RealmModel realm,
        UserAuthenticationIdentityProvider.AuthenticationCallback callback,
        EventBuilder event) {
      return super.callback(realm, new ErrorEventCallback(callback, event), event);
    }
  }

  /**
   * The broker endpoint calls {@code cancelled} and {@code error} only for an error return, after
   * it has verified the authentication session named by {@code state}.
   */
  static class ErrorEventCallback
      implements UserAuthenticationIdentityProvider.AuthenticationCallback {
    private final UserAuthenticationIdentityProvider.AuthenticationCallback delegate;
    private final EventBuilder event;

    ErrorEventCallback(
        UserAuthenticationIdentityProvider.AuthenticationCallback delegate, EventBuilder event) {
      this.delegate = delegate;
      this.event = event;
    }

    private void failed(IdentityProviderModel idp, String error, String reason) {
      EventBuilder failure =
          event
              .clone()
              .event(EventType.IDENTITY_PROVIDER_LOGIN)
              .detail(Details.IDENTITY_PROVIDER, idp.getAlias());
      if (reason != null) failure.detail(Details.REASON, reason);
      failure.error(error);
    }

    @Override
    public org.keycloak.sessions.AuthenticationSessionModel getAndVerifyAuthenticationSession(
        String encodedCode) {
      return delegate.getAndVerifyAuthenticationSession(encodedCode);
    }

    @Override
    public Response authenticated(org.keycloak.broker.provider.BrokeredIdentityContext context) {
      return delegate.authenticated(context);
    }

    @Override
    public Response cancelled(IdentityProviderModel idpConfig) {
      failed(idpConfig, Errors.ACCESS_DENIED, null);
      return delegate.cancelled(idpConfig);
    }

    @Override
    public Response retryLogin(
        UserAuthenticationIdentityProvider<?> identityProvider,
        org.keycloak.sessions.AuthenticationSessionModel authSession) {
      return delegate.retryLogin(identityProvider, authSession);
    }

    @Override
    public Response error(IdentityProviderModel idpConfig, String message) {
      failed(idpConfig, Errors.IDENTITY_PROVIDER_ERROR, message);
      return delegate.error(idpConfig, message);
    }
  }
}
