// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import jakarta.ws.rs.core.MultivaluedMap;
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
import sequent.keycloak.authenticator.messaging.CodeChannels;
import sequent.keycloak.authenticator.messaging.MessageAttemptState;
import sequent.keycloak.authenticator.messaging.MessageChannel;
import sequent.keycloak.authenticator.messaging.MessagingAttributes;
import sequent.keycloak.authenticator.messaging.MessengerLinkStatus;
import sequent.keycloak.authenticator.messaging.MessengerQr;
import sequent.keycloak.authenticator.messaging.PublicMessagingChannels;
import sequent.keycloak.authenticator.messaging.VoterChannel;

@JBossLog
public class MessageOTPAuthenticator
    implements Authenticator, CredentialValidator<MessageOTPCredentialProvider> {
  public static final String MOBILE_NUMBER_FIELD = "sequent.read-only.mobile-number";
  private static final String TPL_CODE = "message-otp.login.ftl";
  private static final String EMAIL_VERIFIED = "Email verified";
  public static final String INVALID_CODE = "invalid otp Code";
  public static final String EXPIRED_CODE = "Code expired";
  public static final String TOO_MANY_ATTEMPTS = "Too many code attempts";

  /** "Too many attempts. Start again.": the session takes no more codes. */
  public static final String TOO_MANY_ATTEMPTS_KEY = "messageOtp.auth.tooManyAttempts";

  public static final String MESSENGER_NOT_CONFIRMED = "Messenger link not confirmed";
  public static final String NO_CHANNEL = "No channel to send the code";
  public static final String INTERNAL_ERROR = "InternalError";

  /** Form parameter asking whether the voter interacted with the Messenger Page yet. */
  public static final String MESSENGER_STATUS_PARAM = "messengerStatus";

  /** Shown when there is no live code to compare with: the voter must ask for a new one. */
  static final String NO_CODE_KEY = "messageOtp.auth.noCode";

  private enum FormRequest {
    SHOW,
    RESEND,
    CHECK_MESSENGER
  }

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
    intiateForm(context, FormRequest.SHOW);
  }

  @Override
  public void action(AuthenticationFlowContext context) {
    log.info("action() called");
    String sessionId = context.getAuthenticationSession().getParentSession().getId();
    MultivaluedMap<String, String> parameters = context.getHttpRequest().getDecodedFormParameters();
    String resend = parameters.getFirst("resend");
    UserModel user = context.getUser();
    Utils.buildEventDetails(context, this.getClass().getSimpleName());

    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    AuthenticatorConfigModel config = context.getAuthenticatorConfig();
    Map<String, String> configMap = MessageOTPAuthenticatorFactory.getConfigMap(config);
    boolean deferredUser = "true".equals(configMap.get(Utils.DEFERRED_USER_ATTRIBUTE));
    Utils.MessageCourier messageCourier =
        Utils.MessageCourier.fromString(configMap.get(Utils.MESSAGE_COURIER_ATTRIBUTE));

    // Five wrong codes finish the session: nothing it submits is checked or sent any more.
    if (Utils.attemptsExhausted(authSession, configMap)) {
      context.getEvent().error(TOO_MANY_ATTEMPTS);
      context.failureChallenge(
          AuthenticationFlowError.INVALID_CREDENTIALS,
          exhaustedForm(context, configMap, messageCourier, deferredUser));
      return;
    }
    if (resend != null && resend.equals("true")) {
      intiateForm(context, FormRequest.RESEND);
      return;
    }
    if ("true".equals(parameters.getFirst(MESSENGER_STATUS_PARAM))) {
      intiateForm(context, FormRequest.CHECK_MESSENGER);
      return;
    }

    boolean isOtl = "true".equals(configMap.get(Utils.ONE_TIME_LINK));
    KeycloakSession session = context.getSession();

    String code = authSession.getAuthNote(Utils.CODE);
    String ttl = authSession.getAuthNote(Utils.CODE_TTL);

    boolean isTestMode = "true".equals(configMap.get(Utils.TEST_MODE_ATTRIBUTE));
    String testModeCode = configMap.get(Utils.TEST_MODE_CODE_ATTRIBUTE);

    try {
      if (code == null && Utils.sendFailed(authSession)) {
        context.getEvent().error(INVALID_CODE);
        LoginFormsProvider form =
            context
                .form()
                .setError(
                    context.form().getMessage("messageOtp.auth.codeInvalid")
                        + "<br><br>code_id: "
                        + sessionId);
        context.failureChallenge(
            AuthenticationFlowError.INVALID_CREDENTIALS,
            codeForm(context, form, configMap, messageCourier, deferredUser, false, false));
        return;
      }
      if (code == null || ttl == null) {
        // No live code (none sent yet, or it was used up): the voter asks for a new one. This is
        // the voter's situation, not a server error.
        context.getEvent().error(INVALID_CODE);
        context.failureChallenge(
            AuthenticationFlowError.INVALID_CREDENTIALS,
            codeForm(
                context,
                context.form().setError(NO_CODE_KEY),
                configMap,
                messageCourier,
                deferredUser,
                false,
                false));
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
          return;
        }
      }

      String enteredCode = parameters.getFirst(Utils.CODE);
      boolean isValid =
          enteredCode != null && Utils.constantTimeIsEqual(enteredCode.getBytes(), code.getBytes());
      boolean isValidTestMode =
          isTestMode
              && testModeCode != null
              && !testModeCode.isEmpty()
              && testModeCode.equals(enteredCode);
      if (isValidTestMode || isValid) {
        context.getAuthenticationSession().removeAuthNote(Utils.CODE);
        if (Long.parseLong(ttl) < System.currentTimeMillis()) {
          // expired
          context.getEvent().error(EXPIRED_CODE);
          context.failureChallenge(
              AuthenticationFlowError.EXPIRED_CODE,
              codeForm(
                  context,
                  context
                      .form()
                      .setError(
                          context.form().getMessage("messageOtp.auth.codeExpired")
                              + "<br><br>code_id: "
                              + sessionId),
                  configMap,
                  messageCourier,
                  deferredUser,
                  false,
                  false));
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
          Optional<MessageChannel> verifiedChannel = verifiedChannel(messageCourier, authSession);
          if (verifiedChannel.equals(Optional.of(MessageChannel.MESSENGER))
              && !confirmMessenger(context, deferredUser, user)) {
            Utils.clearMessengerLink(authSession);
            context.getEvent().error(MESSENGER_NOT_CONFIRMED);
            LoginFormsProvider form =
                context
                    .form()
                    .setError(
                        context.form().getMessage("messageOtp.messenger.notConfirmed")
                            + "<br><br>code_id: "
                            + sessionId);
            context.failureChallenge(
                AuthenticationFlowError.INVALID_CREDENTIALS,
                codeForm(context, form, configMap, messageCourier, deferredUser, false, false));
            return;
          }

          // Set email as verified in the auth note only if we actually verified
          // the email or email and/or sms
          if (messageCourier == Utils.MessageCourier.BOTH
              || messageCourier == Utils.MessageCourier.EMAIL
              || verifiedChannel.equals(Optional.of(MessageChannel.EMAIL))) {
            authSession.setAuthNote(EMAIL_VERIFIED, "true");
          }
          verifiedChannel.ifPresent(channel -> recordVerified(authSession, channel));

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
              /* success */ true,
              deferredUser,
              isOtl);
        }
      } else {
        // invalid
        boolean attemptsExhausted = registerFailedAttempt(authSession, configMap);
        context.getEvent().error(attemptsExhausted ? TOO_MANY_ATTEMPTS : INVALID_CODE);

        AuthenticationExecutionModel execution = context.getExecution();
        if (execution.isRequired()) {
          Response challenge;
          if (attemptsExhausted) {
            challenge = exhaustedForm(context, configMap, messageCourier, deferredUser);
          } else {
            LoginFormsProvider form =
                context
                    .form()
                    .setError(
                        context.form().getMessage("messageOtp.auth.codeInvalid")
                            + "<br><br>code_id: "
                            + sessionId);
            challenge =
                codeForm(context, form, configMap, messageCourier, deferredUser, false, false);
          }
          context.failureChallenge(AuthenticationFlowError.INVALID_CREDENTIALS, challenge);

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
      log.error("Error verifying OTP");
      context.failureChallenge(
          AuthenticationFlowError.INTERNAL_ERROR,
          context
              .form()
              .setError(Utils.ERROR_MESSAGE_NOT_SENT, sessionId)
              .createErrorPage(Response.Status.INTERNAL_SERVER_ERROR));
    }
  }

  /**
   * The channel a correct code proves: the one the live code was sent to. BOTH sends to two, so it
   * proves neither.
   */
  private static Optional<MessageChannel> verifiedChannel(
      Utils.MessageCourier messageCourier, AuthenticationSessionModel authSession) {
    return switch (messageCourier) {
      case EMAIL -> Optional.of(MessageChannel.EMAIL);
      case SMS -> Optional.of(MessageChannel.SMS);
      case CHOSEN ->
          MessageChannel.parse(authSession.getAuthNote(MessagingAttributes.NOTE_CODE_CHANNEL));
      case BOTH, NONE -> Optional.empty();
    };
  }

  /**
   * Keeps what the correct code proved in notes only Keycloak sets: the channel and the destination
   * the code was sent to (Messenger's account is kept by {@link #confirmMessenger}).
   */
  static void recordVerified(AuthenticationSessionModel authSession, MessageChannel channel) {
    authSession.setAuthNote(MessagingAttributes.NOTE_VERIFIED_CHANNEL, channel.name());
    String destination = authSession.getAuthNote(MessagingAttributes.NOTE_CODE_DESTINATION);
    if (channel != MessageChannel.MESSENGER && destination != null && !destination.isBlank()) {
      authSession.setAuthNote(MessagingAttributes.NOTE_VERIFIED_DESTINATION, destination);
    }
  }

  /**
   * Confirms the Messenger link once the code was verified in this session. Harvest only confirms
   * the live, unreplaced reference of this session and code. At sign-in the confirmed account must
   * be the voter's, on the voter's Page; while enrolling it is kept for when the voter is saved.
   */
  private boolean confirmMessenger(
      AuthenticationFlowContext context, boolean deferredUser, UserModel user) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    Optional<MessengerLinkStatus> confirmed =
        Utils.confirmedMessengerLink(context.getSession(), authSession);
    if (confirmed.isEmpty()) {
      return false;
    }
    MessengerLinkStatus status = confirmed.get();
    String pageScopedId = status.pageScopedId();
    if (deferredUser) {
      authSession.setAuthNote(MessagingAttributes.NOTE_VERIFIED_MESSENGER_ID, pageScopedId);
      if (status.pageId() != null) {
        authSession.setAuthNote(MessagingAttributes.NOTE_VERIFIED_MESSENGER_PAGE, status.pageId());
      }
      return true;
    }
    Optional<VoterChannel> voter = user == null ? Optional.empty() : VoterChannel.of(user);
    return voter
        .filter(channel -> channel.channel() == MessageChannel.MESSENGER)
        .filter(channel -> pageScopedId.equals(channel.destination()))
        .filter(channel -> channel.messengerPage().equals(Optional.ofNullable(status.pageId())))
        .isPresent();
  }

  /**
   * Counts a wrong code against the authentication session. Asking for a new code never resets the
   * count. Once the configured maximum is reached the code is removed and the session takes no more
   * codes: the voter starts again.
   *
   * @return true when this attempt used up the session's attempts
   */
  static boolean registerFailedAttempt(
      AuthenticationSessionModel authSession, Map<String, String> configMap) {
    int attempts = Utils.codeAttempts(authSession) + 1;
    authSession.setAuthNote(Utils.CODE_ATTEMPTS, Integer.toString(attempts));
    if (attempts < Utils.maxCodeAttempts(configMap)) {
      return false;
    }
    authSession.removeAuthNote(Utils.CODE);
    return true;
  }

  private void intiateForm(AuthenticationFlowContext context, FormRequest request) {
    AuthenticatorConfigModel config = context.getAuthenticatorConfig();
    Map<String, String> configMap = MessageOTPAuthenticatorFactory.getConfigMap(config);
    KeycloakSession session = context.getSession();
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    String sessionId = context.getAuthenticationSession().getParentSession().getId();
    Utils.MessageCourier messageCourier =
        Utils.MessageCourier.fromString(configMap.get(Utils.MESSAGE_COURIER_ATTRIBUTE));
    boolean deferredUser = "true".equals(configMap.get(Utils.DEFERRED_USER_ATTRIBUTE));
    boolean resend = request == FormRequest.RESEND;
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

    try {
      // Five wrong codes finish the session: no new code, only a restart.
      if (Utils.attemptsExhausted(authSession, configMap)) {
        log.info("intiateForm(): attempts used up; no code is sent in this session");
        context.getEvent().error(TOO_MANY_ATTEMPTS);
        context.challenge(exhaustedForm(context, configMap, messageCourier, deferredUser));
        return;
      }

      // if we have a code in the session and it has not expired, then we don't
      // resend the message
      String code = authSession.getAuthNote(Utils.CODE);
      String resendTimer = configMap.get(Utils.RESEND_ACTIVATION_TIMER);
      String configTtl = configMap.get(Utils.CODE_TTL);
      String ttl = authSession.getAuthNote(Utils.CODE_TTL);
      long currentTime = System.currentTimeMillis();
      log.infov(
          "intiateForm(): request={0}, isOtl={1}, resendTimer={2}", request, isOtl, resendTimer);
      // Every send starts the resend timer, including one that failed: a refused or rate-limited
      // send must not be repeated at once.
      boolean failedSend = Utils.sendFailed(authSession);
      boolean allowResend = Utils.isResendAllowed(ttl, configTtl, resendTimer, currentTime);
      log.infov("intiateForm(): allowResend={0}", allowResend);

      if (messageCourier == Utils.MessageCourier.CHOSEN) {
        CodeChannels.Resolution target =
            deferredUser
                ? CodeChannels.forEnrollment(
                    session, context.getRealm(), authSession, Utils.telUserAttribute(config))
                : CodeChannels.forSignIn(session, context.getRealm(), user);
        if (target.refusal().isPresent()) {
          CodeChannels.Refusal refusal = target.refusal().get();
          log.infov("intiateForm(): no code sent: {0}", refusal.reason());
          context.getEvent().detail("reason", refusal.reason()).error(NO_CHANNEL);
          String channelLabel =
              voterChannelLabel(context, session, user, deferredUser, authSession);
          context.failureChallenge(
              AuthenticationFlowError.INVALID_USER,
              context
                  .form()
                  .setError(refusal.messageKey(), channelLabel)
                  .createErrorPage(Response.Status.BAD_REQUEST));
          return;
        }
        if (request == FormRequest.CHECK_MESSENGER) {
          Utils.refreshMessengerState(session, authSession);
        }
      }

      boolean firstSend =
          request == FormRequest.SHOW && !failedSend && ((code == null && !isOtl) || ttl == null);
      boolean send = firstSend || (resend && allowResend);
      boolean codeJustSent = false;
      if (send) {
        log.info("Send code from InitiateForm");
        MessageAttemptState state =
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
        authSession.setAuthNote(MessagingAttributes.NOTE_DELIVERY_STATE, state.name());
        String via =
            MessageChannel.parse(authSession.getAuthNote(MessagingAttributes.NOTE_CODE_CHANNEL))
                .map(Enum::name)
                .orElse(messageCourier.name());
        context
            .getEvent()
            .detail("action", "send_code via " + via)
            .detail("is_resend", String.valueOf(resend))
            .success();
        codeJustSent = state != MessageAttemptState.FAILED;
        log.infov("intiateForm(): code send outcome {0}", state);
      } else {
        log.info("OTP not resent because we had another one already");
      }

      context.challenge(
          codeForm(
              context,
              context.form(),
              configMap,
              messageCourier,
              deferredUser,
              codeJustSent,
              send));
    } catch (Exception error) {
      log.error("Error resending OTP");
      context.failureChallenge(
          AuthenticationFlowError.INTERNAL_ERROR,
          context
              .form()
              .setError(Utils.ERROR_MESSAGE_NOT_SENT, sessionId)
              .createErrorPage(Response.Status.INTERNAL_SERVER_ERROR));
    }
  }

  /** The display name of the voter's channel, for the refusal text; empty when there is none. */
  private static String voterChannelLabel(
      AuthenticationFlowContext context,
      KeycloakSession session,
      UserModel user,
      boolean deferredUser,
      AuthenticationSessionModel authSession) {
    Optional<MessageChannel> channel =
        deferredUser
            ? MessageChannel.parse(authSession.getAuthNote(MessagingAttributes.FORM_OTP_CHANNEL))
            : Optional.ofNullable(user).flatMap(VoterChannel::of).map(VoterChannel::channel);
    return channel.map(c -> context.form().getMessage(c.labelKey())).orElse("");
  }

  /** The code page saying the session's attempts are used up, with a link to start again. */
  private Response exhaustedForm(
      AuthenticationFlowContext context,
      Map<String, String> configMap,
      Utils.MessageCourier messageCourier,
      boolean deferredUser) {
    LoginFormsProvider form =
        context.form().setError(TOO_MANY_ATTEMPTS_KEY).setAttribute("attemptsExhausted", true);
    return codeForm(context, form, configMap, messageCourier, deferredUser, false, false);
  }

  /**
   * The code page.
   *
   * @param codeJustSent a code was sent now
   * @param resendTimerStarted a send was attempted now, sent or not: the resend countdown starts
   */
  private Response codeForm(
      AuthenticationFlowContext context,
      LoginFormsProvider form,
      Map<String, String> configMap,
      Utils.MessageCourier messageCourier,
      boolean deferredUser,
      boolean codeJustSent,
      boolean resendTimerStarted) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    AuthenticatorConfigModel config = context.getAuthenticatorConfig();
    Optional<CodeProgress> codeProgress = CodeProgress.of(context);
    form.setAttribute("realm", context.getRealm())
        .setAttribute("courier", messageCourier)
        .setAttribute("isOtl", "true".equals(configMap.get(Utils.ONE_TIME_LINK)))
        .setAttribute("ttl", configMap.get(Utils.CODE_TTL))
        .setAttribute(
            "address",
            Utils.getOtpAddress(
                messageCourier, deferredUser, config, authSession, context.getUser()))
        .setAttribute("resendTimer", configMap.get(Utils.RESEND_ACTIVATION_TIMER))
        .setAttribute("codeJustSent", codeJustSent)
        .setAttribute("resendTimerStarted", resendTimerStarted)
        .setAttribute("codeLength", configMap.get(Utils.CODE_LENGTH))
        .setAttribute("codeRequest", codeProgress.map(CodeProgress::request).orElse(null))
        .setAttribute("codeRequests", codeProgress.map(CodeProgress::requests).orElse(null));
    if (messageCourier == Utils.MessageCourier.CHOSEN) {
      setChannelAttributes(context, form, authSession);
    }
    return form.createForm(TPL_CODE);
  }

  /**
   * What the code page shows of the one channel: its name, who sends from it and, for Messenger,
   * the Page, the link, its QR code and whether the voter opened the chat. Stream V's React page
   * reads the same attributes.
   */
  private static void setChannelAttributes(
      AuthenticationFlowContext context,
      LoginFormsProvider form,
      AuthenticationSessionModel authSession) {
    Optional<MessageChannel> current =
        MessageChannel.parse(authSession.getAuthNote(MessagingAttributes.NOTE_CODE_CHANNEL));
    if (current.isEmpty()) {
      return;
    }
    MessageChannel channel = current.get();
    Optional<PublicMessagingChannels> projection =
        PublicMessagingChannels.fromRealm(context.getRealm());
    form.setAttribute("channel", channel.name());
    projection
        .flatMap(channels -> channels.senderLabel(channel))
        .ifPresent(label -> form.setAttribute("senderLabel", label));
    String deliveryState = authSession.getAuthNote(MessagingAttributes.NOTE_DELIVERY_STATE);
    if (deliveryState != null) {
      form.setAttribute("deliveryState", deliveryState);
    }
    if (channel == MessageChannel.MESSENGER) {
      setMessengerAttributes(form, authSession, projection, context.getRealm());
    }
  }

  /**
   * The Messenger page's data: {@code messengerPage} (the Page's name), {@code messengerLink} (the
   * m.me link), {@code messengerQr} (its QR code, an SVG data URI), {@code messengerState} and
   * {@code messagingOrganization}. Shared by sign-in, enrollment and the contact change.
   */
  static void setMessengerAttributes(
      LoginFormsProvider form,
      AuthenticationSessionModel authSession,
      Optional<PublicMessagingChannels> projection,
      RealmModel realm) {
    projection
        .flatMap(PublicMessagingChannels::messengerPage)
        .ifPresent(page -> form.setAttribute("messengerPage", page.displayName()));
    form.setAttribute("messagingOrganization", Utils.getRealmName(realm));
    String link = authSession.getAuthNote(MessagingAttributes.NOTE_MESSENGER_LINK);
    if (link != null) {
      form.setAttribute("messengerLink", link);
      MessengerQr.dataUri(link).ifPresent(qr -> form.setAttribute("messengerQr", qr));
    }
    String state = authSession.getAuthNote(MessagingAttributes.NOTE_MESSENGER_STATE);
    if (state != null) {
      form.setAttribute("messengerState", state);
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
    boolean chosen =
        Utils.MessageCourier.CHOSEN
            == Utils.MessageCourier.fromString(configMap.get(Utils.MESSAGE_COURIER_ATTRIBUTE));

    String mobileNumber = null;
    String emailAddress = null;
    boolean hasOtpAddress;
    if (deferredUser) {
      AuthenticationSessionModel authSession = session.getContext().getAuthenticationSession();
      String mobileNumberAttribute = configMap.get(Utils.TEL_USER_ATTRIBUTE);
      mobileNumber = authSession.getAuthNote(mobileNumberAttribute);
      emailAddress = authSession.getAuthNote("email");
      hasOtpAddress =
          mobileNumber != null
              || emailAddress != null
              || (chosen && authSession.getAuthNote(MessagingAttributes.FORM_OTP_CHANNEL) != null);
    } else if (user != null && chosen) {
      // The voter's one channel; whether it can be used now is told on the code page.
      hasOtpAddress = VoterChannel.of(user).isPresent();
    } else {
      if (user != null) {
        mobileNumber = Utils.getMobile(config.orElse(null), user);
        emailAddress = config.isPresent() ? user.getEmail() : null;
      }
      hasOtpAddress = mobileNumber != null || emailAddress != null;
    }

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
