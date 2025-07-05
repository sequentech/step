// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.Response;
import java.util.function.Consumer;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.InitiatedActionSupport;
import org.keycloak.authentication.RequiredActionContext;
import org.keycloak.authentication.RequiredActionFactory;
import org.keycloak.authentication.RequiredActionProvider;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.sessions.AuthenticationSessionModel;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialModel;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialProvider;

/**
 * RequiredActionProvider for resetting and verifying a user's email address
 * using an OTP sent to the provided email.
 *
 * <p>Flow:
 *
 * <ol>
 *   <li>Prompts the user to enter a new email address.
 *   <li>Sends an OTP to the entered email address.
 *   <li>Prompts the user to enter the OTP.
 *   <li>On successful verification, saves an email OTP credential and updates
 *       the user's email.
 * </ol>
 *
 * <p>All state is managed via AuthenticationSessionModel notes.
 */
@AutoService(RequiredActionFactory.class)
@JBossLog
public class ResetEmailOTPRequiredAction implements RequiredActionFactory, RequiredActionProvider {
  public static final String PROVIDER_ID = "email-otp-ra";
  private static final String FTL_EMAIL_ENTRY = "email-otp.enter-email.ftl";
  private static final String FTL_EMAIL_OTP = "email-otp.enter-otp.ftl";

  /**
   * Session note key for the email address being verified.
   *
   * NOTE: This NEEDS to be "email" as Utils.sendCode expects that note name.
   */
  public static final String NOTE_EMAIL_ADDRESS = "email";

  /** Session note key for the OTP code. */
  public static final String NOTE_OTP_CODE = "email-otp-code";

  /** Session note key for the OTP expiry timestamp. */
  public static final String NOTE_OTP_TTL = "email-otp-ttl";

  /**
   * Indicates this required action supports being initiated by the user or
   * admin.
   * */
  @Override
  public InitiatedActionSupport initiatedActionSupport() {
    return InitiatedActionSupport.SUPPORTED;
  }

  @Override
  public void evaluateTriggers(RequiredActionContext context) {}

  /**
   * Presents the appropriate challenge to the user: email entry or OTP entry.
   */
  @Override
  public void requiredActionChallenge(RequiredActionContext context) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    String email = authSession.getAuthNote(NOTE_EMAIL_ADDRESS);
    if (email == null) {
      // Prompt for email address
      context.challenge(createEmailEntryForm(context, null));
    } else {
      // Prompt for OTP
      context.challenge(createOTPForm(context, null));
    }
  }

  /**
   * Handles form submissions for both email entry and OTP entry. Delegates to
   * separate methods for clarity.
   */
  @Override
  public void processAction(RequiredActionContext context) {
    log.info("ResetEmailOTPRequiredAction.processAction() called");
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    String email = authSession.getAuthNote(NOTE_EMAIL_ADDRESS);
    if (email == null) {
      log.info("ResetEmailOTPRequiredAction: email is null, calling handleEmailEntry");
      handleEmailEntry(context);
    } else {
      log.info("ResetEmailOTPRequiredAction: email present, calling handleOtpEntry");
      handleOtpEntry(context, email);
    }
  }

  /** 
   * Handles the email entry step: validates, stores, and sends OTP to the
   * email.
   */
  private void handleEmailEntry(RequiredActionContext context) {
    log.info("ResetEmailOTPRequiredAction.handleEmailEntry() called");
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    KeycloakSession session = context.getSession();
    AuthenticatorConfigModel config = Utils.getConfig(context.getRealm()).orElse(null);
    String enteredEmail = context.getHttpRequest().getDecodedFormParameters().getFirst("email");
    log.info("ResetEmailOTPRequiredAction: enteredEmail=" + enteredEmail);
    if (enteredEmail == null || !enteredEmail.contains("@")) {
      log.info("ResetEmailOTPRequiredAction: invalid email, showing error");
      context.challenge(
          createEmailEntryForm(context, form -> form.setError("emailOtp.auth.invalidEmail")));
      return;
    }
    // Save email in session and send OTP using shared logic
    authSession.setAuthNote(NOTE_EMAIL_ADDRESS, enteredEmail);
    try {
      log.info("ResetEmailOTPRequiredAction: sending code");
      Utils.sendCode(
          config,
          session,
          context.getUser(),
          authSession,
          Utils.MessageCourier.EMAIL,
          /*deferredUser*/ true,
          /*isOtl*/ false,
          new String[0],
          context);
    } catch (Exception e) {
      log.error("ResetEmailOTPRequiredAction: error sending code", e);
      context.challenge(
          createEmailEntryForm(context, form -> form.setError("emailOtp.auth.sendError")));
      return;
    }
    log.info("ResetEmailOTPRequiredAction: challenge OTP form");
    context.challenge(createOTPForm(context, null));
  }

  /**
   * Handles the OTP entry step: validates the code and updates the user if
   * correct. Also allows the user to go back and change the email address.
   */
  private void handleOtpEntry(RequiredActionContext context, String email) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    KeycloakSession session = context.getSession();
    String changeEmail =
        context.getHttpRequest().getDecodedFormParameters().getFirst("changeEmail");
    if ("true".equals(changeEmail)) {
      authSession.removeAuthNote(NOTE_EMAIL_ADDRESS);
      context.challenge(createEmailEntryForm(context, null));
      return;
    }
    // Handle resend logic
    String resend = context.getHttpRequest().getDecodedFormParameters().getFirst("resend");
    AuthenticatorConfigModel config = Utils.getConfig(context.getRealm()).orElse(null);
    String code = authSession.getAuthNote(Utils.CODE);
    String ttl = authSession.getAuthNote(Utils.CODE_TTL);

    String codeLengthStr = config != null ? config.getConfig().get(Utils.CODE_LENGTH) : "6";
    int codeLength = Integer.parseInt(codeLengthStr);
    String enteredCode = context.getHttpRequest().getDecodedFormParameters().getFirst("code");
    if (resend != null && resend.equals("true")) {
      log.info("ResetEmailOTPRequiredAction: resend OTP requested");
      // Only allow resend if enough time has passed
      String resendTimerStr =
          config != null ? config.getConfig().get(Utils.RESEND_ACTIVATION_TIMER) : "60";
      long resendTimer = Long.parseLong(resendTimerStr);
      long lastSent =
          ttl != null
              ? Long.parseLong(ttl)
                  - (config != null
                      ? Long.parseLong(config.getConfig().get(Utils.CODE_TTL)) * 1000L
                      : 300000L)
              : 0;
      long now = System.currentTimeMillis();
      if (now - lastSent < resendTimer) {
        context.challenge(
            createOTPForm(context, form -> form.setError("emailOtp.auth.resend.timer")));
        return;
      }
      // Resend code
      try {
        Utils.sendCode(
            config,
            session,
            context.getUser(),
            authSession,
            Utils.MessageCourier.EMAIL,
            /*deferredUser*/ true,
            /*isOtl*/ false,
            new String[0],
            context);
      } catch (Exception e) {
        context.challenge(createOTPForm(context, form -> form.setError("emailOtp.auth.sendError")));
        return;
      }
      context.challenge(createOTPForm(context, null));
      return;
    }
    // Validate OTP
    if (enteredCode == null || code == null || ttl == null || enteredCode.length() != codeLength) {
      context.failure();
      return;
    }
    boolean isValid = Utils.constantTimeIsEqual(enteredCode.getBytes(), code.getBytes());
    if (isValid) {
      if (Long.parseLong(ttl) < System.currentTimeMillis()) {
        context.challenge(
            createOTPForm(context, form -> form.setError("emailOtp.auth.codeExpired")));
        return;
      }
      MessageOTPCredentialProvider credentialProvider = new MessageOTPCredentialProvider(session);
      credentialProvider.createCredential(
          context.getRealm(),
          context.getUser(),
          MessageOTPCredentialModel.create(/* isSetup= */ true));
      context.getUser().setEmail(email);
      context.getUser().setEmailVerified(true);
      context.getUser().removeRequiredAction(PROVIDER_ID);
      context.getAuthenticationSession().removeRequiredAction(PROVIDER_ID);
      context.success();
    } else {
      context.challenge(createOTPForm(context, form -> form.setError("emailOtp.auth.codeInvalid")));
    }
  }

  @Override
  public void close() {}

  /** Creates the form for entering the email address. */
  private Response createEmailEntryForm(
      RequiredActionContext context, Consumer<LoginFormsProvider> formConsumer) {
    LoginFormsProvider form = context.form();
    if (formConsumer != null) {
      formConsumer.accept(form);
    }
    return form.createForm(FTL_EMAIL_ENTRY);
  }

  /** Creates the form for entering the OTP code. */
  private Response createOTPForm(
      RequiredActionContext context, Consumer<LoginFormsProvider> formConsumer) {
    LoginFormsProvider form = context.form();
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    AuthenticatorConfigModel config = Utils.getConfig(context.getRealm()).orElse(null);
    String codeLength = config != null ? config.getConfig().get(Utils.CODE_LENGTH) : "6";
    String resendTimer = config != null ? config.getConfig().get(Utils.RESEND_ACTIVATION_TIMER) : "60";

    form.setAttribute("email", authSession.getAuthNote(NOTE_EMAIL_ADDRESS));
    form.setAttribute("codeLength", codeLength);
    form.setAttribute("resendTimer", resendTimer);
    form.setAttribute("ttl", config.getConfig().get(Utils.CODE_TTL));
    // codeJustSent is true if we just sent a code (for resend button state)
    form.setAttribute("codeJustSent", true); // always true after sending or entering
    if (formConsumer != null) {
      formConsumer.accept(form);
    }
    return form.createForm(FTL_EMAIL_OTP);
  }

  /**
   * Generates and sends an OTP to the given email address, storing the code and
   * expiry in the session.
   *
   * @deprecated Use Utils.sendCode instead for consistency.
   */
  @Deprecated
  private void sendEmailOTP(RequiredActionContext context, String email) throws Exception {
    // Deprecated: replaced by Utils.sendCode
    throw new UnsupportedOperationException("Use Utils.sendCode instead");
  }

  @Override
  public String getDisplayText() {
    return "Reset and Configure Email OTP";
  }

  @Override
  public RequiredActionProvider create(KeycloakSession session) {
    return this;
  }

  @Override
  public void init(org.keycloak.Config.Scope config) {}

  @Override
  public void postInit(org.keycloak.models.KeycloakSessionFactory factory) {}

  @Override
  public String getId() {
    return PROVIDER_ID;
  }
}
