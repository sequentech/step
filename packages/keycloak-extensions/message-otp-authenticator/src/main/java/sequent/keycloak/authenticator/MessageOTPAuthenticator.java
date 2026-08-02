// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import jakarta.ws.rs.core.Response;
import java.io.IOException;
import java.util.Map;
import java.util.Optional;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.CredentialValidator;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialModel;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialProvider;

@JBossLog
public class MessageOTPAuthenticator
    implements Authenticator, CredentialValidator<MessageOTPCredentialProvider> {
  public static final String MOBILE_NUMBER_FIELD = "sequent.read-only.mobile-number";
  private static final String TPL_CODE = "message-otp.login.ftl";
  private static final String EMAIL_VERIFIED = "Email verified";
  public static final String INVALID_CODE = "invalid otp Code";
  public static final String EXPIRED_CODE = "Code expired";
  public static final String INTERNAL_ERROR = "InternalError";

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
    intiateForm(context, /*resend*/ false);
  }

  @Override
  public void action(AuthenticationFlowContext context) {
    log.info("action() called");
    String sessionId = context.getAuthenticationSession().getParentSession().getId();
    String resend = context.getHttpRequest().getDecodedFormParameters().getFirst("resend");
    UserModel user = context.getUser();
    Utils.buildEventDetails(context, this.getClass().getSimpleName());

    if (resend != null && resend.equals("true")) {
      intiateForm(context, /*resend*/ true);
      return;
    }

    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    AuthenticatorConfigModel config = context.getAuthenticatorConfig();
    Map<String, String> configMap = MessageOTPAuthenticatorFactory.getConfigMap(config);
    boolean isOtl = "true".equals(configMap.get(Utils.ONE_TIME_LINK));
    boolean deferredUser = "true".equals(configMap.get(Utils.DEFERRED_USER_ATTRIBUTE));
    KeycloakSession session = context.getSession();

    String code = authSession.getAuthNote(Utils.CODE);
    String ttl = authSession.getAuthNote(Utils.CODE_TTL);

    boolean isTestMode = "true".equals(configMap.get(Utils.TEST_MODE_ATTRIBUTE));
    String testModeCode = configMap.get(Utils.TEST_MODE_CODE_ATTRIBUTE);

    try {
      if (code == null || ttl == null) {
        context.getEvent().error(INTERNAL_ERROR + " Missing ttl or code configurations");
        context.failureChallenge(
            AuthenticationFlowError.INTERNAL_ERROR,
            context
                .form()
                .setError("code_id:" + sessionId)
                .createErrorPage(Response.Status.INTERNAL_SERVER_ERROR));
        return;
      }

      // If it's an OTL, the user should never execute an action
      if (isOtl) {
        AuthenticationExecutionModel execution = context.getExecution();
        if (execution.isRequired()) {
          context.failureChallenge(
              AuthenticationFlowError.ACCESS_DENIED,
              context
                  .form()
                  .setError(
                      context.form().getMessage("messageOtp.auth.codeWithOtl")
                          + "<br><br>code_id: "
                          + sessionId)
                  .createErrorPage(Response.Status.BAD_REQUEST));

          return;
        } else if (execution.isConditional() || execution.isAlternative()) {
          context.attempted();
        }
      }

      String enteredCode = context.getHttpRequest().getDecodedFormParameters().getFirst(Utils.CODE);
      boolean isValid = Utils.constantTimeIsEqual(enteredCode.getBytes(), code.getBytes());
      boolean isValidTestMode = isTestMode && testModeCode.equals(enteredCode);
      Utils.MessageCourier messageCourier =
          Utils.MessageCourier.fromString(configMap.get(Utils.MESSAGE_COURIER_ATTRIBUTE));
      if (isValidTestMode || isValid) {
        context.getAuthenticationSession().removeAuthNote(Utils.CODE);
        if (Long.parseLong(ttl) < System.currentTimeMillis()) {
          // expired
          context.getEvent().error(EXPIRED_CODE);
          context.failureChallenge(
              AuthenticationFlowError.EXPIRED_CODE,
              context
                  .form()
                  .setError(
                      context
                          .form()
                          .getMessage(
                              "messageOtp.auth.codeExpired" + "<br><br>code_id: " + sessionId))
                  .createErrorPage(Response.Status.BAD_REQUEST));
          Utils.sendFeedback(
              config,
              session,
              user,
              authSession,
              messageCourier,
              /* success */ false,
              deferredUser,
              isOtl);

        } else {
          // Set email as verified in the auth note only if we actually verified
          // the email or email and/or sms
          if (messageCourier == Utils.MessageCourier.BOTH
              || messageCourier == Utils.MessageCourier.EMAIL) {
            authSession.setAuthNote(EMAIL_VERIFIED, "true");
          }

          // If the user doesn't have a MessageOTPCredential yet, create one now
          // so that on subsequent logins the authenticator is "configured" and
          // appears alongside other ALTERNATIVE authenticators (e.g. passkey)
          // in the credential chooser. This avoids the need for a separate
          // `message-otp-ra` required action on first login, which would result
          // in the user receiving two OTP codes.
          if (!deferredUser && user != null) {
            MessageOTPCredentialProvider credentialProvider = getCredentialProvider(session);
            if (!credentialProvider.isConfiguredFor(
                context.getRealm(), user, credentialProvider.getType())) {
              log.info("Creating MessageOTPCredential for user on successful authentication");
              credentialProvider.createCredential(
                  context.getRealm(), user, MessageOTPCredentialModel.create(/* isSetup= */ true));
            }
          }

          // valid
          context.getEvent().success();
          context.success();

          Utils.sendFeedback(
              config,
              session,
              user,
              authSession,
              messageCourier,
              /* success */ false,
              deferredUser,
              isOtl);
        }
      } else {
        // invalid

        context
            .getEvent()
            .error(INVALID_CODE + " code input: " + enteredCode + " code should be: " + code);

        AuthenticationExecutionModel execution = context.getExecution();
        String codeLength = configMap.get(Utils.CODE_LENGTH);
        String resendTimer = configMap.get(Utils.RESEND_ACTIVATION_TIMER);
        if (resendTimer == null) {
          resendTimer = System.getenv("KC_OTP_RESEND_INTERVAL");
        }
        if (execution.isRequired()) {
          context.failureChallenge(
              AuthenticationFlowError.INVALID_CREDENTIALS,
              context
                  .form()
                  .setError(
                      context.form().getMessage("messageOtp.auth.codeInvalid")
                          + "<br><br>code_id: "
                          + sessionId)
                  .setAttribute("realm", context.getRealm())
                  .setAttribute("courier", messageCourier)
                  .setAttribute("isOtl", isOtl)
                  .setAttribute("codeJustSent", false)
                  .setAttribute(
                      "address",
                      Utils.getOtpAddress(messageCourier, deferredUser, config, authSession, user))
                  .setAttribute("resendTimer", configMap.get(Utils.RESEND_ACTIVATION_TIMER))
                  .setAttribute("ttl", configMap.get(Utils.CODE_TTL))
                  .setAttribute("codeLength", codeLength)
                  .createForm(TPL_CODE));

          Utils.sendFeedback(
              config,
              session,
              user,
              authSession,
              messageCourier,
              /* success */ false,
              deferredUser,
              isOtl);

        } else if (execution.isConditional() || execution.isAlternative()) {
          context.attempted();
        }
      }

    } catch (IOException error) {
      log.error("Error verifying OTP", error);
      context.failureChallenge(
          AuthenticationFlowError.INTERNAL_ERROR,
          context
              .form()
              .setError(Utils.ERROR_MESSAGE_NOT_SENT, sessionId)
              .createErrorPage(Response.Status.INTERNAL_SERVER_ERROR));
    }
  }

  private void intiateForm(AuthenticationFlowContext context, boolean resend) {
    AuthenticatorConfigModel config = context.getAuthenticatorConfig();
    Map<String, String> configMap = MessageOTPAuthenticatorFactory.getConfigMap(config);
    KeycloakSession session = context.getSession();
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    String sessionId = context.getAuthenticationSession().getParentSession().getId();
    Utils.MessageCourier messageCourier =
        Utils.MessageCourier.fromString(configMap.get(Utils.MESSAGE_COURIER_ATTRIBUTE));
    boolean deferredUser = "true".equals(configMap.get(Utils.DEFERRED_USER_ATTRIBUTE));
    boolean codeJustSent = false;
    UserModel user = context.getUser();
    // `requiresUser()` returns false so Keycloak may invoke this authenticator
    // before a user has been identified (e.g. during pre-evaluation of the
    // alternatives in a sub-flow). In that case there is no mobile number or
    // email to send the OTP to, so mark this attempt as not applicable and let
    // Keycloak move on to the next alternative (typically the passkey
    // authenticator). This prevents a NullPointerException deep inside
    // Utils.getMobile / Utils.getEmailAddress.
    if (!deferredUser && user == null) {
      log.info("intiateForm(): user is null and not in deferred mode -> attempted()");
      context.attempted();
      return;
    }
    Utils.buildEventDetails(context, this.getClass().getSimpleName());
    // handle OTL
    boolean isOtl = "true".equals(configMap.get(Utils.ONE_TIME_LINK));
    String otlAuthNotesToRestore = configMap.get(Utils.OTL_RESTORED_AUTH_NOTES_ATTRIBUTE);
    String[] otlAuthNoteNames =
        otlAuthNotesToRestore == null ? new String[0] : otlAuthNotesToRestore.split(",");
    String otlVisited = authSession.getAuthNote(Utils.OTL_VISITED);
    if (!resend && isOtl && otlVisited != null && otlVisited.equals("true")) {
      log.info("OTL visited = true -> context.success()");
      context.success();
      return;
    }

    LoginFormsProvider form =
        context
            .form()
            .setAttribute("realm", context.getRealm())
            .setAttribute("courier", messageCourier)
            .setAttribute("isOtl", isOtl)
            .setAttribute("ttl", configMap.get(Utils.CODE_TTL));

    try {
      // if we have a code in the session and it has not expired, then we don't
      // resend the message
      String code = authSession.getAuthNote(Utils.CODE);
      String resendTimer = configMap.get(Utils.RESEND_ACTIVATION_TIMER);
      String configTtl = configMap.get(Utils.CODE_TTL);
      String ttl = authSession.getAuthNote(Utils.CODE_TTL);
      String codeLength = configMap.get(Utils.CODE_LENGTH);
      long currentTime = System.currentTimeMillis();
      log.info(
          "code="
              + code
              + ", ttl="
              + ttl
              + ", configTtl="
              + configTtl
              + ", resendTimer="
              + resendTimer
              + ", isOtl="
              + isOtl
              + ", currentTime="
              + currentTime);
      boolean allowResend = false;
      if (ttl != null && configTtl != null && resendTimer != null) {
        long initDate = Long.parseLong(ttl) - Long.parseLong(configTtl) * 1000L;
        long resendDate = initDate + Long.parseLong(resendTimer);
        allowResend = resendDate < currentTime;
        log.info(
            "allowResend=" + allowResend + ", initDate=" + initDate + ", resendDate=" + resendDate);
      } else {
        log.info("allowResend IS FALSE");
      }

      if ((!resend && ((code == null && !isOtl) || ttl == null)) || (resend && allowResend)) {
        log.info("Send code from InitiateForm");
        Utils.sendCode(
            config,
            session,
            user,
            authSession,
            messageCourier,
            deferredUser,
            isOtl,
            otlAuthNoteNames,
            context);
        context
            .getEvent()
            .detail("action", "send_code via " + messageCourier)
            .detail("is_resend", String.valueOf(resend))
            .success();
        codeJustSent = true;
        // after sending the code, we have a new ttl
        ttl = authSession.getAuthNote(Utils.CODE_TTL);
        log.info("OTP resent successfully");
      } else {
        log.info("OTP not resent because we had another one already");
      }

      context.challenge(
          form.setAttribute(
                  "address",
                  Utils.getOtpAddress(messageCourier, deferredUser, config, authSession, user))
              .setAttribute("resendTimer", configMap.get(Utils.RESEND_ACTIVATION_TIMER))
              .setAttribute("codeJustSent", codeJustSent)
              .setAttribute("codeLength", codeLength)
              .createForm(TPL_CODE));
    } catch (Exception error) {
      log.error("Error resending OTP", error);
      context.failureChallenge(
          AuthenticationFlowError.INTERNAL_ERROR,
          context
              .form()
              .setError(Utils.ERROR_MESSAGE_NOT_SENT, sessionId)
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
    Optional<AuthenticatorConfigModel> config = Utils.getConfig(realm);
    Map<String, String> configMap =
        MessageOTPAuthenticatorFactory.getConfigMap(config.orElse(null));
    boolean deferredUser = "true".equals(configMap.get(Utils.DEFERRED_USER_ATTRIBUTE));

    String mobileNumber = null;
    String emailAddress = null;
    if (deferredUser) {
      AuthenticationSessionModel authSession = session.getContext().getAuthenticationSession();
      String mobileNumberAttribute = configMap.get(Utils.TEL_USER_ATTRIBUTE);
      mobileNumber = authSession.getAuthNote(mobileNumberAttribute);
      emailAddress = authSession.getAuthNote("email");
    } else if (user != null) {
      mobileNumber = Utils.getMobile(config.orElse(null), user);
      emailAddress = config.isPresent() ? user.getEmail() : null;
    }
    boolean hasOtpAddress = mobileNumber != null || emailAddress != null;

    // In deferred mode the OTP address comes from the auth session notes (set during
    // deferred registration/login), not from a stored credential, so a
    // MessageOTPCredential must not be required. Otherwise, deferred users (which
    // never get a credential created for them) would silently be filtered out of the
    // authentication selection list, failing the flow with `invalid_user_credentials`.
    if (!deferredUser) {
      if (user == null) {
        return false;
      }
      MessageOTPCredentialProvider provider = getCredentialProvider(session);
      if (provider == null) {
        return false;
      }
      if (!provider.isConfiguredFor(realm, user, getType(session))) {
        // If enabled, automatically create the message-otp credential for users
        // that have a mobile phone number or email address configured, so that
        // imported/edited voters don't need a credential enrollment required action.
        boolean autoCreateCredential =
            "true".equals(configMap.get(Utils.AUTO_CREATE_CREDENTIAL_ATTRIBUTE));
        if (!autoCreateCredential || !hasOtpAddress) {
          return false;
        }
        log.info("Auto-creating MessageOTPCredential for user with configured mobile/email");
        provider.createCredential(
            realm, user, MessageOTPCredentialModel.create(/* isSetup= */ true));
      }
    }

    return hasOtpAddress;
  }

  @Override
  public void setRequiredActions(KeycloakSession session, RealmModel realm, UserModel user) {
    log.info("setRequiredActions() called");
  }

  @Override
  public void close() {}
}
