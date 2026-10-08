// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import static sequent.keycloak.authenticator.Utils.ACCESS_DENIED;
import static sequent.keycloak.authenticator.Utils.AUTH_NOTE_DENY_TYPE;
import static sequent.keycloak.authenticator.Utils.USER_NOT_FOUND;

import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.authenticators.x509.UserIdentityToModelMapper;
import org.keycloak.authentication.authenticators.x509.X509AuthenticatorConfigModel;
import org.keycloak.authentication.authenticators.x509.X509ClientCertificateAuthenticator;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;

/**
 * Extends the built-in {@code auth-x509-client-username-form} authenticator to set the {@code
 * deny-type} auth note when user lookup fails. This allows the {@code Deny Subflow} to route to a
 * specific error (e.g. "User not found") rather than the generic access-denied fallback.
 */
@JBossLog
public class X509UserResolutionAuthenticator extends X509ClientCertificateAuthenticator {

  // Marks that the certificate passed validation but matched no enabled user. The parent's
  // USER_NOT_FOUND event error can't be read back: since Keycloak 26.8 EventBuilder.error() sends
  // a clone and leaves the builder's own event untouched.
  static final String AUTH_NOTE_USER_LOOKUP_FAILED = "x509-user-lookup-failed";

  @Override
  public void authenticate(AuthenticationFlowContext context) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    authSession.removeAuthNote(AUTH_NOTE_USER_LOOKUP_FAILED);
    authenticateCertificate(context);
    // If there was an error the SPI do not set the user.
    // We also check that the deny type is not already set by the X509CertClassifierAuthenticator,
    // to avoid overwriting it with a generic access-denied
    if (context.getUser() == null && authSession.getAuthNote(AUTH_NOTE_DENY_TYPE) == null) {
      String denyType =
          authSession.getAuthNote(AUTH_NOTE_USER_LOOKUP_FAILED) != null
              ? USER_NOT_FOUND
              : ACCESS_DENIED;
      log.infov(
          "authenticate(): user not resolved after X509 validation, setting {0}={1}",
          AUTH_NOTE_DENY_TYPE, denyType);
      authSession.setAuthNote(AUTH_NOTE_DENY_TYPE, denyType);
      context.getEvent().detail(AUTH_NOTE_DENY_TYPE, denyType);
    }
    authSession.removeAuthNote(AUTH_NOTE_USER_LOOKUP_FAILED);
  }

  void authenticateCertificate(AuthenticationFlowContext context) {
    super.authenticate(context);
  }

  @Override
  public UserIdentityToModelMapper getUserIdentityToModelMapper(
      X509AuthenticatorConfigModel config) {
    UserIdentityToModelMapper mapper = super.getUserIdentityToModelMapper(config);
    return new UserIdentityToModelMapper() {
      @Override
      public UserModel find(AuthenticationFlowContext context, Object userIdentity)
          throws Exception {
        UserModel user = mapper.find(context, userIdentity);
        if (user == null || !user.isEnabled()) {
          context.getAuthenticationSession().setAuthNote(AUTH_NOTE_USER_LOOKUP_FAILED, "true");
        }
        return user;
      }
    };
  }
}
