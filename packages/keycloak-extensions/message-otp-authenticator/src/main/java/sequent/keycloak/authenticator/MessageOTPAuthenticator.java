// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.Response;
import java.io.IOException;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
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
import sequent.keycloak.authenticator.messaging.MessageAttemptState;
import sequent.keycloak.authenticator.messaging.MessageChannel;
import sequent.keycloak.authenticator.messaging.MessageSenderProvider;
import sequent.keycloak.authenticator.messaging.MessagingAttributes;
import sequent.keycloak.authenticator.messaging.MessengerLinkStatus;
import sequent.keycloak.authenticator.messaging.PublicMessagingChannels;
import sequent.keycloak.authenticator.messaging.VoterChannels;

@JBossLog
public class MessageOTPAuthenticator
    implements Authenticator, CredentialValidator<MessageOTPCredentialProvider> {
  public static final String MOBILE_NUMBER_FIELD = "sequent.read-only.mobile-number";
  private static final String TPL_CODE = "message-otp.login.ftl";
  private static final String EMAIL_VERIFIED = "Email verified";
  public static final String INVALID_CODE = "invalid otp Code";
  public static final String EXPIRED_CODE = "Code expired";
  public static final String TOO_MANY_ATTEMPTS = "Too many code attempts";
  public static final String MESSENGER_NOT_CONFIRMED = "Messenger link not confirmed";
  public static final String NO_CHANNEL = "No channel to send the code";
  public static final String INTERNAL_ERROR = "InternalError";

  /** Form parameter asking for a code on another of the voter's channels. */
  public static final String CHANNEL_PARAM = "channel";

  /** Form parameter asking whether the voter interacted with the Messenger Page yet. */
  public static final String MESSENGER_STATUS_PARAM = "messengerStatus";

  /** What the code page shows: the code entry, or the choice of channel before any code. */
  public enum OtpView {
    CODE,
    CHOOSE
  }

  private enum FormRequest {
    SHOW,
    RESEND,
    SWITCH_CHANNEL,
    CHECK_MESSENGER
  }

  /** The channels a voter may get the code on, and the one in use. */
  record ChannelChoice(
      List<MessageChannel> eligible,
      Optional<MessageChannel> current,
      Map<MessageChannel, String> contacts,
      Optional<PublicMessagingChannels> projection) {}

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
    intiateForm(context, FormRequest.SHOW, null);
  }

  @Override
  public void action(AuthenticationFlowContext context) {
    log.info("action() called");
    String sessionId = context.getAuthenticationSession().getParentSession().getId();
    MultivaluedMap<String, String> parameters = context.getHttpRequest().getDecodedFormParameters();
    String resend = parameters.getFirst("resend");
    UserModel user = context.getUser();
    Utils.buildEventDetails(context, this.getClass().getSimpleName());

    if (resend != null && resend.equals("true")) {
      intiateForm(context, FormRequest.RESEND, null);
      return;
    }
    String requestedChannel = parameters.getFirst(CHANNEL_PARAM);
    if (requestedChannel != null) {
      intiateForm(context, FormRequest.SWITCH_CHANNEL, requestedChannel);
      return;
    }
    if ("true".equals(parameters.getFirst(MESSENGER_STATUS_PARAM))) {
      intiateForm(context, FormRequest.CHECK_MESSENGER, null);
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
      if (code == null && Utils.sendFailed(authSession)) {
        context.getEvent().error(INVALID_CODE);
        Utils.MessageCourier courier =
            Utils.MessageCourier.fromString(configMap.get(Utils.MESSAGE_COURIER_ATTRIBUTE));
        LoginFormsProvider form =
            context
                .form()
                .setError(
                    context.form().getMessage("messageOtp.auth.codeInvalid")
                        + "<br><br>code_id: "
                        + sessionId);
        context.failureChallenge(
            AuthenticationFlowError.INVALID_CREDENTIALS,
            codeForm(context, form, configMap, courier, deferredUser, false));
        return;
      }
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
                codeForm(context, form, configMap, messageCourier, deferredUser, false));
            return;
          }

          // Set email as verified in the auth note only if we actually verified
          // the email or email and/or sms
          if (messageCourier == Utils.MessageCourier.BOTH
              || messageCourier == Utils.MessageCourier.EMAIL
              || verifiedChannel.equals(Optional.of(MessageChannel.EMAIL))) {
            authSession.setAuthNote(EMAIL_VERIFIED, "true");
          }
          verifiedChannel.ifPresent(
              channel ->
                  authSession.setAuthNote(
                      MessagingAttributes.NOTE_VERIFIED_CHANNEL, channel.name()));

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
          String errorKey =
              attemptsExhausted ? "messageOtp.auth.tooManyAttempts" : "messageOtp.auth.codeInvalid";
          LoginFormsProvider form =
              context
                  .form()
                  .setError(context.form().getMessage(errorKey) + "<br><br>code_id: " + sessionId);
          context.failureChallenge(
              AuthenticationFlowError.INVALID_CREDENTIALS,
              codeForm(context, form, configMap, messageCourier, deferredUser, false));

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

  /** The channel a correct code proves; BOTH sends to two, so it proves neither. */
  private static Optional<MessageChannel> verifiedChannel(
      Utils.MessageCourier messageCourier, AuthenticationSessionModel authSession) {
    return switch (messageCourier) {
      case EMAIL -> Optional.of(MessageChannel.EMAIL);
      case SMS -> Optional.of(MessageChannel.SMS);
      case CHOSEN ->
          MessageChannel.parse(authSession.getAuthNote(MessagingAttributes.FORM_OTP_CHANNEL));
      case BOTH, NONE -> Optional.empty();
    };
  }

  /**
   * Confirms the Messenger link once the code was verified in this session. Harvest only confirms
   * the live, unreplaced reference of this session and code. At sign-in the confirmed account must
   * be the voter's; while enrolling it is kept for when the voter is saved.
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
    if (user == null
        || !pageScopedId.equals(user.getFirstAttribute(MessagingAttributes.MESSENGER_ID))) {
      return false;
    }
    String page = user.getFirstAttribute(MessagingAttributes.MESSENGER_PAGE);
    return page == null || page.equals(status.pageId());
  }

  /**
   * Counts a wrong code against the current code. Once the configured maximum is reached the code
   * is invalidated, so the voter must request a new one, which is subject to the resend timer.
   *
   * @return true when this attempt exhausted the code
   */
  static boolean registerFailedAttempt(
      AuthenticationSessionModel authSession, Map<String, String> configMap) {
    String configuredMax = configMap.get(Utils.MAX_CODE_ATTEMPTS);
    int maxAttempts =
        Integer.parseInt(
            configuredMax == null || configuredMax.isBlank()
                ? Utils.MAX_CODE_ATTEMPTS_DEFAULT
                : configuredMax);
    String previous = authSession.getAuthNote(Utils.CODE_ATTEMPTS);
    int attempts = (previous == null ? 0 : Integer.parseInt(previous)) + 1;
    authSession.setAuthNote(Utils.CODE_ATTEMPTS, Integer.toString(attempts));
    if (attempts < maxAttempts) {
      return false;
    }
    authSession.removeAuthNote(Utils.CODE);
    return true;
  }

  static ChannelChoice channelChoice(
      KeycloakSession session,
      RealmModel realm,
      AuthenticatorConfigModel config,
      UserModel user,
      AuthenticationSessionModel authSession,
      boolean deferredUser) {
    String mobileAttribute = Utils.telUserAttribute(config);
    MessageSenderProvider sender = VoterChannels.sender(session);
    Optional<PublicMessagingChannels> projection = PublicMessagingChannels.fromRealm(realm);
    Map<MessageChannel, String> contacts;
    List<MessageChannel> eligible;
    if (deferredUser) {
      contacts = VoterChannels.enrollmentContacts(authSession, mobileAttribute);
      String offered = authSession.getAuthNote(MessagingAttributes.NOTE_OFFERED_CHANNELS);
      eligible =
          VoterChannels.eligibleForOtp(
              offered != null
                  ? VoterChannels.parseList(offered)
                  : VoterChannels.offeredForOtp(projection, Set.of()),
              sender,
              contacts,
              Optional.empty());
    } else {
      contacts = VoterChannels.contacts(user, mobileAttribute);
      eligible =
          VoterChannels.eligibleForOtp(
              VoterChannels.offeredForOtp(projection, VoterChannels.elections(user)),
              sender,
              contacts,
              Optional.of(VoterChannels.verified(user, mobileAttribute)));
    }
    Optional<MessageChannel> current =
        Optional.ofNullable(authSession)
            .flatMap(
                notes ->
                    MessageChannel.parse(notes.getAuthNote(MessagingAttributes.FORM_OTP_CHANNEL)))
            .filter(eligible::contains);
    return new ChannelChoice(eligible, current, contacts, projection);
  }

  private void intiateForm(
      AuthenticationFlowContext context, FormRequest request, String requestedChannel) {
    AuthenticatorConfigModel config = context.getAuthenticatorConfig();
    Map<String, String> configMap = MessageOTPAuthenticatorFactory.getConfigMap(config);
    KeycloakSession session = context.getSession();
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    String sessionId = context.getAuthenticationSession().getParentSession().getId();
    Utils.MessageCourier messageCourier =
        Utils.MessageCourier.fromString(configMap.get(Utils.MESSAGE_COURIER_ATTRIBUTE));
    boolean deferredUser = "true".equals(configMap.get(Utils.DEFERRED_USER_ATTRIBUTE));
    boolean resend = request == FormRequest.RESEND;
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

    try {
      // if we have a code in the session and it has not expired, then we don't
      // resend the message
      String code = authSession.getAuthNote(Utils.CODE);
      String resendTimer = configMap.get(Utils.RESEND_ACTIVATION_TIMER);
      String configTtl = configMap.get(Utils.CODE_TTL);
      String ttl = authSession.getAuthNote(Utils.CODE_TTL);
      long currentTime = System.currentTimeMillis();
      log.info(
          "ttl="
              + ttl
              + ", configTtl="
              + configTtl
              + ", resendTimer="
              + resendTimer
              + ", isOtl="
              + isOtl
              + ", currentTime="
              + currentTime);
      // A send that failed for certain delivered nothing and kept no code: it does not start the
      // resend timer, and showing the page again does not repeat it.
      boolean failedSend = Utils.sendFailed(authSession);
      boolean allowResend =
          failedSend || Utils.isResendAllowed(ttl, configTtl, resendTimer, currentTime);
      log.info("allowResend=" + allowResend);

      // A code invalidated by too many attempts is only replaced through the resend timer.
      boolean exhausted = code == null && authSession.getAuthNote(Utils.CODE_ATTEMPTS) != null;

      // A replacement code on another channel is a resend: the same timer applies, so switching
      // channels never yields more codes or attempts than resending would.
      boolean replacement = false;
      Optional<ChannelChoice> choice = Optional.empty();
      if (messageCourier == Utils.MessageCourier.CHOSEN) {
        ChannelChoice channels =
            channelChoice(session, context.getRealm(), config, user, authSession, deferredUser);
        if (channels.eligible().isEmpty()) {
          context.getEvent().error(NO_CHANNEL);
          context.failureChallenge(
              AuthenticationFlowError.INVALID_USER,
              context
                  .form()
                  .setError("messageOtp.auth.noChannel", sessionId)
                  .createErrorPage(Response.Status.BAD_REQUEST));
          return;
        }
        Optional<MessageChannel> requested =
            MessageChannel.parse(requestedChannel).filter(channels.eligible()::contains);
        if (request == FormRequest.SWITCH_CHANNEL && requested.isPresent()) {
          if (ttl == null || allowResend) {
            authSession.setAuthNote(MessagingAttributes.FORM_OTP_CHANNEL, requested.get().name());
            replacement = true;
          } else {
            log.info("Channel change refused until the resend timer elapses");
          }
        } else if (channels.current().isEmpty() && channels.eligible().size() == 1) {
          authSession.setAuthNote(
              MessagingAttributes.FORM_OTP_CHANNEL, channels.eligible().get(0).name());
        }
        if (request == FormRequest.CHECK_MESSENGER) {
          Utils.refreshMessengerState(session, authSession);
        }
        choice =
            Optional.of(
                channelChoice(
                    session, context.getRealm(), config, user, authSession, deferredUser));
      }
      boolean needsChoice = choice.map(c -> c.current().isEmpty()).orElse(false);

      boolean firstSend =
          request == FormRequest.SHOW
              && !exhausted
              && !failedSend
              && ((code == null && !isOtl) || ttl == null);
      boolean send =
          !needsChoice && (firstSend || replacement || ((resend || exhausted) && allowResend));
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
            choice
                .flatMap(ChannelChoice::current)
                .or(
                    () ->
                        MessageChannel.parse(
                            authSession.getAuthNote(MessagingAttributes.FORM_OTP_CHANNEL)))
                .map(Enum::name)
                .orElse(messageCourier.name());
        context
            .getEvent()
            .detail("action", "send_code via " + via)
            .detail("is_resend", String.valueOf(resend || replacement))
            .success();
        codeJustSent = state != MessageAttemptState.FAILED;
        log.info("OTP resent successfully");
      } else {
        log.info("OTP not resent because we had another one already");
      }

      context.challenge(
          codeForm(context, context.form(), configMap, messageCourier, deferredUser, codeJustSent));
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

  private Response codeForm(
      AuthenticationFlowContext context,
      LoginFormsProvider form,
      Map<String, String> configMap,
      Utils.MessageCourier messageCourier,
      boolean deferredUser,
      boolean codeJustSent) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    AuthenticatorConfigModel config = context.getAuthenticatorConfig();
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
        .setAttribute("codeLength", configMap.get(Utils.CODE_LENGTH));
    if (messageCourier == Utils.MessageCourier.CHOSEN) {
      ChannelChoice choice =
          channelChoice(
              context.getSession(),
              context.getRealm(),
              config,
              context.getUser(),
              authSession,
              deferredUser);
      setChannelAttributes(form, choice, authSession);
    }
    return form.createForm(TPL_CODE);
  }

  private static void setChannelAttributes(
      LoginFormsProvider form, ChannelChoice choice, AuthenticationSessionModel authSession) {
    Map<String, String> addresses = new LinkedHashMap<>();
    for (MessageChannel channel : choice.eligible()) {
      addresses.put(channel.name(), VoterChannels.mask(channel, choice.contacts().get(channel)));
    }
    form.setAttribute(
            "otpView", choice.current().isPresent() ? OtpView.CODE.name() : OtpView.CHOOSE.name())
        .setAttribute(
            "otherWayChannels",
            VoterChannels.names(
                choice.eligible().stream()
                    .filter(channel -> choice.current().map(c -> c != channel).orElse(true))
                    .toList()))
        .setAttribute("channelAddresses", addresses);
    if (choice.current().isEmpty()) {
      return;
    }
    MessageChannel channel = choice.current().get();
    form.setAttribute("channel", channel.name());
    choice
        .projection()
        .flatMap(projection -> projection.senderLabel(channel))
        .ifPresent(label -> form.setAttribute("senderLabel", label));
    String deliveryState = authSession.getAuthNote(MessagingAttributes.NOTE_DELIVERY_STATE);
    if (deliveryState != null) {
      form.setAttribute("deliveryState", deliveryState);
    }
    if (channel == MessageChannel.MESSENGER) {
      choice
          .projection()
          .flatMap(PublicMessagingChannels::messengerPage)
          .ifPresent(page -> form.setAttribute("messengerPage", page.displayName()));
      setNoteAttribute(form, "messengerLink", authSession, MessagingAttributes.NOTE_MESSENGER_LINK);
      setNoteAttribute(form, "messengerWord", authSession, MessagingAttributes.NOTE_MESSENGER_WORD);
      setNoteAttribute(
          form, "messengerState", authSession, MessagingAttributes.NOTE_MESSENGER_STATE);
    }
  }

  private static void setNoteAttribute(
      LoginFormsProvider form,
      String attribute,
      AuthenticationSessionModel authSession,
      String note) {
    String value = authSession.getAuthNote(note);
    if (value != null) {
      form.setAttribute(attribute, value);
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
      hasOtpAddress =
          !channelChoice(session, realm, config.orElse(null), user, null, false)
              .eligible()
              .isEmpty();
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
