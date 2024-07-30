// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import jakarta.ws.rs.core.Response;
import java.util.Optional;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.CredentialValidator;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialProvider;

@JBossLog
public class MessageOTPAuthenticator
    implements Authenticator, CredentialValidator<MessageOTPCredentialProvider> {
  public static final String MOBILE_NUMBER_FIELD = "sequent.read-only.mobile-number";
  private static final String TPL_CODE = "login-message-otp.ftl";

  @Override
  public MessageOTPCredentialProvider getCredentialProvider(KeycloakSession session) {
    log.info("getCredentialProvider()");
    return new MessageOTPCredentialProvider(session);
    // TODO: doesn't work - why?
    // return (MessageOTPCredentialProvider) session
    // 	.getProvider(
    // 		CredentialProvider.class,
    // 		MessageOTPCredentialProviderFactory.PROVIDER_ID
    // 	);
  }

  @Override
  public void authenticate(AuthenticationFlowContext context) {
    log.info("authenticate() called");
    AuthenticatorConfigModel config = context.getAuthenticatorConfig();

    log.infov("authenticate() Alias: {0}", config.getAlias());

    KeycloakSession session = context.getSession();
    AuthenticationSessionModel authSession = context.getAuthenticationSession();

    Utils.MessageCourier messageCourier =
        Utils.MessageCourier.fromString(config.getConfig().get(Utils.MESSAGE_COURIER_ATTRIBUTE));
    boolean deferredUser = config.getConfig().get(Utils.DEFERRED_USER_ATTRIBUTE).equals("true");
    try {
      UserModel user = context.getUser();
      Utils.sendCode(config, session, user, authSession, messageCourier, deferredUser);
      context.challenge(
          context
              .form()
              .setAttribute("realm", context.getRealm())
              .setAttribute("courier", messageCourier)
              .createForm(TPL_CODE));
    } catch (Exception error) {
      log.infov("there was an error {0}", error);
      context.failureChallenge(
          AuthenticationFlowError.INTERNAL_ERROR,
          context
              .form()
              .setError("messageNotSent", error.getMessage())
              .createErrorPage(Response.Status.INTERNAL_SERVER_ERROR));
    }
  }

  @Override
  public void action(AuthenticationFlowContext context) {
    log.info("action() called");
    String enteredCode = context.getHttpRequest().getDecodedFormParameters().getFirst(Utils.CODE);

    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    String code = authSession.getAuthNote(Utils.CODE);
    String ttl = authSession.getAuthNote(Utils.CODE_TTL);

    if (code == null || ttl == null) {
      context.failureChallenge(
          AuthenticationFlowError.INTERNAL_ERROR,
          context.form().createErrorPage(Response.Status.INTERNAL_SERVER_ERROR));
      return;
    }

    boolean isValid = Utils.constantTimeIsEqual(enteredCode.getBytes(), code.getBytes());
    if (isValid) {
      context.getAuthenticationSession().removeAuthNote(Utils.CODE);
      if (Long.parseLong(ttl) < System.currentTimeMillis()) {
        // expired
        context.failureChallenge(
            AuthenticationFlowError.EXPIRED_CODE,
            context
                .form()
                .setError("messageOtpAuthCodeExpired")
                .createErrorPage(Response.Status.BAD_REQUEST));
      } else {
        // valid
        context.success();
      }
    } else {
      // invalid
      AuthenticationExecutionModel execution = context.getExecution();
      if (execution.isRequired()) {
        context.failureChallenge(
            AuthenticationFlowError.INVALID_CREDENTIALS,
            context
                .form()
                .setAttribute("realm", context.getRealm())
                .setError("messageOtpAuthCodeInvalid")
                .createForm(TPL_CODE));
      } else if (execution.isConditional() || execution.isAlternative()) {
        context.attempted();
      }
    }
  }

  @Override
  public boolean requiresUser() {
    log.info("requiresUser() called");
    return false;
  }

  @Override
  public boolean configuredFor(KeycloakSession session, RealmModel realm, UserModel user) {
    log.info("configuredFor() called");
    MessageOTPCredentialProvider provider = getCredentialProvider(session);
    if (provider == null || !provider.isConfiguredFor(realm, user, getType(session))) {
      return false;
    }

    Optional<AuthenticatorConfigModel> config = Utils.getConfig(realm);

    // If no configuration is found, fall back to default behavior
    if (!config.isPresent() && user != null) {
      return user.getFirstAttribute(MOBILE_NUMBER_FIELD) != null;
    }
    boolean deferredUser =
        config.get().getConfig().get(Utils.DEFERRED_USER_ATTRIBUTE).equals("true");
    String mobileNumber = null;
    String emailAddress = null;

    if (deferredUser) {
      AuthenticationSessionModel authSession = session.getContext().getAuthenticationSession();
      String mobileNumberAttribute = config.get().getConfig().get(Utils.TEL_USER_ATTRIBUTE);
      mobileNumber = authSession.getAuthNote(mobileNumberAttribute);
      emailAddress = authSession.getAuthNote("email");
    } else if (user != null) {
      mobileNumber = Utils.getMobile(config.get(), user);
      emailAddress = user.getEmail();
    }

    return mobileNumber != null || emailAddress != null;
  }

  @Override
  public void setRequiredActions(KeycloakSession session, RealmModel realm, UserModel user) {
    log.info("setRequiredActions() called");
  }

  @Override
  public void close() {}
}
