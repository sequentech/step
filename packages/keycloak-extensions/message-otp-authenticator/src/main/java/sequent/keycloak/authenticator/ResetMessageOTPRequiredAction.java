// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import jakarta.ws.rs.core.Response;
import java.io.PrintWriter;
import java.io.StringWriter;
import java.util.Optional;
import java.util.function.Consumer;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.InitiatedActionSupport;
import org.keycloak.authentication.RequiredActionContext;
import org.keycloak.authentication.RequiredActionProvider;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialModel;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialProvider;

@JBossLog
public class ResetMessageOTPRequiredAction implements RequiredActionProvider {
  public static final String PROVIDER_ID = "message-otp-ra";

  public static final String IS_SETUP_FIELD = "is-setup";
  private static final String FTL_RESET_MESSAGE_OTP = "message-otp.login.ftl";

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
  public InitiatedActionSupport initiatedActionSupport() {
    return InitiatedActionSupport.SUPPORTED;
  }

  @Override
  public void evaluateTriggers(RequiredActionContext context) {}

  @Override
  public void requiredActionChallenge(RequiredActionContext context) {
    context.challenge(createForm(context, null));
  }

  @Override
  public void processAction(RequiredActionContext context) {
    log.info("action() called");

    UserModel user = context.getUser();
    String enteredCode = context.getHttpRequest().getDecodedFormParameters().getFirst(Utils.CODE);

    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    String code = authSession.getAuthNote(Utils.CODE);
    String ttl = authSession.getAuthNote(Utils.CODE_TTL);

    String resend = context.getHttpRequest().getDecodedFormParameters().getFirst("resend");
    if (resend != null && resend.equals("true")) {
      context.challenge(createForm(context, null));
      return;
    }

    if (code == null || ttl == null) {
      context.failure();
      return;
    }

    boolean isValid = Utils.constantTimeIsEqual(enteredCode.getBytes(), code.getBytes());
    if (isValid) {
      context.getAuthenticationSession().removeAuthNote(Utils.CODE);
      if (Long.parseLong(ttl) < System.currentTimeMillis()) {
        // expired
        context.challenge(
            createForm(
                context,
                form ->
                    form.setError("messageOtp.auth.codeExpired")
                        .createErrorPage(Response.Status.BAD_REQUEST)));
        return;
      }
    } else {
      // invalid
      context.challenge(
          createForm(
              context,
              form ->
                  form.setError("messageOtp.auth.codeInvalid")
                      .createErrorPage(Response.Status.BAD_REQUEST)));
      return;
    }

    // Generate a MessageOTP credential for the user and remove the required
    // action
    MessageOTPCredentialProvider credentialProvider = getCredentialProvider(context.getSession());
    credentialProvider.createCredential(
        context.getRealm(),
        context.getUser(),
        MessageOTPCredentialModel.create(/* isSetup= */ true));

    user.removeRequiredAction(PROVIDER_ID);
    context.getAuthenticationSession().removeRequiredAction(PROVIDER_ID);

    context.success();
  }

  @Override
  public void close() {}

  private Response createForm(
      RequiredActionContext context, Consumer<LoginFormsProvider> formConsumer) {
    Optional<AuthenticatorConfigModel> config = Utils.getConfig(context.getRealm());
    String codeLength = config.get().getConfig().get(Utils.CODE_LENGTH);
    KeycloakSession session = context.getSession();
    UserModel user = context.getUser();
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    String resendTimer = config.get().getConfig().get(Utils.RESEND_ACTIVATION_TIMER);
    boolean isOtl =
        Optional.ofNullable(config.get())
            .map(c -> c.getConfig())
            .map(c -> c.get(Utils.ONE_TIME_LINK))
            .map(c -> c.equals("true"))
            .orElse(false);

    try {
      Utils.sendCode(
          config.get(),
          session,
          user,
          authSession,
          Utils.MessageCourier.BOTH,
          /* deferredUser */ false,
          isOtl,
          new String[0],
          context);
    } catch (Exception error) {
      StringWriter sw = new StringWriter();
      error.printStackTrace(new PrintWriter(sw));
      log.infov("There was an error: {0}", sw.toString());
      context.failure();
    }

    LoginFormsProvider form = context.form();
    form.setAttribute("realm", context.getRealm())
        .setAttribute(
            "address",
            Utils.getOtpAddress(Utils.MessageCourier.BOTH, false, config.get(), authSession, user))
        .setAttribute("ttl", config.get().getConfig().get(Utils.CODE_TTL))
        .setAttribute("codeJustSent", true)
        .setAttribute("isOtl", isOtl)
        .setAttribute("codeLength", codeLength)
        .setAttribute("resendTimer", resendTimer);

    if (formConsumer != null) {
      formConsumer.accept(form);
    }

    return form.createForm(FTL_RESET_MESSAGE_OTP);
  }
}
