// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import static sequent.keycloak.authenticator.Utils.ACCESS_DENIED;
import static sequent.keycloak.authenticator.Utils.AUTH_NOTE_DENY_TYPE;
import static sequent.keycloak.authenticator.Utils.USER_NOT_FOUND;

import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.authenticators.x509.X509ClientCertificateAuthenticator;
import org.keycloak.events.Errors;

/**
 * Extends the built-in {@code auth-x509-client-username-form} authenticator to set the {@code
 * deny-type} auth note when user lookup fails. This allows the {@code Deny Subflow} to route to a
 * specific error (e.g. "User not found") rather than the generic access-denied fallback.
 */
@JBossLog
public class X509UserResolutionAuthenticator extends X509ClientCertificateAuthenticator {

  @Override
  public void authenticate(AuthenticationFlowContext context) {
    super.authenticate(context);
    String eventError = context.getEvent().getEvent().getError();
    log.infov("X509UserResolutionAuthenticator.authenticate: eventError={0}", eventError);
    // If there was an error the SPI do not set the user.
    // We also check that the deny type is not already set by the X509CertClassifierAuthenticator,
    // to avoid overwriting it with a generic access-denied
    if (context.getUser() == null
        && context.getAuthenticationSession().getAuthNote(AUTH_NOTE_DENY_TYPE) == null) {
      String denyType =
          Errors.USER_NOT_FOUND.equals(eventError) || Errors.USER_DISABLED.equals(eventError)
              ? USER_NOT_FOUND
              : ACCESS_DENIED;
      log.infov(
          "authenticate(): user not resolved after X509 validation (eventError={0}), setting {1}={2}",
          eventError, AUTH_NOTE_DENY_TYPE, denyType);
      context.getAuthenticationSession().setAuthNote(AUTH_NOTE_DENY_TYPE, denyType);
      context.getEvent().detail(AUTH_NOTE_DENY_TYPE, denyType);
    }
  }
}
