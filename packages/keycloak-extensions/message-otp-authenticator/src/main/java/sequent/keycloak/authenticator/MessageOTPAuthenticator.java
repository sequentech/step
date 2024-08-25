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
  private static final String EMAIL_VERIFIED = "Email verified";

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
    intiateForm(context);
  }

  @Override
  public void action(AuthenticationFlowContext context) {
    log.info("action() called");
    String enteredCode = context.getHttpRequest().getDecodedFormParameters().getFirst(Utils.CODE);
    String resend = context.getHttpRequest().getDecodedFormParameters().getFirst("resend");
    if (resend != null && resend.equals("true")) {
      intiateForm(context);
      return;
    }
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
        authSession.setAuthNote(EMAIL_VERIFIED, "true");
        // valid
        context.success();
      }
    } else {
      // invalid
      AuthenticatorConfigModel config = context.getAuthenticatorConfig();

      Utils.MessageCourier messageCourier =
          Utils.MessageCourier.fromString(config.getConfig().get(Utils.MESSAGE_COURIER_ATTRIBUTE));
      boolean deferredUser = config.getConfig().get(Utils.DEFERRED_USER_ATTRIBUTE).equals("true");
      AuthenticationExecutionModel execution = context.getExecution();
      UserModel user = context.getUser();
      String resendTimer = config.getConfig().get(Utils.RESEND_ACTIVATION_TIMER);
      if (resendTimer == null) {
        resendTimer = System.getenv("KC_OTP_RESEND_INTERVAL");
      }
      if (execution.isRequired()) {
        context.failureChallenge(
            AuthenticationFlowError.INVALID_CREDENTIALS,
            context
                .form()
                .setAttribute("realm", context.getRealm())
                .setError("messageOtpAuthCodeInvalid")
                .setAttribute("courier", messageCourier)
                .setAttribute(
                    "address",
                    Utils.getOtpAddress(messageCourier, deferredUser, config, authSession, user))
                .setAttribute("resendTimer", config.getConfig().get(Utils.RESEND_ACTIVATION_TIMER))
                .setAttribute("ttl", config.getConfig().get(Utils.CODE_TTL))
                .createForm(TPL_CODE));
      } else if (execution.isConditional() || execution.isAlternative()) {
        context.attempted();
      }
    }
  }

  private void intiateForm(AuthenticationFlowContext context) {
    AuthenticatorConfigModel config = context.getAuthenticatorConfig();
    KeycloakSession session = context.getSession();
    AuthenticationSessionModel authSession = context.getAuthenticationSession();

    Utils.MessageCourier messageCourier =
        Utils.MessageCourier.fromString(config.getConfig().get(Utils.MESSAGE_COURIER_ATTRIBUTE));
    boolean deferredUser = config.getConfig().get(Utils.DEFERRED_USER_ATTRIBUTE).equals("true");

    try {
      UserModel user = context.getUser();
      Utils.sendCode(config, session, user, authSession, messageCourier, deferredUser);
      log.info("OTP resent successfully");
      context.challenge(
          context
              .form()
              .setAttribute("realm", context.getRealm())
              .setAttribute("courier", messageCourier)
              .setAttribute(
                  "address",
                  Utils.getOtpAddress(messageCourier, deferredUser, config, authSession, user))
              .setAttribute("resendTimer", config.getConfig().get(Utils.RESEND_ACTIVATION_TIMER))
              .setAttribute("ttl", config.getConfig().get(Utils.CODE_TTL))
              .createForm(TPL_CODE));
    } catch (Exception error) {
      log.error("Error resending OTP", error);
      context.failureChallenge(
          AuthenticationFlowError.INTERNAL_ERROR,
          context
              .form()
              .setError("messageNotSent", error.getMessage())
              .createErrorPage(Response.Status.INTERNAL_SERVER_ERROR));
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
