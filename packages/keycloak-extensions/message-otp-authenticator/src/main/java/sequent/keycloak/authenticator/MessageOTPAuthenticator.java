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
import org.keycloak.authentication.AuthenticatorUtil;
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
import sequent.keycloak.authenticator.messaging.MessageReason;
import sequent.keycloak.authenticator.messaging.MessagingAttributes;
import sequent.keycloak.authenticator.messaging.MessengerLinkStatus;
import sequent.keycloak.authenticator.messaging.MessengerQr;
import sequent.keycloak.authenticator.messaging.PublicMessagingChannels;
import sequent.keycloak.authenticator.messaging.VerifiedContacts;
import sequent.keycloak.authenticator.messaging.VoterChannel;

@JBossLog
public class MessageOTPAuthenticator
    implements Authenticator, CredentialValidator<MessageOTPCredentialProvider> {
  public static final String MOBILE_NUMBER_FIELD = "sequent.read-only.mobile-number";
  private static final String TPL_CODE = "message-otp.login.ftl";
  public static final String INVALID_CODE = "invalid otp Code";
  public static final String EXPIRED_CODE = "Code expired";
  public static final String TOO_MANY_ATTEMPTS = "Too many code attempts";

  /** "Too many attempts. Start again.": the session takes no more codes. */
  public static final String TOO_MANY_ATTEMPTS_KEY = "messageOtp.auth.tooManyAttempts";

  public static final String MESSENGER_NOT_CONFIRMED = "Messenger link not confirmed";
  public static final String NO_CHANNEL = "No channel to send the code";
  public static final String INTERNAL_ERROR = "InternalError";
  private static final String CODE_SEND_FAILED = "Code send failed";

  /** Form parameter asking whether the voter interacted with the Messenger Page yet. */
  public static final String MESSENGER_STATUS_PARAM = "messengerStatus";

  /** Shown when there is no live code to compare with: the voter must ask for a new one. */
  static final String NO_CODE_KEY = "messageOtp.auth.noCode";

  /** The Messenger link wasn't confirmed for this voter. */
  static final String NOT_CONFIRMED_KEY = "messageOtp.messenger.notConfirmed";

  /** A Messenger reconnect confirmed an account another voter uses (R7-10). */
  static final String ACCOUNT_IN_USE_KEY = "resetAppOtp.auth.error.maxReceiverReuse";

  /** A Messenger voter's Page changed, and this step has no password to reconnect after (R7-10). */
  static final String RECONNECT_NEEDS_PASSWORD_KEY = "messageOtp.messenger.reconnectNeedsPassword";

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
    // A voter with a messaging channel is only ever reached there, whatever the step's courier.
    Utils.MessageCourier messageCourier =
        Utils.codeCourier(
            Utils.MessageCourier.fromString(configMap.get(Utils.MESSAGE_COURIER_ATTRIBUTE)),
            user,
            deferredUser);

    String challenge = challenge(context);
    boolean neutral =
        Utils.CodePageDisclosure.of(configMap, deferredUser) == Utils.CodePageDisclosure.NEUTRAL;
    // Five wrong codes finish the challenge: nothing it submits is checked or sent any more.
    if (Utils.attemptsExhausted(authSession, configMap, challenge)) {
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
      // A neutral page tells nothing about the account: an unknown account, or a code that was
      // never sent, reads exactly like a wrong code.
      if (neutral
          && !deferredUser
          && (user == null ? isRequired(context) : (code == null || ttl == null))) {
        wrongCode(context, configMap, messageCourier, deferredUser, challenge, sessionId);
        return;
      }
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
          Optional<String> notConfirmed =
              verifiedChannel.equals(Optional.of(MessageChannel.MESSENGER))
                  ? confirmMessenger(context, deferredUser, user)
                  : Optional.empty();
          if (notConfirmed.isPresent()) {
            Utils.clearMessengerLink(authSession);
            context.getEvent().error(MESSENGER_NOT_CONFIRMED);
            LoginFormsProvider form =
                context
                    .form()
                    .setError(
                        context.form().getMessage(notConfirmed.get())
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
            authSession.setAuthNote(Utils.EMAIL_VERIFIED, "true");
          }
          // Only a code that went to the channel proves it: a test-mode code proves nothing, and a
          // realm on a configured courier (EMAIL, SMS) records no messaging channel.
          if (isValid && messageCourier == Utils.MessageCourier.CHOSEN) {
            verifiedChannel.ifPresent(channel -> recordVerified(authSession, channel));
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
              /* success */ true,
              deferredUser,
              isOtl);
        }
      } else {
        // invalid
        boolean attemptsExhausted = Utils.registerFailedAttempt(authSession, configMap, challenge);
        context.getEvent().error(attemptsExhausted ? TOO_MANY_ATTEMPTS : INVALID_CODE);

        AuthenticationExecutionModel execution = context.getExecution();
        if (execution.isRequired()) {
          Response page;
          if (attemptsExhausted) {
            page = exhaustedForm(context, configMap, messageCourier, deferredUser);
          } else {
            LoginFormsProvider form =
                context
                    .form()
                    .setError(
                        context.form().getMessage("messageOtp.auth.codeInvalid")
                            + "<br><br>code_id: "
                            + sessionId);
            page = codeForm(context, form, configMap, messageCourier, deferredUser, false, false);
          }
          context.failureChallenge(AuthenticationFlowError.INVALID_CREDENTIALS, page);

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
   * be the voter's, on the voter's Page; while enrolling it is kept for when the voter is saved. A
   * sign-in reconnect after a Page change (R7-10) saves the confirmed account when it is on the
   * event's Page and no other voter uses it.
   *
   * @return empty when confirmed; otherwise the message key the page shows
   */
  private Optional<String> confirmMessenger(
      AuthenticationFlowContext context, boolean deferredUser, UserModel user) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    Optional<MessengerLinkStatus> confirmed =
        Utils.confirmedMessengerLink(context.getSession(), authSession);
    if (confirmed.isEmpty()) {
      return Optional.of(NOT_CONFIRMED_KEY);
    }
    MessengerLinkStatus status = confirmed.get();
    String pageScopedId = status.pageScopedId();
    if (deferredUser) {
      authSession.setAuthNote(MessagingAttributes.NOTE_VERIFIED_MESSENGER_ID, pageScopedId);
      if (status.pageId() != null) {
        authSession.setAuthNote(MessagingAttributes.NOTE_VERIFIED_MESSENGER_PAGE, status.pageId());
      }
      return Optional.empty();
    }
    String reconnectPage = authSession.getAuthNote(MessagingAttributes.NOTE_MESSENGER_RECONNECT);
    if (reconnectPage != null && user != null) {
      return reconnectMessenger(context, user, status, reconnectPage);
    }
    Optional<VoterChannel> voter = user == null ? Optional.empty() : VoterChannel.of(user);
    boolean voters =
        voter
            .filter(channel -> channel.channel() == MessageChannel.MESSENGER)
            .filter(channel -> pageScopedId.equals(channel.destination()))
            .filter(channel -> channel.messengerPage().equals(Optional.ofNullable(status.pageId())))
            .isPresent();
    return voters ? Optional.empty() : Optional.of(NOT_CONFIRMED_KEY);
  }

  /**
   * Saves the account a sign-in reconnect confirmed (R7-10): only on the event's current Page, the
   * one the link opened, and only an account no other voter uses. Each refusal is logged with its
   * reason.
   */
  private Optional<String> reconnectMessenger(
      AuthenticationFlowContext context,
      UserModel user,
      MessengerLinkStatus status,
      String reconnectPage) {
    Optional<String> eventPage =
        PublicMessagingChannels.fromRealm(context.getRealm())
            .flatMap(PublicMessagingChannels::messengerPage)
            .map(PublicMessagingChannels.MessengerPage::pageId);
    if (!reconnectPage.equals(status.pageId()) || !eventPage.equals(Optional.of(reconnectPage))) {
      log.warn("confirmMessenger(): the reconnect was confirmed on another Page; nothing saved");
      context.getEvent().detail("reason", "Messenger reconnect confirmed on another Page");
      return Optional.of(NOT_CONFIRMED_KEY);
    }
    String voterId = user.getId();
    boolean usedByAnother =
        context
            .getSession()
            .users()
            .searchForUserByUserAttributeStream(
                context.getRealm(), MessagingAttributes.MESSENGER_ID, status.pageScopedId())
            .anyMatch(other -> !other.getId().equals(voterId));
    if (usedByAnother) {
      log.warn("confirmMessenger(): another voter uses the reconnected account; nothing saved");
      context.getEvent().detail("reason", "Messenger account used by another voter");
      return Optional.of(ACCOUNT_IN_USE_KEY);
    }
    VerifiedContacts.reconnectMessenger(user, status.pageScopedId(), reconnectPage);
    context.getAuthenticationSession().removeAuthNote(MessagingAttributes.NOTE_MESSENGER_RECONNECT);
    context.getEvent().detail("messenger", "reconnected");
    return Optional.empty();
  }

  /** The challenge a code belongs to: this flow execution. Its attempts are counted apart. */
  static String challenge(AuthenticationFlowContext context) {
    AuthenticationExecutionModel execution = context.getExecution();
    return execution == null ? null : execution.getId();
  }

  private static boolean isRequired(AuthenticationFlowContext context) {
    AuthenticationExecutionModel execution = context.getExecution();
    return execution != null && execution.isRequired();
  }

  /** A wrong code on a neutral page: counted, and answered like any wrong code. */
  private void wrongCode(
      AuthenticationFlowContext context,
      Map<String, String> configMap,
      Utils.MessageCourier messageCourier,
      boolean deferredUser,
      String challenge,
      String sessionId) {
    boolean exhausted =
        Utils.registerFailedAttempt(context.getAuthenticationSession(), configMap, challenge);
    context.getEvent().error(exhausted ? TOO_MANY_ATTEMPTS : INVALID_CODE);
    Response page =
        exhausted
            ? exhaustedForm(context, configMap, messageCourier, deferredUser)
            : codeForm(
                context,
                context
                    .form()
                    .setError(
                        context.form().getMessage("messageOtp.auth.codeInvalid")
                            + "<br><br>code_id: "
                            + sessionId),
                configMap,
                messageCourier,
                deferredUser,
                false,
                false);
    context.failureChallenge(AuthenticationFlowError.INVALID_CREDENTIALS, page);
  }

  /**
   * Shows the code page, sending a code when one is due. A neutral page (one that doesn't tell
   * whether an account exists) answers no sooner than its floor after the request began, with or
   * without an account (R7-06, {@link NeutralPace}).
   */
  private void intiateForm(AuthenticationFlowContext context, FormRequest request) {
    long start = NeutralPace.start();
    try {
      showForm(context, request);
    } finally {
      Map<String, String> configMap =
          MessageOTPAuthenticatorFactory.getConfigMap(context.getAuthenticatorConfig());
      boolean deferredUser = "true".equals(configMap.get(Utils.DEFERRED_USER_ATTRIBUTE));
      boolean neutral =
          Utils.CodePageDisclosure.of(configMap, deferredUser) == Utils.CodePageDisclosure.NEUTRAL;
      if (neutral && !deferredUser && (context.getUser() != null || isRequired(context))) {
        NeutralPace.hold(start, NeutralPace.floor(configMap));
      }
    }
  }

  private void showForm(AuthenticationFlowContext context, FormRequest request) {
    AuthenticatorConfigModel config = context.getAuthenticatorConfig();
    Map<String, String> configMap = MessageOTPAuthenticatorFactory.getConfigMap(config);
    KeycloakSession session = context.getSession();
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    String sessionId = context.getAuthenticationSession().getParentSession().getId();
    boolean deferredUser = "true".equals(configMap.get(Utils.DEFERRED_USER_ATTRIBUTE));
    boolean resend = request == FormRequest.RESEND;
    UserModel user = context.getUser();
    // A voter with a messaging channel is only ever reached there, whatever the step's courier.
    Utils.MessageCourier messageCourier =
        Utils.codeCourier(
            Utils.MessageCourier.fromString(configMap.get(Utils.MESSAGE_COURIER_ATTRIBUTE)),
            user,
            deferredUser);
    // `requiresUser()` returns false so Keycloak may invoke this authenticator
    // before a user has been identified (e.g. during pre-evaluation of the
    // alternatives in a sub-flow). In that case there is no mobile number or
    // email to send the OTP to, so mark this attempt as not applicable and let
    // Keycloak move on to the next alternative (typically the passkey
    // authenticator). This prevents a NullPointerException deep inside
    // Utils.getMobile / Utils.getEmailAddress.
    boolean neutral =
        Utils.CodePageDisclosure.of(configMap, deferredUser) == Utils.CodePageDisclosure.NEUTRAL;
    if (!deferredUser && user == null) {
      // An optional step (an alternative being evaluated before the user is known) still lets
      // Keycloak move on; a required step answers like it would for an account.
      if (neutral && isRequired(context)) {
        // A neutral page looks the same whether or not the account exists; nothing is sent.
        log.info("intiateForm(): no account; the neutral code page is shown and nothing is sent");
        neutralNothingSent(
            context, request, configMap, messageCourier, "No account for the identifier");
        return;
      }
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
      // Five wrong codes finish the challenge: no new code, only a restart.
      if (Utils.attemptsExhausted(authSession, configMap, challenge(context))) {
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

      Utils.MessengerLinkPolicy messengerLink = Utils.MessengerLinkPolicy.VOTERS_ACCOUNT;
      if (messageCourier == Utils.MessageCourier.CHOSEN) {
        CodeChannels.Resolution target =
            deferredUser
                ? CodeChannels.forEnrollment(
                    session, context.getRealm(), authSession, Utils.telUserAttribute(config))
                : CodeChannels.forSignIn(session, context.getRealm(), user);
        if (target.refusal().isPresent()) {
          CodeChannels.Refusal refusal = target.refusal().get();
          log.infov("intiateForm(): no code sent: {0}", refusal.reason());
          if (neutral && !deferredUser) {
            // The refusal names the voter's channel: a neutral page doesn't. It takes the same
            // steps
            // as for no account, the resend timer included (R7-07).
            neutralNothingSent(context, request, configMap, messageCourier, refusal.reason());
            return;
          }
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
        Optional<String> newPage =
            deferredUser
                ? Optional.empty()
                : Utils.messengerPageChange(context.getRealm(), target.channel().get());
        if (newPage.isPresent()) {
          // R7-10: the voter's Messenger account is on a Page the event no longer uses.
          if (neutral) {
            log.infov(
                "intiateForm(): no code sent: {0}", MessageReason.MESSENGER_PAGE_CHANGED.reason());
            neutralNothingSent(
                context,
                request,
                configMap,
                messageCourier,
                MessageReason.MESSENGER_PAGE_CHANGED.reason());
            return;
          }
          if (!AuthenticatorUtil.isPasswordValidated(authSession)) {
            log.infov(
                "intiateForm(): no code sent: {0}",
                MessageReason.MESSENGER_RECONNECT_NEEDS_PASSWORD.reason());
            context
                .getEvent()
                .detail("reason", MessageReason.MESSENGER_RECONNECT_NEEDS_PASSWORD.reason())
                .error(NO_CHANNEL);
            context.failureChallenge(
                AuthenticationFlowError.INVALID_USER,
                context
                    .form()
                    .setError(RECONNECT_NEEDS_PASSWORD_KEY)
                    .createErrorPage(Response.Status.BAD_REQUEST));
            return;
          }
          log.info("intiateForm(): the voter's Messenger Page changed; reconnecting at sign-in");
          context.getEvent().detail("messenger", "reconnect");
          messengerLink = Utils.MessengerLinkPolicy.RECONNECT;
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
                context,
                messengerLink);
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
        // A neutral page reads the same whatever happened to the send.
        codeJustSent = neutral || state != MessageAttemptState.FAILED;
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
      if (neutral && !deferredUser) {
        // A failed provider call must not reveal that the identifier has an account.
        authSession.removeAuthNote(Utils.CODE);
        Utils.clearMessengerLink(authSession);
        authSession.setAuthNote(
            MessagingAttributes.NOTE_DELIVERY_STATE, MessageAttemptState.FAILED.name());
        if (authSession.getAuthNote(Utils.CODE_TTL) == null) {
          long lifetime = Long.parseLong(configMap.get(Utils.CODE_TTL)) * 1000L;
          authSession.setAuthNote(
              Utils.CODE_TTL, Long.toString(System.currentTimeMillis() + lifetime));
        }
        String reason =
            MessageReason.SMS_PROVIDER_SENDS_ONLY_CODES.reason().equals(error.getMessage())
                ? MessageReason.SMS_PROVIDER_SENDS_ONLY_CODES.reason()
                : CODE_SEND_FAILED;
        context.getEvent().detail("reason", reason).error(NO_CHANNEL);
        context.challenge(
            codeForm(
                context,
                context.form(),
                configMap,
                messageCourier,
                false,
                send(request),
                send(request)));
        return;
      }
      context.failureChallenge(
          AuthenticationFlowError.INTERNAL_ERROR,
          context
              .form()
              .setError(Utils.ERROR_MESSAGE_NOT_SENT, sessionId)
              .createErrorPage(Response.Status.INTERNAL_SERVER_ERROR));
    }
  }

  private static boolean send(FormRequest request) {
    return request == FormRequest.SHOW || request == FormRequest.RESEND;
  }

  /**
   * The neutral code page when nothing is sent: an identifier with no account, or an account whose
   * channel is refused. The same page, the same resend timer ({@link Utils#CODE_TTL}) and the same
   * attempt limit as for an account whose code was sent (R3-04, R7-07); the reason goes to the
   * event only.
   */
  private void neutralNothingSent(
      AuthenticationFlowContext context,
      FormRequest request,
      Map<String, String> configMap,
      Utils.MessageCourier messageCourier,
      String reason) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    if (Utils.attemptsExhausted(authSession, configMap, challenge(context))) {
      context.getEvent().error(TOO_MANY_ATTEMPTS);
      context.challenge(exhaustedForm(context, configMap, messageCourier, false));
      return;
    }
    String ttl = authSession.getAuthNote(Utils.CODE_TTL);
    boolean allowResend =
        Utils.isResendAllowed(
            ttl,
            configMap.get(Utils.CODE_TTL),
            configMap.get(Utils.RESEND_ACTIVATION_TIMER),
            System.currentTimeMillis());
    boolean send =
        (request == FormRequest.SHOW && ttl == null)
            || (request == FormRequest.RESEND && allowResend);
    if (send) {
      // The timer an account's code would have started, and a new Messenger link like an
      // account's resend would give.
      Utils.clearMessengerLink(authSession);
      long lifetime = Long.parseLong(configMap.get(Utils.CODE_TTL)) * 1000L;
      authSession.setAuthNote(Utils.CODE_TTL, Long.toString(System.currentTimeMillis() + lifetime));
    }
    context.getEvent().detail("reason", reason).error(NO_CHANNEL);
    context.challenge(
        codeForm(context, context.form(), configMap, messageCourier, false, send, send));
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
    Utils.CodePageDisclosure disclosure = Utils.CodePageDisclosure.of(configMap, deferredUser);
    form.setAttribute("realm", context.getRealm())
        .setAttribute("courier", messageCourier)
        .setAttribute("isOtl", "true".equals(configMap.get(Utils.ONE_TIME_LINK)))
        .setAttribute("ttl", configMap.get(Utils.CODE_TTL))
        .setAttribute("codeDisclosure", disclosure.name());
    if (disclosure == Utils.CodePageDisclosure.NEUTRAL) {
      // Nothing about the account: no channel, destination, sender or Messenger link.
      setNeutralMessengerAttributes(form, authSession, context.getRealm());
      form.setAttribute("resendTimer", configMap.get(Utils.RESEND_ACTIVATION_TIMER))
          .setAttribute("codeJustSent", codeJustSent)
          .setAttribute("resendTimerStarted", resendTimerStarted)
          .setAttribute("codeLength", configMap.get(Utils.CODE_LENGTH))
          .setAttribute("codeRequest", codeProgress.map(CodeProgress::request).orElse(null))
          .setAttribute("codeRequests", codeProgress.map(CodeProgress::requests).orElse(null));
      return form.createForm(TPL_CODE);
    }
    form.setAttribute(
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
   * The Messenger box of a neutral page, shown to everyone when the event offers Messenger for
   * codes: {@code messengerConnect}, {@code messengerPage}, {@code messagingOrganization}, {@code
   * messengerLink} and {@code messengerQr}. A Messenger voter's live link, or a link whose
   * reference binds to nothing (kept until the next send, as a live one is). No link state: it
   * would tell a live reference from a decoy.
   */
  static void setNeutralMessengerAttributes(
      LoginFormsProvider form, AuthenticationSessionModel authSession, RealmModel realm) {
    Optional<PublicMessagingChannels> projection = PublicMessagingChannels.fromRealm(realm);
    Optional<PublicMessagingChannels.MessengerPage> page =
        projection
            .filter(
                channels ->
                    channels
                        .channelsWithPurpose(
                            sequent.keycloak.authenticator.messaging.MessagePurpose.OTP)
                        .contains(MessageChannel.MESSENGER))
            .flatMap(PublicMessagingChannels::messengerPage);
    if (page.isEmpty()) {
      return;
    }
    String link = authSession.getAuthNote(MessagingAttributes.NOTE_MESSENGER_LINK);
    if (!Utils.isSafeLink(link)) {
      link = Utils.decoyMessengerLink(page.get());
      authSession.setAuthNote(MessagingAttributes.NOTE_MESSENGER_LINK, link);
    }
    form.setAttribute("messengerConnect", true)
        .setAttribute("messengerPage", page.get().displayName())
        .setAttribute("messagingOrganization", Utils.getRealmName(realm))
        .setAttribute("messengerLink", link);
    MessengerQr.dataUri(link).ifPresent(qr -> form.setAttribute("messengerQr", qr));
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
    if (Utils.isSafeLink(link)) {
      form.setAttribute("messengerLink", link);
      MessengerQr.dataUri(link).ifPresent(qr -> form.setAttribute("messengerQr", qr));
    }
    String state = authSession.getAuthNote(MessagingAttributes.NOTE_MESSENGER_STATE);
    if (state != null) {
      form.setAttribute("messengerState", state);
    }
    if (authSession.getAuthNote(MessagingAttributes.NOTE_MESSENGER_RECONNECT) != null) {
      // The page tells why the voter connects Messenger again (R7-10).
      form.setAttribute("messengerReconnect", true);
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
            == Utils.codeCourier(
                Utils.MessageCourier.fromString(configMap.get(Utils.MESSAGE_COURIER_ATTRIBUTE)),
                user,
                deferredUser);

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
