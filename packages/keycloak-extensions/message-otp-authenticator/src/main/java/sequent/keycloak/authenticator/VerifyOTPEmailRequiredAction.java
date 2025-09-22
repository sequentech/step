// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.Response;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.Config;
import org.keycloak.authentication.InitiatedActionSupport;
import org.keycloak.authentication.RequiredActionContext;
import org.keycloak.authentication.RequiredActionFactory;
import org.keycloak.authentication.RequiredActionProvider;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;

/** Required Action that requires users to verify its email using an OTP. */
@AutoService(RequiredActionFactory.class)
@JBossLog
public class VerifyOTPEmailRequiredAction implements RequiredActionFactory, RequiredActionProvider {
  public static final String PROVIDER_ID = "verify-email-otp-ra";
  private static final String TPL_CODE = "message-otp.login.ftl";

  @Override
  public InitiatedActionSupport initiatedActionSupport() {
    return InitiatedActionSupport.SUPPORTED;
  }

  @Override
  public void evaluateTriggers(RequiredActionContext context) {
    log.info("evaluateTriggers()");
    // self registering if user doesn't have already one out of the
    // configured credential types
    UserModel user = context.getUser();
    AuthenticationSessionModel authSession = context.getAuthenticationSession();

    if (authSession.getRequiredActions().stream().noneMatch(PROVIDER_ID::equals)
        && user.getRequiredActionsStream().noneMatch(PROVIDER_ID::equals)
        && !user.isEmailVerified()) {
      log.info("evaluateTriggers(): adding required action to the user");
      user.addRequiredAction(PROVIDER_ID);
    }
  }

  @Override
  public void requiredActionChallenge(RequiredActionContext context) {
    initiateForm(context);
  }

  @Override
  public void processAction(RequiredActionContext context) {
    log.info("action() called");
    String resend = context.getHttpRequest().getDecodedFormParameters().getFirst("resend");
    if (resend != null && resend.equals("true")) {
      initiateForm(context);
      return;
    }
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    AuthenticatorConfigModel config = Utils.getConfig(authSession.getRealm()).get();
    boolean isOtl = config.getConfig().get(Utils.ONE_TIME_LINK).equals("true");
    String code = authSession.getAuthNote(Utils.CODE);
    String ttl = authSession.getAuthNote(Utils.CODE_TTL);
    String codeLength = config.getConfig().get(Utils.CODE_LENGTH);
    UserModel user = context.getUser();
    String resendTimer = config.getConfig().get(Utils.RESEND_ACTIVATION_TIMER);
    if (code == null || ttl == null) {
      context.failure();
      context.challenge(context.form().createErrorPage(Response.Status.INTERNAL_SERVER_ERROR));
      return;
    }

    // If it's an OTL, the user should never execute an action
    if (isOtl) {
      context.failure();
      context.challenge(
          context
              .form()
              .setError("messageOtp.auth.codeWithOtl")
              .createErrorPage(Response.Status.BAD_REQUEST));
      return;
    }

    String enteredCode = context.getHttpRequest().getDecodedFormParameters().getFirst(Utils.CODE);
    boolean isValid = Utils.constantTimeIsEqual(enteredCode.getBytes(), code.getBytes());
    if (isValid) {
      context.getAuthenticationSession().removeAuthNote(Utils.CODE);
      if (Long.parseLong(ttl) < System.currentTimeMillis()) {
        // expired
        context.failure();
        context.challenge(
            context
                .form()
                .setError("messageOtp.auth.codeExpired")
                .createErrorPage(Response.Status.BAD_REQUEST));
      } else {
        // valid
        user.setEmailVerified(true);
        user.removeRequiredAction(PROVIDER_ID);
        context.success();
      }
    } else {
      // invalid
      context.failure();
      context.challenge(
          context
              .form()
              .setAttribute("realm", context.getRealm())
              .setAttribute("codeJustSent", true)
              .setAttribute("ttl", ttl)
              .setAttribute("user", context.getUser())
              .setAttribute("isOtl", isOtl)
              .setAttribute("resendTimer", resendTimer)
              .setAttribute("codeLength", codeLength)
              .setError("messageOtp.auth.codeInvalid")
              .createForm(TPL_CODE));
    }
  }

  private void initiateForm(RequiredActionContext context) {
    KeycloakSession session = context.getSession();
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    String sessionId = context.getAuthenticationSession().getParentSession().getId();
    AuthenticatorConfigModel config = Utils.getConfig(authSession.getRealm()).get();
    String resendTimer = config.getConfig().get(Utils.RESEND_ACTIVATION_TIMER);
    boolean isOtl = config.getConfig().get(Utils.ONE_TIME_LINK).equals("true");
    String codeLength = config.getConfig().get(Utils.CODE_LENGTH);
    // initial form
    LoginFormsProvider form =
        context
            .form()
            .setAttribute("realm", context.getRealm())
            .setAttribute("codeJustSent", true)
            .setAttribute("user", context.getUser())
            .setAttribute("isOtl", isOtl)
            .setAttribute("ttl", config.getConfig().get(Utils.CODE_TTL))
            .setAttribute("codeLength", codeLength)
            .setAttribute("resendTimer", resendTimer);

    try {
      UserModel user = context.getUser();
      Utils.sendCode(
          config,
          session,
          user,
          authSession,
          Utils.MessageCourier.EMAIL,
          /* deferred user */ false,
          isOtl,
          new String[0],
          context);
      context.challenge(
          form.setAttribute(
                  "address",
                  Utils.getOtpAddress(Utils.MessageCourier.EMAIL, false, config, authSession, user))
              .setAttribute("ttl", config.getConfig().get(Utils.CODE_TTL))
              .setAttribute("codeJustSent", false)
              .setAttribute("resendTimer", resendTimer)
              .createForm(TPL_CODE));
    } catch (Exception error) {
      log.infov("there was an error {0}", error);
      context.failure();
      context.challenge(
          form.setError(Utils.ERROR_MESSAGE_NOT_SENT, sessionId)
              .createErrorPage(Response.Status.INTERNAL_SERVER_ERROR));
    }
  }

  @Override
  public String getDisplayText() {
    return "Verify Email using OTP";
  }

  @Override
  public RequiredActionProvider create(KeycloakSession session) {
    return this;
  }

  @Override
  public void init(Config.Scope config) {}

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public void close() {}

  @Override
  public String getId() {
    return PROVIDER_ID;
  }
}
