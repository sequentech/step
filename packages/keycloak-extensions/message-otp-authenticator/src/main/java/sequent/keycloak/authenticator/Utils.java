// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.google.common.base.Strings;
import com.google.common.collect.ImmutableList;
import com.google.common.collect.Maps;
import jakarta.ws.rs.core.UriBuilder;
import jakarta.ws.rs.core.UriInfo;
import java.io.IOException;
import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.text.Bidi;
import java.text.MessageFormat;
import java.time.Instant;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashMap;
import java.util.HexFormat;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Optional;
import java.util.Properties;
import java.util.UUID;
import java.util.function.Supplier;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import java.util.stream.Collectors;
import lombok.experimental.UtilityClass;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.RequiredActionContext;
import org.keycloak.authentication.actiontoken.ActionTokenContext;
import org.keycloak.authentication.actiontoken.DefaultActionToken;
import org.keycloak.common.util.SecretGenerator;
import org.keycloak.common.util.Time;
import org.keycloak.email.EmailException;
import org.keycloak.email.EmailSenderProvider;
import org.keycloak.email.EmailTemplateProvider;
import org.keycloak.email.freemarker.beans.ProfileBean;
import org.keycloak.events.Event;
import org.keycloak.events.EventBuilder;
import org.keycloak.forms.login.freemarker.model.UrlBean;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.Constants;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakUriInfo;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.representations.userprofile.config.UPAttribute;
import org.keycloak.representations.userprofile.config.UPConfig;
import org.keycloak.services.Urls;
import org.keycloak.services.resources.LoginActionsService;
import org.keycloak.services.resources.RealmsResource;
import org.keycloak.sessions.AuthenticationSessionCompoundId;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.theme.FreeMarkerException;
import org.keycloak.theme.Theme;
import org.keycloak.theme.beans.MessageFormatterMethod;
import org.keycloak.theme.freemarker.FreeMarkerProvider;
import org.keycloak.userprofile.UserProfileProvider;
import sequent.keycloak.authenticator.gateway.SmsSenderProvider;
import sequent.keycloak.authenticator.messaging.CodeChannels;
import sequent.keycloak.authenticator.messaging.ContactDetails;
import sequent.keycloak.authenticator.messaging.CreateMessengerLinkRequest;
import sequent.keycloak.authenticator.messaging.CreateMessengerLinkResponse;
import sequent.keycloak.authenticator.messaging.MessageAttemptState;
import sequent.keycloak.authenticator.messaging.MessageChannel;
import sequent.keycloak.authenticator.messaging.MessageContent;
import sequent.keycloak.authenticator.messaging.MessagePurpose;
import sequent.keycloak.authenticator.messaging.MessageReason;
import sequent.keycloak.authenticator.messaging.MessageRecipient;
import sequent.keycloak.authenticator.messaging.MessageSenderProvider;
import sequent.keycloak.authenticator.messaging.MessagingAttributes;
import sequent.keycloak.authenticator.messaging.MessengerLinkRequest;
import sequent.keycloak.authenticator.messaging.MessengerLinkState;
import sequent.keycloak.authenticator.messaging.MessengerLinkStatus;
import sequent.keycloak.authenticator.messaging.NoticeRecipient;
import sequent.keycloak.authenticator.messaging.PublicMessagingChannels;
import sequent.keycloak.authenticator.messaging.SendMessageRequest;
import sequent.keycloak.authenticator.messaging.SendMessageResponse;
import sequent.keycloak.authenticator.messaging.VoterChannel;
import sequent.keycloak.authenticator.messaging.VoterChannels;
import sequent.keycloak.authenticator.otl.OTLActionToken;

@UtilityClass
@JBossLog
public class Utils {
  // Login details:
  public final String VOTER_CERT_SUBJECT_DN = "voter_cert_subject_dn";
  public final String CA_CERT_ISSUER_CN = "ca_cert_issuer_cn";
  // Authentication notes:
  public final String AUTH_NOTE_DENY_TYPE = "deny-type";
  // Deny codes:
  public final String CERT_NOT_PROVIDED = "cert-not-provided";
  public final String USER_NOT_FOUND = "user-not-found";
  public final String ACCESS_DENIED = "access-denied";

  public final String CODE = "code";
  public final String CODE_LENGTH = "length";
  public final String CODE_TTL = "ttl";
  public final String SENDER_ID = "senderId";
  public final String ONE_TIME_LINK = "one-time-link";
  public final String OTL_VISITED = "one-time-link.visited";
  public static final String USER_ID = "userId";
  public final String TEL_USER_ATTRIBUTE = "telUserAttribute";
  public final String MESSAGE_COURIER_ATTRIBUTE = "messageCourierAttribute";
  public final String DEFERRED_USER_ATTRIBUTE = "deferredUserAttribute";
  public final String AUTO_CREATE_CREDENTIAL_ATTRIBUTE = "autoCreateCredentialAttribute";
  public final String OTL_RESTORED_AUTH_NOTES_ATTRIBUTE = "otlRestoredAuthNotesAttribute";

  public final String SEND_CODE_SMS_I18N_KEY = "messageOtp.sendCode.sms.text";
  public final String SEND_CODE_MESSAGE_I18N_KEY = "messageOtp.sendCode.message.text";
  public final String SEND_CODE_EMAIL_SUBJECT = "messageOtp.sendCode.email.subject";
  public final String SEND_CODE_EMAIL_FTL = "send-code-email.ftl";
  public final String RESEND_ACTIVATION_TIMER = "resendCoudActivationTimer";
  public final String MAX_CODE_ATTEMPTS = "max-code-attempts";
  public final String CODE_ATTEMPTS = "code-attempts";

  // Default values for message-otp authenticator configuration. Referenced
  // from both MessageOTPAuthenticatorFactory (admin UI default) and
  // ResetMessageOTPRequiredAction (runtime fallback when the realm's saved
  // config is missing the key).
  public final String CODE_LENGTH_DEFAULT = "6";
  public final String CODE_TTL_DEFAULT = "300";
  public final String RESEND_ACTIVATION_TIMER_DEFAULT = "60";
  public final String MAX_CODE_ATTEMPTS_DEFAULT = "5";

  /**
   * Whether a new code may be sent, given the current code's expiry auth note (epoch millis) and
   * the configured code TTL and resend timer (both in seconds).
   */
  public static boolean isResendAllowed(
      String codeExpiryMillis, String codeTtlSeconds, String resendTimerSeconds, long nowMillis) {
    if (codeExpiryMillis == null || codeTtlSeconds == null || resendTimerSeconds == null) {
      return false;
    }
    long sentAt = Long.parseLong(codeExpiryMillis) - Long.parseLong(codeTtlSeconds) * 1000L;
    return sentAt + Long.parseLong(resendTimerSeconds) * 1000L < nowMillis;
  }

  /** The most wrong codes a challenge takes; resends never reset the count. */
  public int maxCodeAttempts(Map<String, String> configMap) {
    String configured = configMap == null ? null : configMap.get(MAX_CODE_ATTEMPTS);
    try {
      return Integer.parseInt(
          configured == null || configured.isBlank() ? MAX_CODE_ATTEMPTS_DEFAULT : configured);
    } catch (NumberFormatException e) {
      return Integer.parseInt(MAX_CODE_ATTEMPTS_DEFAULT);
    }
  }

  /**
   * The note counting one challenge's wrong codes. A challenge is one code step: a flow execution,
   * or a required action. Each has its own count, so a sign-in step's wrong codes never use up a
   * later step's, nor the contact change's; asking for a new code in the same step keeps it, and
   * only a restart of the login (which clears every note) resets it.
   *
   * @param challenge the execution id or required action id; null for the session-wide note older
   *     releases used
   */
  public String attemptsNote(String challenge) {
    return challenge == null || challenge.isBlank()
        ? CODE_ATTEMPTS
        : CODE_ATTEMPTS + "." + challenge;
  }

  /** Wrong codes entered for the challenge in this authentication session. */
  public int codeAttempts(AuthenticationSessionModel authSession, String challenge) {
    String value = authSession.getAuthNote(attemptsNote(challenge));
    try {
      return value == null ? 0 : Integer.parseInt(value);
    } catch (NumberFormatException e) {
      return Integer.MAX_VALUE;
    }
  }

  /**
   * Whether the challenge used up its attempts: it then takes no more codes, and the voter starts
   * again. Compared with the limit, never inferred from the counter's presence.
   */
  public boolean attemptsExhausted(
      AuthenticationSessionModel authSession, Map<String, String> configMap, String challenge) {
    return codeAttempts(authSession, challenge) >= maxCodeAttempts(configMap);
  }

  /**
   * Counts a wrong code against the challenge. Once the configured maximum is reached the code is
   * removed and the challenge takes no more codes: the voter starts again.
   *
   * @return true when this attempt used up the challenge's attempts
   */
  public boolean registerFailedAttempt(
      AuthenticationSessionModel authSession, Map<String, String> configMap, String challenge) {
    int attempts = codeAttempts(authSession, challenge) + 1;
    authSession.setAuthNote(attemptsNote(challenge), Integer.toString(attempts));
    if (attempts < maxCodeAttempts(configMap)) {
      return false;
    }
    authSession.removeAuthNote(CODE);
    return true;
  }

  /** Config key of {@link CodePageDisclosure}. */
  public final String CODE_PAGE_DISCLOSURE = "code-page-disclosure";

  /**
   * What the code page tells about the account the code went to (policy enum, config key {@link
   * #CODE_PAGE_DISCLOSURE}). The default is {@link #NEUTRAL}: a code step reveals nothing unless
   * the realm says the voter is already known there.
   */
  public enum CodePageDisclosure {
    /**
     * The channel and the masked destination. For steps after the voter proved who they are (the
     * sign-in after the password). Enrollment always names the channel: the voter just typed it.
     */
    NAMED,
    /**
     * Nothing about the account: "If an account exists, we sent a code to its contact." An unknown
     * account gets the same page, timer and attempt limit, and nothing is sent. When the event
     * offers Messenger for codes, every neutral page shows the Messenger box (QR and m.me link):
     * the account's live link for a Messenger voter, a link whose reference binds to nothing
     * otherwise. The default; for flows anyone can start with an identifier (reset).
     */
    NEUTRAL;

    /** The configured policy; {@link #NEUTRAL} when absent or unknown. */
    public static CodePageDisclosure fromConfig(Map<String, String> configMap) {
      String value = configMap == null ? null : configMap.get(CODE_PAGE_DISCLOSURE);
      if (value != null) {
        for (CodePageDisclosure disclosure : values()) {
          if (disclosure.name().equalsIgnoreCase(value.trim())) {
            return disclosure;
          }
        }
      }
      return NEUTRAL;
    }

    /** The policy of a step: enrolling voters (deferred) always see the channel they entered. */
    public static CodePageDisclosure of(Map<String, String> configMap, boolean deferredUser) {
      return deferredUser ? NAMED : fromConfig(configMap);
    }
  }

  /**
   * An {@code m.me} link to the Page with a reference that binds to nothing: what a neutral page
   * shows when the account has no live Messenger link, shaped like harvest's ({@code
   * https://m.me/<username or Page id>?ref=<64 hex>}).
   */
  public String decoyMessengerLink(PublicMessagingChannels.MessengerPage page) {
    String target =
        page.username() != null && !page.username().isBlank() ? page.username() : page.pageId();
    return "https://m.me/"
        + java.net.URLEncoder.encode(target, StandardCharsets.UTF_8).replace("+", "%20")
        + "?ref="
        + SecretGenerator.getInstance().randomString(64, "0123456789abcdef".toCharArray());
  }

  /**
   * Whether a Messenger link from harvest may be shown as a link: an absolute https URL with a
   * host. Harvest is trusted; this is defence in depth against a scheme a browser would run.
   */
  public boolean isSafeLink(String link) {
    if (link == null || link.isBlank()) {
      return false;
    }
    try {
      URI uri = URI.create(link.trim());
      return "https".equalsIgnoreCase(uri.getScheme())
          && uri.getHost() != null
          && !uri.getHost().isBlank();
    } catch (IllegalArgumentException e) {
      return false;
    }
  }

  /** Template keys of codes and one-time links; a notice uses its message key. */
  public final String OTP_TEMPLATE_KEY = "otp";

  public final String OTL_TEMPLATE_KEY = "otl";

  public String codeTemplateKey(boolean isOtl) {
    return isOtl ? OTL_TEMPLATE_KEY : OTP_TEMPLATE_KEY;
  }

  /** The voter's Keycloak locale code, e.g. {@code en} or {@code tl}; never empty. */
  public String languageCode(KeycloakSession session, RealmModel realm, UserModel user) {
    Locale locale = session.getContext().resolveLocale(user);
    if (locale == null) {
      String realmLocale = realm == null ? null : realm.getDefaultLocale();
      locale =
          realmLocale == null || realmLocale.isBlank()
              ? Locale.ENGLISH
              : Locale.forLanguageTag(realmLocale);
    }
    return locale.toLanguageTag();
  }

  /**
   * Whether the last code was not sent for certain. No code is kept then, so none can be guessed.
   * The resend timer still applies, as after any send.
   */
  public boolean sendFailed(AuthenticationSessionModel authSession) {
    return MessageAttemptState.FAILED
        .name()
        .equals(authSession.getAuthNote(MessagingAttributes.NOTE_DELIVERY_STATE));
  }

  private MessageAttemptState recordDelivery(
      AuthenticationSessionModel authSession, MessageAttemptState state) {
    authSession.setAuthNote(MessagingAttributes.NOTE_DELIVERY_STATE, state.name());
    if (state == MessageAttemptState.FAILED) {
      authSession.removeAuthNote(Utils.CODE);
    }
    return state;
  }

  public final String SEND_LINK_SMS_I18N_KEY = "messageOtp.sendLink.sms.text";
  public final String SEND_LINK_EMAIL_SUBJECT = "messageOtp.sendLink.email.subject";
  public final String SEND_LINK_EMAIL_FTL = "send-link-email.ftl";

  public static final String SEND_SUCCESS_SMS_I18N_KEY = "messageSuccessSms";
  public static final String SEND_SUCCESS_SMS_I18N_KEY_KIOSK = "messageSuccessSmsKiosk";
  public static final String SEND_SUCCESS_EMAIL_SUBJECT = "messageSuccessEmailSubject";
  public static final String SEND_SUCCESS_EMAIL_FTL = "success-email.ftl";
  public static final String SEND_PENDING_SMS_I18N_KEY = "messagePendingSms";
  public static final String SEND_PENDING_EMAIL_SUBJECT = "messagePendingEmailSubject";
  public static final String SEND_PENDING_EMAIL_FTL = "pending-email.ftl";
  public static final String SEND_REJECT_SMS_I18N_KEY = "messageRejectedSms";
  public static final String SEND_REJECT_EMAIL_SUBJECT = "messageRejectedEmailSubject";
  public static final String SEND_REJECT_EMAIL_FTL = "reject-email.ftl";
  public static final String SEND_SUCCESS_EMAIL_DIFF_POST_FTL = "success-email-diff-post.ftl";
  public static final String ERROR_MESSAGE_NOT_SENT = "messageNotSent";

  public static final String SEND_SUPPORT_ERROR_EMAIL_SUBJECT =
      "userRegistrationErrorNotificationSubject";
  public static final String SEND_SUPPORT_ERROR_EMAIL_FTL = "support-error-email.ftl";

  public static final String ID_NUMBER_ATTRIBUTE = "sequent.read-only.id-card-number";
  public static final String PHONE_NUMBER_ATTRIBUTE = "sequent.read-only.mobile-number";

  public static final String ID_NUMBER = "ID_number";
  public static final String PHONE_NUMBER = "Phone_number";
  public static final String USER_PROFILE_ATTRIBUTES = "user_profile_attributes";
  public static final String AUTHENTICATOR_CLASS_NAME = "authenticator_class_name";

  public static final String EVENT_TYPE_COMMUNICATIONS = "communications";
  public static final String TEST_MODE_ATTRIBUTE = "test-mode";
  public static final String TEST_MODE_CODE_ATTRIBUTE = "test-mode-code";
  public static final String MAX_RECEIVER_REUSE = "max-receiver-reuse";
  public static final String VALID_COUNTRY_CODES = "valid-country-codes";
  public static final List<String> VALID_COUNTRY_CODES_DEFAULT =
      Collections.unmodifiableList(Arrays.asList());

  public enum MessageCourier {
    SMS,
    EMAIL,
    BOTH,
    NONE,
    /**
     * One channel: the one the voter chose among the event's channels for the purpose (realm
     * attribute {@code sequent.messaging}), or email and SMS when the event publishes none.
     */
    CHOSEN;

    // Method to convert a string value to a NotificationType
    public static MessageCourier fromString(String type) {
      if (type == null || type.isEmpty()) {
        return BOTH;
      }
      for (MessageCourier messageCourier : MessageCourier.values()) {
        if (type.equalsIgnoreCase(messageCourier.name())) {
          return messageCourier;
        }
      }
      throw new IllegalArgumentException("No constant with text " + type + " found");
    }
  }

  String escapeJson(String value) {
    return value != null
        ? value.replace("\"", "\\\"").replace("\n", "\\n").replace("\r", "\\r")
        : null;
  }

  void sendFeedback(
      AuthenticatorConfigModel config,
      KeycloakSession session,
      UserModel user,
      AuthenticationSessionModel authSession,
      MessageCourier messageCourier,
      boolean isSuccess,
      boolean deferredUser,
      boolean isOtl)
      throws IOException {
    log.info("sendFeedback(): start");
    String mobileNumber = null;

    // Handle deferred user
    if (messageCourier == MessageCourier.CHOSEN) {
      mobileNumber =
          MessageChannel.parse(authSession.getAuthNote(MessagingAttributes.NOTE_CODE_CHANNEL))
                  .equals(Optional.of(MessageChannel.SMS))
              ? authSession.getAuthNote(MessagingAttributes.NOTE_CODE_DESTINATION)
              : null;
    } else if (deferredUser) {
      String mobileNumberAttribute = config.getConfig().get(Utils.TEL_USER_ATTRIBUTE);
      mobileNumber = authSession.getAuthNote(mobileNumberAttribute);
    } else {
      mobileNumber = Utils.getMobile(config, user);
    }

    if (isOtl) {
      log.info("sendFeedback(): isOtl so not sending feedback");
      return;
    }
    RealmModel realm = authSession.getRealm();
    MessageCourier keycloakCourier =
        keycloakCourier(
            messageCourier,
            MessageChannel.parse(authSession.getAuthNote(MessagingAttributes.NOTE_CODE_CHANNEL)),
            VoterChannels.sender(session));

    if (mobileNumber != null
        && mobileNumber.trim().length() > 0
        && (keycloakCourier == MessageCourier.SMS || keycloakCourier == MessageCourier.BOTH)) {
      SmsSenderProvider smsSenderProvider = session.getProvider(SmsSenderProvider.class);
      smsSenderProvider.sendFeedback(mobileNumber, isSuccess, realm, user, session);
    }
  }

  public List<String> getMultivalueString(
      AuthenticatorConfigModel config, String configKey, List<String> defaultValue) {
    log.infov("getMultivalueString(configKey={0}, defaultValue={1})", configKey, defaultValue);
    if (config == null) {
      log.infov("getMultivalueString(): NULL config={0}", config);
      return defaultValue;
    }

    Map<String, String> mapConfig = config.getConfig();
    if (mapConfig == null
        || !mapConfig.containsKey(configKey)
        || mapConfig.get(configKey).strip().length() == 0) {
      log.infov("getMultivalueString(): missing configuration for {0}", configKey);
      return defaultValue;
    }

    log.infov("getMultivalueString(): value={0}", mapConfig.get(configKey));

    return Arrays.asList(mapConfig.get(configKey).split(","));
  }

  String getMobileNumber(
      AuthenticatorConfigModel config,
      UserModel user,
      AuthenticationSessionModel authSession,
      boolean deferredUser)
      throws IOException {
    String mobileNumber = null;

    // Handle deferred user
    if (deferredUser) {
      mobileNumber = authSession.getAuthNote(telUserAttribute(config));
    } else {
      mobileNumber = Utils.getMobile(config, user);
    }

    return mobileNumber;
  }

  String getEmailAddress(
      UserModel user, AuthenticationSessionModel authSession, boolean deferredUser)
      throws IOException {
    String emailAddress = null;

    // Handle deferred user
    if (deferredUser) {
      emailAddress = authSession.getAuthNote("email");
    } else {
      emailAddress = user.getEmail();
    }

    return emailAddress;
  }

  /**
   * The courier Keycloak's own email and SMS providers serve. For {@link MessageCourier#CHOSEN}
   * that is the code's channel when it is email or SMS and the message sender does not deliver it,
   * and NONE otherwise.
   */
  MessageCourier keycloakCourier(
      MessageCourier messageCourier,
      Optional<MessageChannel> channel,
      MessageSenderProvider sender) {
    if (messageCourier != MessageCourier.CHOSEN) {
      return messageCourier;
    }
    if (channel.isEmpty() || sender.delivers(channel.get())) {
      return MessageCourier.NONE;
    }
    return switch (channel.get()) {
      case EMAIL -> MessageCourier.EMAIL;
      case SMS -> MessageCourier.SMS;
      default -> MessageCourier.NONE;
    };
  }

  /**
   * The courier a code step uses for this voter (R7-01). A saved voter whose record holds a
   * messaging channel (RECORDED, or a first-release VERIFIED_CHANNELS record) always gets the code
   * on that channel, whatever the step's configured courier: {@link MessageCourier#CHOSEN}; when
   * that channel can't be used, no code is sent and the refusal says why. The configured courier
   * (EMAIL, SMS, BOTH) stays the realm's policy for LEGACY voters only, and while enrolling. NONE
   * sends nothing, so it never reaches another channel either.
   */
  public MessageCourier codeCourier(
      MessageCourier configured, UserModel user, boolean deferredUser) {
    if (configured == MessageCourier.CHOSEN
        || configured == MessageCourier.NONE
        || deferredUser
        || user == null) {
      return configured;
    }
    VoterChannel.Source shape = VoterChannel.shape(user);
    if (shape == VoterChannel.Source.LEGACY) {
      return configured;
    }
    log.infov(
        "codeCourier(): the voter's {0} record overrides the step's courier {1}: the code goes only"
            + " to the voter's channel",
        shape, configured);
    return MessageCourier.CHOSEN;
  }

  /**
   * The realm's message OTP steps that could send a code elsewhere than the voter's channel: steps
   * that can run (reached from a top-level flow through sub-flows, neither the step nor a sub-flow
   * on the way DISABLED) whose courier is neither {@link MessageCourier#CHOSEN} nor {@link
   * MessageCourier#NONE} (which sends nothing). A step without its own configuration uses the
   * default, BOTH. An orphan sub-flow never runs, so it doesn't count. Their execution ids, for the
   * log.
   */
  public List<String> codeStepsNotOnVoterChannel(RealmModel realm) {
    if (realm == null) {
      return List.of();
    }
    List<String> steps = new ArrayList<>();
    java.util.Set<String> visited = new java.util.HashSet<>();
    java.util.Deque<String> pending = new java.util.ArrayDeque<>();
    realm
        .getAuthenticationFlowsStream()
        .filter(org.keycloak.models.AuthenticationFlowModel::isTopLevel)
        .map(org.keycloak.models.AuthenticationFlowModel::getId)
        .forEach(pending::add);
    while (!pending.isEmpty()) {
      String flowId = pending.poll();
      if (flowId == null || !visited.add(flowId)) {
        continue;
      }
      realm
          .getAuthenticationExecutionsStream(flowId)
          .filter(
              execution ->
                  execution.getRequirement()
                      != org.keycloak.models.AuthenticationExecutionModel.Requirement.DISABLED)
          .forEach(
              execution -> {
                if (execution.isAuthenticatorFlow()) {
                  pending.add(execution.getFlowId());
                } else if (MessageOTPAuthenticatorFactory.PROVIDER_ID.equals(
                        execution.getAuthenticator())
                    && !sendsOnlyToTheVotersChannel(realm, execution)) {
                  steps.add(String.valueOf(execution.getId()));
                }
              });
    }
    return steps;
  }

  /** Whether a code step's courier is CHOSEN, or NONE (it sends nothing). */
  private boolean sendsOnlyToTheVotersChannel(
      RealmModel realm, org.keycloak.models.AuthenticationExecutionModel execution) {
    AuthenticatorConfigModel config =
        execution.getAuthenticatorConfig() == null
            ? null
            : realm.getAuthenticatorConfigById(execution.getAuthenticatorConfig());
    String courier =
        MessageOTPAuthenticatorFactory.getConfigMap(config).get(MESSAGE_COURIER_ATTRIBUTE);
    try {
      MessageCourier parsed = MessageCourier.fromString(courier);
      return parsed == MessageCourier.CHOSEN || parsed == MessageCourier.NONE;
    } catch (IllegalArgumentException e) {
      return false;
    }
  }

  /** Forgets the Messenger link of the previous code, so it cannot confirm a contact. */
  void clearMessengerLink(AuthenticationSessionModel authSession) {
    authSession.removeAuthNote(MessagingAttributes.NOTE_MESSENGER_REFERENCE);
    authSession.removeAuthNote(MessagingAttributes.NOTE_MESSENGER_LINK);
    authSession.removeAuthNote(MessagingAttributes.NOTE_MESSENGER_WORD);
    authSession.removeAuthNote(MessagingAttributes.NOTE_MESSENGER_STATE);
    authSession.removeAuthNote(MessagingAttributes.NOTE_MESSENGER_RECONNECT);
  }

  /**
   * The event's Messenger Page, when a saved Messenger voter's Page is another one: the voter's
   * account on the old Page can't be reached any more (R7-10). Empty for any other voter, and when
   * either Page is unknown.
   */
  public Optional<String> messengerPageChange(RealmModel realm, VoterChannel channel) {
    if (channel.channel() != MessageChannel.MESSENGER
        || channel.destination().isBlank()
        || channel.messengerPage().isEmpty()) {
      return Optional.empty();
    }
    return PublicMessagingChannels.fromRealm(realm)
        .flatMap(PublicMessagingChannels::messengerPage)
        .map(PublicMessagingChannels.MessengerPage::pageId)
        .filter(page -> !page.equals(channel.messengerPage().get()));
  }

  /**
   * Where a {@link MessageCourier#CHOSEN} code goes: the voter's one channel at sign-in, the
   * channel and contact just entered otherwise.
   */
  public CodeChannels.Resolution codeChannel(
      AuthenticatorConfigModel config,
      KeycloakSession session,
      UserModel user,
      AuthenticationSessionModel authSession,
      boolean deferredUser) {
    return deferredUser
        ? CodeChannels.forEnrollment(
            session, authSession.getRealm(), authSession, telUserAttribute(config))
        : CodeChannels.forSignIn(session, authSession.getRealm(), user);
  }

  /**
   * Sends code and also sets the auth notes related to the code. Issuing a code replaces the
   * previous one and its Messenger link in the same authentication session; the session's attempt
   * count is kept, so asking for codes never yields more attempts.
   *
   * @return the delivery outcome; email and SMS sent by Keycloak's providers are ACCEPTED
   */
  MessageAttemptState sendCode(
      AuthenticatorConfigModel config,
      KeycloakSession session,
      UserModel user,
      AuthenticationSessionModel authSession,
      MessageCourier messageCourier,
      boolean deferredUser,
      boolean isOtl,
      String[] otlAuthNotesNames,
      Object context)
      throws IOException, EmailException {
    return sendCode(
        config,
        session,
        user,
        authSession,
        messageCourier,
        deferredUser,
        isOtl,
        otlAuthNotesNames,
        context,
        MessengerLinkPolicy.VOTERS_ACCOUNT);
  }

  /**
   * Whose Messenger account a saved voter's code link may reach: {@code VOTERS_ACCOUNT} (their
   * saved account on their saved Page; harvest refuses the link when the Page changed), or {@code
   * RECONNECT} (any account on the event's Page, after a Page change; only a sign-in step after a
   * validated password asks for it, R7-10).
   */
  public enum MessengerLinkPolicy {
    VOTERS_ACCOUNT,
    RECONNECT
  }

  /** {@link #sendCode}, with the Messenger link's policy. */
  MessageAttemptState sendCode(
      AuthenticatorConfigModel config,
      KeycloakSession session,
      UserModel user,
      AuthenticationSessionModel authSession,
      MessageCourier messageCourier,
      boolean deferredUser,
      boolean isOtl,
      String[] otlAuthNotesNames,
      Object context,
      MessengerLinkPolicy messengerLink)
      throws IOException, EmailException {
    log.info("sendCode(): start");
    Map<String, String> configMap = MessageOTPAuthenticatorFactory.getConfigMap(config);
    String mobileNumber;
    String emailAddress;
    String code = null;
    MessageSenderProvider sender = VoterChannels.sender(session);
    Optional<VoterChannel> target = Optional.empty();
    if (messageCourier == MessageCourier.CHOSEN) {
      CodeChannels.Resolution resolution =
          codeChannel(config, session, user, authSession, deferredUser);
      if (resolution.refusal().isPresent()) {
        throw new IOException("No code sent: " + resolution.refusal().get().reason());
      }
      target = resolution.channel();
      VoterChannel channel = target.get();
      mobileNumber = channel.channel() == MessageChannel.SMS ? channel.destination() : null;
      emailAddress = channel.channel() == MessageChannel.EMAIL ? channel.destination() : null;
    } else {
      mobileNumber = Utils.getMobileNumber(config, user, authSession, deferredUser);
      emailAddress = Utils.getEmailAddress(user, authSession, deferredUser);
    }
    Optional<MessageChannel> chosenChannel = target.map(VoterChannel::channel);
    MessageCourier keycloakCourier = keycloakCourier(messageCourier, chosenChannel, sender);

    if (isOtl
        && (keycloakCourier == MessageCourier.SMS || keycloakCourier == MessageCourier.BOTH)
        && session.getProvider(SmsSenderProvider.class).sendsOnlyCodes()) {
      log.warn(MessageReason.SMS_PROVIDER_SENDS_ONLY_CODES.reason());
      throw new IOException(MessageReason.SMS_PROVIDER_SENDS_ONLY_CODES.reason());
    }

    int length = Integer.parseInt(configMap.get(Utils.CODE_LENGTH));
    int ttl = Integer.parseInt(configMap.get(Utils.CODE_TTL));
    authSession.setAuthNote(
        Utils.CODE_TTL, Long.toString(System.currentTimeMillis() + (ttl * 1000L)));
    authSession.setAuthNote(MessagingAttributes.NOTE_CODE_ID, UUID.randomUUID().toString());
    clearMessengerLink(authSession);
    if (target.isPresent()) {
      authSession.setAuthNote(MessagingAttributes.NOTE_CODE_CHANNEL, target.get().channel().name());
      if (target.get().destination().isBlank()) {
        authSession.removeAuthNote(MessagingAttributes.NOTE_CODE_DESTINATION);
      } else {
        authSession.setAuthNote(
            MessagingAttributes.NOTE_CODE_DESTINATION, target.get().destination());
      }
    }

    // Handle OTL/OTP
    if (isOtl) {
      code =
          generateOTL(
              authSession,
              session,
              ttl,
              otlAuthNotesNames,
              authSession.getRedirectUri(),
              deferredUser);
      authSession.setAuthNote(Utils.OTL_VISITED, "false");
    } else {
      code = SecretGenerator.getInstance().randomString(length, SecretGenerator.DIGITS);
      authSession.setAuthNote(Utils.CODE, code);
    }

    RealmModel realm = authSession.getRealm();
    String realmName = getRealmName(realm);

    log.infov("sendCode(): messageCourier=`{0}`, channel=`{1}`", messageCourier, chosenChannel);

    if (target.isPresent() && keycloakCourier == MessageCourier.NONE) {
      return recordDelivery(
          authSession,
          sendCodeThroughSender(
              sender,
              target.get(),
              session,
              user,
              authSession,
              isOtl,
              code,
              ttl,
              context,
              messengerLink));
    }

    // Sending via SMS
    if (mobileNumber != null
        && mobileNumber.trim().length() > 0
        && (keycloakCourier == MessageCourier.SMS || keycloakCourier == MessageCourier.BOTH)) {
      SmsSenderProvider smsSenderProvider = session.getProvider(SmsSenderProvider.class);
      log.info("sendCode(): sending SMS");
      List<String> smsAttributes =
          ImmutableList.of(realmName, code, String.valueOf(Math.floorDiv(ttl, 60)));

      String smsTemplateKey = (isOtl) ? Utils.SEND_LINK_SMS_I18N_KEY : Utils.SEND_CODE_SMS_I18N_KEY;
      String formattedMessage =
          smsSenderProvider.send(
              mobileNumber.trim(), smsTemplateKey, smsAttributes, realm, user, session);
      keycloakSent(context, MessageChannel.SMS, mobileNumber.trim());
    } else {
      log.info("sendCode(): no SMS for this code");
    }

    // Sending via Email
    if (emailAddress != null
        && emailAddress.trim().length() > 0
        && (keycloakCourier == MessageCourier.EMAIL || keycloakCourier == MessageCourier.BOTH)) {
      log.info("sendCode(): sending email");
      EmailTemplateProvider emailTemplateProvider =
          session.getProvider(EmailTemplateProvider.class);

      Map<String, Object> messageAttributes = Maps.newHashMap();
      messageAttributes.put("realmName", realmName);
      messageAttributes.put("code", code);
      messageAttributes.put("ttl", Math.floorDiv(ttl, 60));

      List<Object> subjAttr = ImmutableList.of(realmName);
      log.infov("sendCode(): Sending email: prepared messageAttributes");

      try {
        String subjectKey = (isOtl) ? Utils.SEND_LINK_EMAIL_SUBJECT : Utils.SEND_CODE_EMAIL_SUBJECT;
        String ftlKey = (isOtl) ? Utils.SEND_LINK_EMAIL_FTL : Utils.SEND_CODE_EMAIL_FTL;
        String textBody =
            sendEmail(
                session,
                realm,
                user,
                subjectKey,
                subjAttr,
                ftlKey,
                messageAttributes,
                emailAddress.trim(),
                deferredUser,
                null);
        keycloakSent(context, MessageChannel.EMAIL, emailAddress.trim());
      } catch (EmailException error) {
        log.debug("sendCode(): Exception sending email");
        throw error;
      }
    } else {
      log.info("sendCode(): no email for this code");
    }
    return recordDelivery(authSession, MessageAttemptState.ACCEPTED);
  }

  private MessageAttemptState sendCodeThroughSender(
      MessageSenderProvider sender,
      VoterChannel target,
      KeycloakSession session,
      UserModel user,
      AuthenticationSessionModel authSession,
      boolean isOtl,
      String code,
      int ttl,
      Object context,
      MessengerLinkPolicy messengerLink)
      throws IOException {
    RealmModel realm = authSession.getRealm();
    MessageChannel channel = target.channel();
    String minutes = String.valueOf(Math.floorDiv(ttl, 60));
    String textKey = isOtl ? Utils.SEND_LINK_SMS_I18N_KEY : Utils.SEND_CODE_MESSAGE_I18N_KEY;
    String text =
        formatLoginMessage(
            session, realm, user, textKey, List.of(getRealmName(realm), code, minutes));
    MessageContent content =
        new MessageContent(null, text, null, List.of(code, minutes), isOtl ? null : code);
    String expiresAt =
        Instant.ofEpochMilli(Long.parseLong(authSession.getAuthNote(Utils.CODE_TTL))).toString();
    String language = languageCode(session, realm, user);
    String codeId = authSession.getAuthNote(MessagingAttributes.NOTE_CODE_ID);

    if (channel == MessageChannel.MESSENGER) {
      // A saved voter: only their own Messenger account, on their Page, may get the code. A
      // reconnect after a Page change binds to the account that taps Get Started on the event's
      // Page; its confirmation then replaces the saved one (R7-10).
      boolean saved = user != null && !target.destination().isBlank();
      Optional<String> reconnectPage =
          saved && messengerLink == MessengerLinkPolicy.RECONNECT
              ? messengerPageChange(realm, target)
              : Optional.empty();
      String expectedRecipient = saved && reconnectPage.isEmpty() ? target.destination() : null;
      String expectedPage =
          saved && reconnectPage.isEmpty() ? target.messengerPage().orElse(null) : null;
      try {
        CreateMessengerLinkResponse link =
            sender.createMessengerLink(
                new CreateMessengerLinkRequest(
                    tenantId(realm),
                    electionEventId(realm),
                    sessionDigest(authSession),
                    challengeDigest(authSession, codeId),
                    code,
                    language,
                    content,
                    expiresAt,
                    expectedRecipient,
                    expectedPage));
        if (!isSafeLink(link.link())) {
          log.warn("sendCode(): harvest's Messenger link is not an https link; not shown");
          return MessageAttemptState.FAILED;
        }
        authSession.setAuthNote(MessagingAttributes.NOTE_MESSENGER_REFERENCE, link.reference());
        authSession.setAuthNote(MessagingAttributes.NOTE_MESSENGER_LINK, link.link());
        authSession.setAuthNote(
            MessagingAttributes.NOTE_MESSENGER_STATE, MessengerLinkState.PENDING.name());
        reconnectPage.ifPresent(
            page -> authSession.setAuthNote(MessagingAttributes.NOTE_MESSENGER_RECONNECT, page));
        // Harvest logs the code's send once the voter taps Get Started (R2-07).
        return MessageAttemptState.QUEUED;
      } catch (IOException e) {
        log.warn("sendCode(): the Messenger link could not be created");
        return MessageAttemptState.FAILED;
      }
    }

    String destination = target.destination();
    if (destination == null || destination.isBlank()) {
      throw new IOException("The voter has no contact for the chosen channel");
    }
    String voterId = user != null ? user.getId() : authSession.getAuthNote(USER_ID);
    SendMessageResponse response =
        sender.send(
            new SendMessageRequest(
                tenantId(realm),
                electionEventId(realm),
                voterId,
                channel,
                MessagePurpose.OTP,
                destination,
                language,
                content,
                "otp:" + codeId,
                expiresAt,
                codeTemplateKey(isOtl)));
    // Harvest writes the attempt's one electoral-log entry (R2-07): none here.
    return recordSendOutcome(context, channel, response);
  }

  /**
   * A send harvest didn't accept: its state and reason go into the event's details ({@code
   * send_state}, {@code send_reason}) and the log, so administrators see why. The page stays
   * generic. Harvest's reason is short and never holds content or the destination.
   *
   * @return the send's state
   */
  MessageAttemptState recordSendOutcome(
      Object context, MessageChannel channel, SendMessageResponse response) {
    MessageAttemptState state = response.attemptState();
    String reason = response.reason();
    if (state == MessageAttemptState.ACCEPTED || reason == null || reason.isBlank()) {
      return state;
    }
    log.warnv(
        "sendCode(): not accepted: channel={0}, state={1}, reason={2}", channel, state, reason);
    EventBuilder event = getEvent(context);
    if (event != null) {
      event.detail("send_state", state.name()).detail("send_reason", reason);
    }
    return state;
  }

  /** What the communications log keeps of a message sent by the sender: never its content. */
  String sanitizedSend(
      MessageChannel channel, String maskedTo, String messageId, MessageAttemptState state) {
    return sanitizedSend(channel, maskedTo, messageId, state, null);
  }

  /** Same, with the reason a message was refused or failed. */
  String sanitizedSend(
      MessageChannel channel,
      String maskedTo,
      String messageId,
      MessageAttemptState state,
      String reason) {
    ObjectNode entry = new ObjectMapper().createObjectNode();
    entry.put("channel", channel == null ? null : channel.name());
    entry.put("to", maskedTo);
    if (messageId != null) {
      entry.put("message_id", messageId);
    }
    entry.put("state", state.name());
    if (reason != null) {
      entry.put("reason", reason);
    }
    return entry.toString();
  }

  String telUserAttribute(AuthenticatorConfigModel config) {
    String attribute =
        MessageOTPAuthenticatorFactory.getConfigMap(config).get(Utils.TEL_USER_ATTRIBUTE);
    return attribute == null || attribute.isBlank()
        ? MessageOTPAuthenticator.MOBILE_NUMBER_FIELD
        : attribute;
  }

  /** Opaque digest of the authentication session that harvest binds a Messenger link to. */
  String sessionDigest(AuthenticationSessionModel authSession) {
    return sha256Hex(
        "auth-session:" + authSession.getParentSession().getId() + ":" + authSession.getTabId());
  }

  /** Opaque digest of the live code; it changes whenever a code is issued. */
  String challengeDigest(AuthenticationSessionModel authSession, String codeId) {
    return sha256Hex("challenge:" + sessionDigest(authSession) + ":" + codeId);
  }

  private String sha256Hex(String value) {
    try {
      MessageDigest digest = MessageDigest.getInstance("SHA-256");
      return HexFormat.of().formatHex(digest.digest(value.getBytes(StandardCharsets.UTF_8)));
    } catch (NoSuchAlgorithmException e) {
      throw new IllegalStateException(e);
    }
  }

  MessengerLinkRequest messengerLinkRequest(AuthenticationSessionModel authSession) {
    return new MessengerLinkRequest(
        tenantId(authSession.getRealm()),
        authSession.getAuthNote(MessagingAttributes.NOTE_MESSENGER_REFERENCE),
        sessionDigest(authSession),
        challengeDigest(authSession, authSession.getAuthNote(MessagingAttributes.NOTE_CODE_ID)));
  }

  /**
   * Confirms the Messenger link once the code was verified in this session. Harvest only confirms
   * the live, unreplaced reference of this session and code.
   *
   * @return the confirmed link with the voter's Page-scoped ID, or empty when it is not confirmed
   */
  Optional<MessengerLinkStatus> confirmedMessengerLink(
      KeycloakSession session, AuthenticationSessionModel authSession) {
    if (authSession.getAuthNote(MessagingAttributes.NOTE_MESSENGER_REFERENCE) == null) {
      return Optional.empty();
    }
    MessengerLinkStatus status;
    try {
      status =
          VoterChannels.sender(session).confirmMessengerLink(messengerLinkRequest(authSession));
    } catch (IOException e) {
      log.warn("confirmedMessengerLink(): harvest did not confirm the Messenger link");
      return Optional.empty();
    }
    String pageScopedId = status.pageScopedId();
    if (status.linkState() != MessengerLinkState.CONFIRMED
        || pageScopedId == null
        || pageScopedId.isBlank()) {
      return Optional.empty();
    }
    return Optional.of(status);
  }

  /** Asks harvest whether the voter interacted with the Messenger Page yet. */
  void refreshMessengerState(KeycloakSession session, AuthenticationSessionModel authSession) {
    if (authSession.getAuthNote(MessagingAttributes.NOTE_MESSENGER_REFERENCE) == null) {
      return;
    }
    try {
      MessengerLinkStatus status =
          VoterChannels.sender(session).messengerLinkStatus(messengerLinkRequest(authSession));
      authSession.setAuthNote(MessagingAttributes.NOTE_MESSENGER_STATE, status.linkState().name());
    } catch (IOException e) {
      log.warn("refreshMessengerState(): harvest did not report the Messenger link");
    }
  }

  /** Formats a message of the login theme, the way SMS texts are formatted. */
  public String formatLoginMessage(
      KeycloakSession session, RealmModel realm, UserModel user, String key, List<?> attributes)
      throws IOException {
    Locale locale = session.getContext().resolveLocale(user);
    Theme theme = session.theme().getTheme(Theme.Type.LOGIN);
    Properties messages = theme.getEnhancedMessages(realm, locale);
    return new MessageFormat(messages.getProperty(key, key), locale).format(attributes.toArray());
  }

  /** The tenant of an election event or tenant realm, from its name. */
  public String tenantId(RealmModel realm) {
    Matcher matcher =
        Pattern.compile(
                "\\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\\b")
            .matcher(realm.getName());
    return matcher.find() ? matcher.group() : null;
  }

  public String electionEventId(RealmModel realm) {
    String[] parts = realm.getName().split("event-");
    return parts.length > 1 ? parts[1] : null;
  }

  /* Masks the auth code from the content body with stars */
  protected String maskCode(String content, String code) {
    String masked = "*".repeat(code.length());
    String result = content.replace(code, masked).replace(code.replace("&", "&amp;"), masked);
    String query = URI.create(code).getRawQuery();
    if (query != null) {
      // Template sanitizers can normalize the surrounding URL without changing its token.
      for (String parameter : query.split("&")) {
        String prefix = Constants.KEY + "=";
        if (parameter.startsWith(prefix)) {
          String token = parameter.substring(prefix.length());
          if (!token.isEmpty()) {
            result = result.replace(token, "*".repeat(token.length()));
          }
        }
      }
    }
    return result;
  }

  void communicationsLog(Object context, String body) {
    if (context instanceof AuthenticationFlowContext) {
      logCommunications((AuthenticationFlowContext) context, body);
    } else if (context instanceof RequiredActionContext) {
      logCommunications((RequiredActionContext) context, body);
    } else if (context != null) {
      log.warn(
          "Unsupported context type for communications logging: " + context.getClass().getName());
    }
  }

  private <T> void logCommunications(T context, String body) {
    EventBuilder event = getEvent(context);
    if (event != null) {
      event.detail("type", EVENT_TYPE_COMMUNICATIONS).detail("msgBody", body).success();
    }
  }

  private EventBuilder getEvent(Object context) {
    if (context instanceof AuthenticationFlowContext) {
      return ((AuthenticationFlowContext) context).getEvent();
    } else if (context instanceof RequiredActionContext) {
      return ((RequiredActionContext) context).getEvent();
    }
    return null;
  }

  String getMobile(AuthenticatorConfigModel config, UserModel user) {
    log.infov("getMobile()");
    if (config == null) {
      log.infov("getMobile(): NULL config={0}", config);
      return user.getFirstAttribute(MessageOTPAuthenticator.MOBILE_NUMBER_FIELD);
    }

    Map<String, String> mapConfig = config.getConfig();
    if (mapConfig == null || !mapConfig.containsKey(Utils.TEL_USER_ATTRIBUTE)) {
      log.info("getMobile(): missing telephone attribute configuration");
      return user.getFirstAttribute(MessageOTPAuthenticator.MOBILE_NUMBER_FIELD);
    }
    String telUserAttribute = mapConfig.get(Utils.TEL_USER_ATTRIBUTE);

    String mobile = user.getFirstAttribute(telUserAttribute);
    log.infov("getMobile(): telUserAttribute={0}", telUserAttribute);
    return mobile;
  }

  public static String linkFromActionToken(
      KeycloakSession session, RealmModel realm, DefaultActionToken token) {
    UriInfo uriInfo = session.getContext().getUri();
    UriBuilder builder =
        actionTokenBuilder(
            uriInfo.getBaseUri(), token.serialize(session, realm, uriInfo), token.getIssuedFor());
    return builder.build(realm.getName()).toString();
  }

  UriBuilder actionTokenBuilder(URI baseUri, String tokenString, String clientId) {
    log.infof("actionTokenBuilder(): baseUri: %s, clientId: %s", baseUri, clientId);
    return Urls.realmBase(baseUri)
        .path(RealmsResource.class, "getLoginActionsService")
        .path(LoginActionsService.class, "executeActionToken")
        .queryParam(Constants.KEY, tokenString)
        .queryParam(Constants.CLIENT_ID, clientId);
  }

  String generateOTL(
      AuthenticationSessionModel authSession,
      KeycloakSession session,
      int ttl,
      String[] otlAuthNotesNames,
      String redirectUri,
      boolean isDeferredUser) {
    // Get necessary components from the context
    AuthenticationSessionCompoundId compoundId =
        AuthenticationSessionCompoundId.fromAuthSession(authSession);
    String sessionId = compoundId.getEncodedId();
    String userId =
        authSession.getAuthenticatedUser() == null
            ? authSession.getAuthNote(USER_ID)
            : authSession.getAuthenticatedUser().getId();
    RealmModel realm = authSession.getRealm();

    // Create the OTLActionToken with the necessary information
    OTLActionToken token =
        new OTLActionToken(
            userId,
            Time.currentTime() + ttl,
            sessionId, // Original compound session ID
            otlAuthNotesNames,
            isDeferredUser,
            redirectUri,
            authSession.getClient().getClientId());

    // Generate the OTL link
    return linkFromActionToken(session, realm, token);
  }
  ;

  Optional<AuthenticatorConfigModel> getConfig(RealmModel realm) {
    // Using streams to find the first matching configuration
    // TODO: We're assuming there's only one instance in this realm of this
    // authenticator
    Optional<AuthenticatorConfigModel> configOptional =
        realm
            .getAuthenticationFlowsStream()
            .flatMap(flow -> realm.getAuthenticationExecutionsStream(flow.getId()))
            .filter(
                model -> {
                  boolean ret =
                      (model.getAuthenticator() != null
                          && model
                              .getAuthenticator()
                              .equals(MessageOTPAuthenticatorFactory.PROVIDER_ID));
                  return ret;
                })
            .map(model -> realm.getAuthenticatorConfigById(model.getAuthenticatorConfig()))
            .findFirst();
    return configOptional;
  }

  /** We use constant time comparison for security reasons, to avoid timing attacks */
  boolean constantTimeIsEqual(byte[] digesta, byte[] digestb) {
    if (digesta.length != digestb.length) {
      return false;
    }

    int result = 0;
    // time-constant comparison
    for (int i = 0; i < digesta.length; i++) {
      result |= digesta[i] ^ digestb[i];
    }
    return result == 0;
  }

  public static String getRealmName(RealmModel realm) {
    return Strings.isNullOrEmpty(realm.getDisplayName()) ? realm.getName() : realm.getDisplayName();
  }

  protected EmailTemplate processEmailTemplate(
      KeycloakSession session,
      RealmModel realm,
      UserModel user,
      String subjectKey,
      List<Object> subjectAttributes,
      String template,
      Map<String, Object> attributes)
      throws EmailException {
    try {
      Theme theme = session.theme().getTheme(Theme.Type.EMAIL);

      Locale locale;
      if (user != null) {
        locale = session.getContext().resolveLocale(user);
      } else {
        locale = session.getContext().resolveLocale(null);
        if (locale == null) {
          String defaultLocale = realm.getDefaultLocale();
          if (defaultLocale != null) {
            locale = Locale.forLanguageTag(defaultLocale);
          } else {
            locale = Locale.getDefault();
          }
        }
      }
      attributes.put("locale", locale);

      // Determine text direction based on locale using java.text.Bidi
      // This follows the same approach as Keycloak's LocaleBean.isLeftToRight()
      String localizedName = locale.getDisplayName(locale);
      Bidi bidi = new Bidi(localizedName, Bidi.DIRECTION_DEFAULT_LEFT_TO_RIGHT);
      boolean isLtr = bidi.isLeftToRight();
      attributes.put("ltr", isLtr);

      Properties messages = theme.getEnhancedMessages(realm, locale);
      attributes.put("msg", new MessageFormatterMethod(locale, messages));

      attributes.put("properties", theme.getProperties());
      attributes.put("realmName", realm.getName());
      if (user != null) {
        attributes.put("user", new ProfileBean(user, session));
      }
      KeycloakUriInfo uriInfo = session.getContext().getUri();
      attributes.put("url", new UrlBean(realm, theme, uriInfo.getBaseUri(), null));

      String subject =
          new MessageFormat(messages.getProperty(subjectKey, subjectKey), locale)
              .format(subjectAttributes.toArray());
      String textTemplate = String.format("text/%s", template);
      String textBody;
      FreeMarkerProvider freeMarker = session.getProvider(FreeMarkerProvider.class);
      try {
        textBody = freeMarker.processTemplate(attributes, textTemplate, theme);
      } catch (final FreeMarkerException e) {
        throw new EmailException("Failed to template plain text email.", e);
      }
      String htmlTemplate = String.format("html/%s", template);
      String htmlBody;
      try {
        htmlBody = freeMarker.processTemplate(attributes, htmlTemplate, theme);
      } catch (final FreeMarkerException e) {
        throw new EmailException("Failed to template html email.", e);
      }
      return new EmailTemplate(subject, textBody, htmlBody);
    } catch (Exception e) {
      throw new EmailException("Failed to template email", e);
    }
  }

  protected String sendEmail(
      KeycloakSession session,
      RealmModel realm,
      UserModel user,
      String subjectFormatKey,
      List<Object> subjectAttributes,
      String bodyTemplate,
      Map<String, Object> bodyAttributes,
      String address,
      boolean useEmailSender,
      String username)
      throws EmailException {
    try {
      EmailTemplate emailTemplate =
          processEmailTemplate(
              session,
              realm,
              user,
              subjectFormatKey,
              subjectAttributes,
              bodyTemplate,
              bodyAttributes);

      if (useEmailSender) {
        EmailSenderProvider emailSender = session.getProvider(EmailSenderProvider.class);
        emailSender.send(
            realm.getSmtpConfig(),
            address,
            emailTemplate.getSubject(),
            emailTemplate.getTextBody(),
            emailTemplate.getHtmlBody());

      } else {
        EmailTemplateProvider emailTemplateProvider =
            session.getProvider(EmailTemplateProvider.class);
        String realmName = getRealmName(realm);
        emailTemplateProvider.setRealm(realm).setUser(user).setAttribute("realmName", realmName);

        if (username != null && !username.isEmpty()) {
          emailTemplateProvider.setAttribute("username", username);
        }

        emailTemplateProvider.send(
            subjectFormatKey, subjectAttributes, bodyTemplate, bodyAttributes);
      }

      return String.format(
          "{\"to\": \"%s\", \"subject\": \"%s\", \"textBody\": \"%s\", \"htmlBody\": \"%s\"}",
          escapeJson(address),
          escapeJson(emailTemplate.getSubject()),
          escapeJson(emailTemplate.getTextBody()),
          escapeJson(emailTemplate.getHtmlBody() != null ? emailTemplate.getHtmlBody() : ""));
    } catch (EmailException e) {
      throw e;
    } catch (Exception e) {
      throw new EmailException("Failed to template email", e);
    }
  }

  /**
   * Tells support that a registration matched no voter. Nothing goes to the voter: the address or
   * number typed in the form is unverified, and events don't carry it (R3-06, R7-18).
   */
  public static void sendRegistrationErrorToSupport(
      KeycloakSession session, String realmId, Event event) throws EmailException {
    sendSupportNotificationEmail(session, realmId, event);
  }

  // Sending support email with event details
  protected static void sendSupportNotificationEmail(
      KeycloakSession session, String realmId, Event event) throws EmailException {
    try {
      RealmModel realm = session.realms().getRealm(realmId);

      String supportEmail = "no-reply@sequentech.io";

      Map<String, Object> attributes = new HashMap<>();
      attributes.put("event", event);

      List<Object> subjectAttributes = Collections.emptyList();

      sendEmail(
          session,
          realm,
          null,
          SEND_SUPPORT_ERROR_EMAIL_SUBJECT,
          subjectAttributes,
          SEND_SUPPORT_ERROR_EMAIL_FTL,
          attributes,
          supportEmail,
          /* useEmailSender */ true,
          /* username */ null);

      log.info("Support notification email sent to: " + supportEmail);
    } catch (EmailException error) {
      log.error("sendSupportNotificationEmail(): Exception sending email", error);
      throw error;
    }
  }

  protected static class EmailTemplate {

    private String subject;
    private String textBody;
    private String htmlBody;

    public EmailTemplate(String subject, String textBody, String htmlBody) {
      this.subject = subject;
      this.textBody = textBody;
      this.htmlBody = htmlBody;
    }

    public String getSubject() {
      return subject;
    }

    public String getTextBody() {
      return textBody;
    }

    public String getHtmlBody() {
      return htmlBody;
    }
  }

  protected static String getOtpAddress(
      Utils.MessageCourier courier,
      boolean deferredUser,
      AuthenticatorConfigModel config,
      AuthenticationSessionModel authSession,
      UserModel user) {
    String mobileNumber = null;
    String emailAddress = null;

    if (courier == Utils.MessageCourier.CHOSEN) {
      // The channel the code went to, read below.
    } else if (deferredUser) {
      String mobileNumberAttribute = config.getConfig().get(Utils.TEL_USER_ATTRIBUTE);
      mobileNumber = authSession.getAuthNote(mobileNumberAttribute);
      emailAddress = authSession.getAuthNote("email");
    } else {
      mobileNumber = Utils.getMobile(config, user);
      emailAddress = user.getEmail();
    }
    // Masked like every record, log and screen (R7-11: Rust's rule).
    switch (courier) {
      case EMAIL:
        return VoterChannels.mask(MessageChannel.EMAIL, emailAddress);
      case SMS:
        return VoterChannels.mask(MessageChannel.SMS, mobileNumber);
      case BOTH:
        return emailAddress != null && !emailAddress.isEmpty()
            ? VoterChannels.mask(MessageChannel.EMAIL, emailAddress)
            : VoterChannels.mask(MessageChannel.SMS, mobileNumber);
      case CHOSEN:
        Optional<MessageChannel> channel =
            MessageChannel.parse(authSession.getAuthNote(MessagingAttributes.NOTE_CODE_CHANNEL));
        if (channel.isEmpty()) {
          return "";
        }
        return VoterChannels.display(
            channel.get(), authSession.getAuthNote(MessagingAttributes.NOTE_CODE_DESTINATION));
      default:
        break;
    }
    return emailAddress;
  }

  /** The masked number, as {@link VoterChannels#mask} shows it (R7-11). */
  public static String obscurePhoneNumber(String phoneNumber) {
    return phoneNumber == null ? null : VoterChannels.mask(MessageChannel.SMS, phoneNumber);
  }

  /** The masked email address, as {@link VoterChannels#mask} shows it (R7-11). */
  public static String obscureEmail(String email) {
    return email == null ? null : VoterChannels.mask(MessageChannel.EMAIL, email);
  }

  /**
   * Gets the tenant id from the realm name
   *
   * @param session
   * @param realmId
   * @return Tenant id found in the realm name or null if it wasn't present
   */
  public String getTenantId(KeycloakSession session, String realmId) {
    String realmName = session.realms().getRealm(realmId).getName();

    // Regular expression to match a UUID pattern
    Pattern uuidPattern =
        Pattern.compile(
            "\\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\\b");
    Matcher matcher = uuidPattern.matcher(realmName);

    // Find the first match
    return matcher.find() ? matcher.group() : null;
  }

  public String getElectionEventId(KeycloakSession session, String realmId) {
    String realmName = session.realms().getRealm(realmId).getName();
    String[] parts = realmName.split("event-");
    if (parts.length > 1) {
      return parts[1];
    }
    return null;
  }

  public String buildAuthUrl(KeycloakSession session, String realmId, String urlType) {
    String tenantId = getTenantId(session, realmId);
    String electionEventId = getElectionEventId(session, realmId);
    String baseUrl =
        session.getContext().getRealm().getClientByClientId("voting-portal").getRootUrl();
    if (baseUrl.endsWith("/")) {
      baseUrl = baseUrl.substring(0, baseUrl.length() - 1);
    }

    if (tenantId != null && electionEventId != null) {
      return String.format("%s/tenant/%s/event/%s/%s", baseUrl, tenantId, electionEventId, urlType);
    } else {
      log.warn("Tenant ID or Election Event ID is null");
      return null;
    }
  }

  public static String getClientName(Object context) {
    AuthenticationSessionModel authSession = null;
    if (context instanceof AuthenticationFlowContext) {
      authSession = ((AuthenticationFlowContext) context).getAuthenticationSession();
    } else if (context instanceof ActionTokenContext) {
      authSession = ((ActionTokenContext<?>) context).getAuthenticationSession();
    } else {
      throw new IllegalArgumentException("Unsupported context type");
    }
    return authSession.getClient().getName();
  }

  /**
   * What Keycloak's own email and SMS providers still send for a notice: the courier and the
   * addresses they use.
   */
  public record KeycloakNotice(MessageCourier courier, String email, String mobileNumber) {}

  /**
   * What happened to a notice. Refusals and failures are never collapsed into "handled": a caller
   * that must know whether the voter may have received it (a temporary password, a reset link)
   * reads {@link #handedOver()} and {@link #reason()}.
   *
   * @param kind who took the notice, or that nobody did
   * @param state harvest's answer, for {@link Kind#HANDED_OVER}
   * @param reason why it was refused or failed; the one reason string harvest or Keycloak gave
   * @param keycloak what Keycloak's own providers must still send, for {@link Kind#KEYCLOAK}
   */
  public record NoticeOutcome(
      Kind kind,
      Optional<MessageAttemptState> state,
      Optional<String> reason,
      Optional<KeycloakNotice> keycloak) {

    public enum Kind {
      /** Harvest answered; {@link #state} says how. */
      HANDED_OVER,
      /** Keycloak's own email or SMS provider must send it: see {@link #keycloak}. */
      KEYCLOAK,
      /** Nobody sends it; {@link #reason} says why. */
      NOT_SENT
    }

    static NoticeOutcome handedOver(MessageAttemptState state, String reason) {
      return new NoticeOutcome(
          Kind.HANDED_OVER,
          Optional.of(state),
          Optional.ofNullable(reason).filter(value -> !value.isBlank()),
          Optional.empty());
    }

    static NoticeOutcome keycloak(KeycloakNotice notice) {
      return new NoticeOutcome(
          Kind.KEYCLOAK, Optional.empty(), Optional.empty(), Optional.of(notice));
    }

    static NoticeOutcome notSent(String reason) {
      return new NoticeOutcome(
          Kind.NOT_SENT, Optional.empty(), Optional.of(reason), Optional.empty());
    }

    /**
     * Whether harvest took the notice or may have sent it: ACCEPTED, QUEUED, DELIVERED or UNKNOWN.
     * FAILED and refusals are not; neither is a notice Keycloak's providers still have to send.
     */
    public boolean handedOver() {
      return kind == Kind.HANDED_OVER
          && state.orElse(MessageAttemptState.FAILED) != MessageAttemptState.FAILED;
    }

    /** The reason to record when the notice did not go out; a generic one when none was given. */
    public String failureReason() {
      return reason.orElse(state.map(Enum::name).orElse("not sent"));
    }
  }

  /**
   * Sends a notice on the voter's one channel, and nowhere else.
   *
   * <ul>
   *   <li>A channel the message sender (harvest) carries is always handed to it; harvest decides
   *       whether the event uses the channel for notices, refuses with its reason and records the
   *       outcome. A voter without a channel is handed over too, without one, so harvest records
   *       "No contact channel". FAILED and UNKNOWN are never sent elsewhere.
   *   <li>Email and SMS that the sender doesn't carry use Keycloak's providers: the outcome says
   *       which. With {@link MessageCourier#CHOSEN} only the voter's channel. A realm configured
   *       with another courier (EMAIL, SMS, BOTH) keeps it for LEGACY voters only (no messaging
   *       channel in the record): that courier is the realm's explicit policy, and it is logged. A
   *       voter whose recorded channel can't be used gets nothing by that courier (R7-01).
   *   <li>A channel nothing can carry is not sent, and the reason is logged.
   * </ul>
   */
  public static NoticeOutcome sendNotice(
      KeycloakSession session,
      RealmModel realm,
      UserModel user,
      NoticeRecipient recipient,
      MessageCourier configured,
      String email,
      String mobileNumber,
      String textKey,
      Supplier<List<String>> attributes,
      Object context)
      throws IOException {
    return sendNotice(
        session,
        realm,
        user,
        recipient,
        configured,
        email,
        mobileNumber,
        textKey,
        attributes,
        context,
        null);
  }

  /**
   * {@link #sendNotice} to a chosen recipient: {@link MessageRecipient#PREVIOUS_CHANNEL} for the
   * notice to the channel the voter just replaced; null for the voter's own channel.
   */
  public static NoticeOutcome sendNotice(
      KeycloakSession session,
      RealmModel realm,
      UserModel user,
      NoticeRecipient recipient,
      MessageCourier configured,
      String email,
      String mobileNumber,
      String textKey,
      Supplier<List<String>> attributes,
      Object context,
      MessageRecipient recipientKind)
      throws IOException {
    MessageSenderProvider sender = VoterChannels.sender(session);
    Optional<VoterChannel> channel = recipient.channel();
    if (configured != MessageCourier.CHOSEN && recipient.shape() == VoterChannel.Source.LEGACY) {
      // A realm that doesn't use the voter's channel, for a voter without a messaging channel.
      log.infov(
          "sendNotice(): {0} notice by the realm's configured courier {1} (the voter has no"
              + " messaging channel)",
          textKey, configured);
      return NoticeOutcome.keycloak(new KeycloakNotice(configured, email, mobileNumber));
    }
    NoticeRecipient.Delivery delivery = recipient.delivery(sender);
    switch (delivery.route()) {
      case SENDER -> {
        List<String> parameters = attributes.get();
        String text = formatLoginMessage(session, realm, user, textKey, parameters);
        MessageChannel name = channel.map(VoterChannel::channel).orElse(null);
        String destination = channel.map(VoterChannel::destination).orElse(null);
        SendMessageResponse response =
            sender.send(
                new SendMessageRequest(
                    tenantId(realm),
                    electionEventId(realm),
                    recipient.voterId(),
                    name,
                    MessagePurpose.NOTICE,
                    destination,
                    languageCode(session, realm, user),
                    new MessageContent(null, text, null, parameters, null),
                    "notice:" + UUID.randomUUID(),
                    null,
                    textKey,
                    List.copyOf(recipient.electionIds()),
                    recipientKind));
        MessageAttemptState state = response.attemptState();
        if (state == MessageAttemptState.FAILED) {
          log.warnv(
              "sendNotice(): {0} notice on {1} not sent: {2}; not sent elsewhere",
              textKey, name, response.reason());
        }
        // Harvest writes the attempt's one electoral-log entry (R2-07): none here.
        return NoticeOutcome.handedOver(state, response.reason());
      }
      case KEYCLOAK -> {
        VoterChannel target = channel.get();
        return NoticeOutcome.keycloak(
            target.channel() == MessageChannel.EMAIL
                ? new KeycloakNotice(MessageCourier.EMAIL, target.destination(), null)
                : new KeycloakNotice(MessageCourier.SMS, null, target.destination()));
      }
      default -> {
        String reason = delivery.refusal().map(MessageReason::reason).orElse("not sent");
        log.warnv("sendNotice(): {0} notice not sent: {1}", textKey, reason);
        communicationsLog(
            context,
            sanitizedSend(
                channel.map(VoterChannel::channel).orElse(null),
                channel.map(c -> VoterChannels.mask(c.channel(), c.destination())).orElse(""),
                null,
                MessageAttemptState.FAILED,
                reason));
        return NoticeOutcome.notSent(reason);
      }
    }
  }

  /**
   * Records a message Keycloak's own email or SMS provider sent: the channel and the masked
   * destination, never its content (it may hold links, usernames or numbers).
   */
  static void keycloakSent(Object context, MessageChannel channel, String destination) {
    communicationsLog(
        context,
        sanitizedSend(
            channel, VoterChannels.mask(channel, destination), null, MessageAttemptState.ACCEPTED));
  }

  public static void sendConfirmation(
      KeycloakSession session,
      RealmModel realm,
      UserModel user,
      MessageCourier messageCourier,
      String mobileNumber,
      Object context)
      throws EmailException, IOException {
    sendConfirmation(
        session,
        realm,
        user,
        messageCourier,
        mobileNumber,
        NoticeRecipient.fromUser(user, PHONE_NUMBER_ATTRIBUTE),
        context);
  }

  public static void sendConfirmation(
      KeycloakSession session,
      RealmModel realm,
      UserModel user,
      MessageCourier messageCourier,
      String mobileNumber,
      NoticeRecipient recipient,
      Object context)
      throws EmailException, IOException {
    log.info("sendConfirmation(): start");
    String username = user.getEmail() != null ? user.getEmail() : mobileNumber;
    Optional<KeycloakNotice> keycloak =
        sendNotice(
                session,
                realm,
                user,
                recipient,
                messageCourier,
                user.getEmail(),
                mobileNumber,
                successSmsKey(context),
                () ->
                    List.of(
                        buildAuthUrl(session, realm.getId(), "login"), String.valueOf(username)),
                context)
            .keycloak();
    if (keycloak.isEmpty()) {
      return;
    }
    messageCourier = keycloak.get().courier();
    mobileNumber = keycloak.get().mobileNumber();

    String realName = realm.getName();
    // Send a confirmation email
    EmailTemplateProvider emailTemplateProvider = session.getProvider(EmailTemplateProvider.class);

    log.infov("sendConfirmation(): messageCourier {0}", messageCourier);

    String email = keycloak.get().email();

    if (email != null
        && email.trim().length() > 0
        && (MessageCourier.EMAIL.equals(messageCourier)
            || MessageCourier.BOTH.equals(messageCourier))) {
      log.info("sendConfirmation(): sending email");
      List<Object> subjAttr = ImmutableList.of(realName);
      Map<String, Object> messageAttributes = Maps.newHashMap();
      messageAttributes.put("realmName", realName);
      messageAttributes.put("username", username);
      messageAttributes.put("enrollmentUrl", buildAuthUrl(session, realm.getId(), "enroll"));
      messageAttributes.put("loginUrl", buildAuthUrl(session, realm.getId(), "login"));
      messageAttributes.put("isKiosk", getClientName(context).endsWith("-kiosk"));
      String textBody =
          sendEmail(
              session,
              realm,
              user,
              SEND_SUCCESS_EMAIL_SUBJECT,
              subjAttr,
              SEND_SUCCESS_EMAIL_FTL,
              messageAttributes,
              email.trim(),
              false,
              username);
      keycloakSent(context, MessageChannel.EMAIL, email.trim());
    }

    if (mobileNumber != null
        && mobileNumber.trim().length() > 0
        && (MessageCourier.SMS.equals(messageCourier)
            || MessageCourier.BOTH.equals(messageCourier))) {
      log.info("sendConfirmation(): sending sms");
      SmsSenderProvider smsSenderProvider = session.getProvider(SmsSenderProvider.class);
      String url = buildAuthUrl(session, realm.getId(), "login");
      List<String> smsAttributes = ImmutableList.of(url, mobileNumber.trim());
      String smsTranslationKey =
          getClientName(context).endsWith("-kiosk")
              ? SEND_SUCCESS_SMS_I18N_KEY_KIOSK
              : SEND_SUCCESS_SMS_I18N_KEY;
      String formattedText =
          smsSenderProvider.send(
              mobileNumber.trim(), smsTranslationKey, smsAttributes, realm, user, session);
      keycloakSent(context, MessageChannel.SMS, mobileNumber.trim());
    }
  }

  private static String successSmsKey(Object context) {
    return getClientName(context).endsWith("-kiosk")
        ? SEND_SUCCESS_SMS_I18N_KEY_KIOSK
        : SEND_SUCCESS_SMS_I18N_KEY;
  }

  public static void sendConfirmationDiffPost(
      KeycloakSession session,
      RealmModel realm,
      UserModel user,
      MessageCourier messageCourier,
      String mobileNumber,
      Object context)
      throws EmailException, IOException {
    sendConfirmationDiffPost(
        session,
        realm,
        user,
        messageCourier,
        mobileNumber,
        NoticeRecipient.fromUser(user, PHONE_NUMBER_ATTRIBUTE),
        context);
  }

  public static void sendConfirmationDiffPost(
      KeycloakSession session,
      RealmModel realm,
      UserModel user,
      MessageCourier messageCourier,
      String mobileNumber,
      NoticeRecipient recipient,
      Object context)
      throws EmailException, IOException {
    log.info("sendConfirmationDiffPost(): start");
    // We get the username we are going to provide the user in other to login. It's
    // going to be
    // either email or mobileNumber.
    String username = user.getEmail() != null ? user.getEmail() : mobileNumber;
    Optional<KeycloakNotice> keycloak =
        sendNotice(
                session,
                realm,
                user,
                recipient,
                messageCourier,
                user.getEmail(),
                mobileNumber,
                successSmsKey(context),
                () ->
                    List.of(
                        buildAuthUrl(session, realm.getId(), "login"), String.valueOf(username)),
                context)
            .keycloak();
    if (keycloak.isEmpty()) {
      return;
    }
    messageCourier = keycloak.get().courier();
    mobileNumber = keycloak.get().mobileNumber();

    String realName = realm.getName();
    // Send a confirmation email
    EmailTemplateProvider emailTemplateProvider = session.getProvider(EmailTemplateProvider.class);

    log.infov("sendConfirmationDiffPost(): messageCourier {0}", messageCourier);

    String email = keycloak.get().email();
    String embassy = user.getFirstAttribute("embassy");

    if (email != null
        && email.trim().length() > 0
        && (MessageCourier.EMAIL.equals(messageCourier)
            || MessageCourier.BOTH.equals(messageCourier))) {
      log.info("sendConfirmationDiffPost(): sending email");
      List<Object> subjAttr = ImmutableList.of(realName);
      Map<String, Object> messageAttributes = Maps.newHashMap();
      messageAttributes.put("realmName", realName);
      messageAttributes.put("username", username);
      messageAttributes.put("embassy", embassy);
      messageAttributes.put("enrollmentUrl", buildAuthUrl(session, realm.getId(), "enroll"));
      messageAttributes.put("loginUrl", buildAuthUrl(session, realm.getId(), "login"));

      String textBody =
          sendEmail(
              session,
              realm,
              user,
              SEND_SUCCESS_EMAIL_SUBJECT,
              subjAttr,
              SEND_SUCCESS_EMAIL_DIFF_POST_FTL,
              messageAttributes,
              email.trim(),
              false,
              username);
      keycloakSent(context, MessageChannel.EMAIL, email.trim());
    }

    if (mobileNumber != null
        && mobileNumber.trim().length() > 0
        && (MessageCourier.SMS.equals(messageCourier)
            || MessageCourier.BOTH.equals(messageCourier))) {
      log.info("sendConfirmation(): sending sms");

      SmsSenderProvider smsSenderProvider = session.getProvider(SmsSenderProvider.class);
      String url = buildAuthUrl(session, realm.getId(), "login");
      List<String> smsAttributes = ImmutableList.of(url, mobileNumber.trim());
      String smsTranslationKey =
          getClientName(context).endsWith("-kiosk")
              ? SEND_SUCCESS_SMS_I18N_KEY_KIOSK
              : SEND_SUCCESS_SMS_I18N_KEY;
      String formattedText =
          smsSenderProvider.send(
              mobileNumber.trim(), smsTranslationKey, smsAttributes, realm, user, session);
      keycloakSent(context, MessageChannel.SMS, mobileNumber.trim());
    }
  }

  public static String convertToString(
      HashMap<String, String> values, String separator, String format) {
    log.info("convertToString(): start");
    if (values == null || values.isEmpty()) {
      return ""; // Return an empty string if the map is null or empty
    }
    log.info("convertToString(): not empty");

    // Default format: "key: value"
    String entryFormat = (format != null && !format.isEmpty()) ? format : "%s: %s";

    return values.entrySet().stream()
        // Format each entry using String.format with the provided or default format
        .map(
            entry -> {
              String val = String.format(entryFormat, entry.getKey(), entry.getValue());
              return val;
            })
        // Join entries with the specified separator
        .collect(Collectors.joining(separator));
  }

  public static void sendManualCommunication(
      KeycloakSession session,
      RealmModel realm,
      MessageCourier messageCourier,
      String email,
      String mobileNumber,
      String rejectReasonKey,
      HashMap<String, String> mismatchedFields,
      Object context)
      throws EmailException, IOException {
    sendManualCommunication(
        session,
        realm,
        messageCourier,
        email,
        mobileNumber,
        rejectReasonKey,
        mismatchedFields,
        NoticeRecipient.none(),
        context);
  }

  public static void sendManualCommunication(
      KeycloakSession session,
      RealmModel realm,
      MessageCourier messageCourier,
      String email,
      String mobileNumber,
      String rejectReasonKey,
      HashMap<String, String> mismatchedFields,
      NoticeRecipient recipient,
      Object context)
      throws EmailException, IOException {
    log.info("sendManualCommunication(): start");
    Optional<KeycloakNotice> keycloak =
        sendNotice(
                session,
                realm,
                null,
                recipient,
                messageCourier,
                email,
                mobileNumber,
                SEND_PENDING_SMS_I18N_KEY,
                () ->
                    List.of(
                        String.valueOf(rejectReasonKey),
                        convertToString(mismatchedFields, ", ", null)),
                context)
            .keycloak();
    if (keycloak.isEmpty()) {
      return;
    }
    messageCourier = keycloak.get().courier();
    mobileNumber = keycloak.get().mobileNumber();
    email = keycloak.get().email();

    String realName = realm.getName();
    // Send a confirmation email
    EmailTemplateProvider emailTemplateProvider = session.getProvider(EmailTemplateProvider.class);

    // We get the username we are going to provide the user in other to login. It's
    // going to be
    // either email or mobileNumber.
    String username = email != null ? email : mobileNumber;
    log.infov("sendManualCommunication(): messageCourier {0}", messageCourier);

    if (email != null
        && email.trim().length() > 0
        && (MessageCourier.EMAIL.equals(messageCourier)
            || MessageCourier.BOTH.equals(messageCourier))) {
      log.info("sendManualCommunication(): sending email");
      List<Object> subjAttr = ImmutableList.of(realName);
      Map<String, Object> messageAttributes = Maps.newHashMap();
      messageAttributes.put("rejectReasonKey", rejectReasonKey);
      messageAttributes.put(
          "mismatchedFieldsPlain", convertToString(mismatchedFields, "\n", "- %s: %s"));
      messageAttributes.put(
          "mismatchedFieldsHtml",
          convertToString(mismatchedFields, "<br>", "- %s: <strong>%s</strong>"));

      String textBody =
          sendEmail(
              session,
              realm,
              null,
              SEND_PENDING_EMAIL_SUBJECT,
              subjAttr,
              SEND_PENDING_EMAIL_FTL,
              messageAttributes,
              email.trim(),
              true,
              username);
      keycloakSent(context, MessageChannel.EMAIL, email.trim());
    }

    if (mobileNumber != null
        && mobileNumber.trim().length() > 0
        && (MessageCourier.SMS.equals(messageCourier)
            || MessageCourier.BOTH.equals(messageCourier))) {
      log.info("sendManualCommunication(): sending sms");

      SmsSenderProvider smsSenderProvider = session.getProvider(SmsSenderProvider.class);
      List<String> smsAttributes =
          ImmutableList.of(rejectReasonKey, convertToString(mismatchedFields, ", ", null));

      String formattedText =
          smsSenderProvider.send(
              mobileNumber.trim(), SEND_PENDING_SMS_I18N_KEY, smsAttributes, realm, null, session);
      keycloakSent(context, MessageChannel.SMS, mobileNumber.trim());
    }
  }

  public static void sendRejectCommunication(
      KeycloakSession session,
      RealmModel realm,
      MessageCourier messageCourier,
      String email,
      String mobileNumber,
      String rejectReasonKey,
      HashMap<String, String> mismatchedFields,
      Object context)
      throws EmailException, IOException {
    sendRejectCommunication(
        session,
        realm,
        messageCourier,
        email,
        mobileNumber,
        rejectReasonKey,
        mismatchedFields,
        NoticeRecipient.none(),
        context);
  }

  public static void sendRejectCommunication(
      KeycloakSession session,
      RealmModel realm,
      MessageCourier messageCourier,
      String email,
      String mobileNumber,
      String rejectReasonKey,
      HashMap<String, String> mismatchedFields,
      NoticeRecipient recipient,
      Object context)
      throws EmailException, IOException {
    log.info("sendRejectCommunication(): start");
    Optional<KeycloakNotice> keycloak =
        sendNotice(
                session,
                realm,
                null,
                recipient,
                messageCourier,
                email,
                mobileNumber,
                SEND_REJECT_SMS_I18N_KEY,
                () ->
                    List.of(
                        String.valueOf(rejectReasonKey),
                        convertToString(mismatchedFields, ", ", null)),
                context)
            .keycloak();
    if (keycloak.isEmpty()) {
      return;
    }
    messageCourier = keycloak.get().courier();
    mobileNumber = keycloak.get().mobileNumber();
    email = keycloak.get().email();

    String realName = realm.getName();
    // Send a confirmation email
    EmailTemplateProvider emailTemplateProvider = session.getProvider(EmailTemplateProvider.class);

    // We get the username we are going to provide the user in other to login. It's
    // going to be
    // either email or mobileNumber.
    String username = email != null ? email : mobileNumber;
    log.infov("sendRejectCommunication(): messageCourier {0}", messageCourier);

    if (email != null
        && email.trim().length() > 0
        && (MessageCourier.EMAIL.equals(messageCourier)
            || MessageCourier.BOTH.equals(messageCourier))) {
      log.info("sendRejectCommunication(): sending email");
      List<Object> subjAttr = ImmutableList.of(realName);
      Map<String, Object> messageAttributes = Maps.newHashMap();
      messageAttributes.put("rejectReasonKey", rejectReasonKey);
      messageAttributes.put("missmatchedFields", convertToString(mismatchedFields, ", ", null));

      String textBody =
          sendEmail(
              session,
              realm,
              null,
              SEND_REJECT_EMAIL_SUBJECT,
              subjAttr,
              SEND_REJECT_EMAIL_FTL,
              messageAttributes,
              email.trim(),
              true,
              username);
      keycloakSent(context, MessageChannel.EMAIL, email.trim());
    }

    if (mobileNumber != null
        && mobileNumber.trim().length() > 0
        && (MessageCourier.SMS.equals(messageCourier)
            || MessageCourier.BOTH.equals(messageCourier))) {
      log.info("sendRejectCommunication(): sending sms");

      SmsSenderProvider smsSenderProvider = session.getProvider(SmsSenderProvider.class);
      List<String> smsAttributes =
          ImmutableList.of(rejectReasonKey, convertToString(mismatchedFields, ", ", null));

      String formattedText =
          smsSenderProvider.send(
              mobileNumber.trim(), SEND_REJECT_SMS_I18N_KEY, smsAttributes, realm, null, session);
      keycloakSent(context, MessageChannel.SMS, mobileNumber.trim());
    }
  }

  public void buildEventDetails(AuthenticationFlowContext context, String className) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    UserModel user = context.getUser();
    List<UPAttribute> realmsAttributesList = getRealmUserProfileAttributes(context.getSession());
    for (UPAttribute attribute : realmsAttributesList) {
      if (CODE.equals(attribute.getName())) {
        continue;
      }
      String authNoteValue = authSession.getAuthNote(attribute.getName());
      // Events reach the electoral log: never a voter's number or email address.
      if (ContactDetails.isContact(attribute.getName(), authNoteValue)) {
        continue;
      }
      context.getEvent().detail(attribute.getName(), authNoteValue);
    }
    if (user != null) {
      context.getEvent().detail(USER_PROFILE_ATTRIBUTES, getUserAttributesString(user));
      context.getEvent().user(user.getId());
    } else {
      String userId = context.getAuthenticationSession().getAuthNote(USER_ID);
      context.getEvent().user(userId);
    }
    context.getEvent().detail(AUTHENTICATOR_CLASS_NAME, className);
  }

  public String getUserAttributesString(UserModel user) {
    Map<String, List<String>> attributes = user.getAttributes();
    ObjectMapper mapper = new ObjectMapper();
    ObjectNode attributesJson = mapper.createObjectNode();

    for (String attributeName : attributes.keySet()) {
      List<String> values = attributes.get(attributeName);
      String value = values == null || values.isEmpty() ? null : values.get(0);
      if (ContactDetails.isContact(attributeName, value)) {
        continue;
      }
      attributesJson.put(attributeName, value);
    }

    return attributesJson.toString();
  }

  public List<UPAttribute> getRealmUserProfileAttributes(KeycloakSession session) {
    UserProfileProvider userProfileProvider = session.getProvider(UserProfileProvider.class);
    UPConfig userProfileConfig = userProfileProvider.getConfiguration();
    return userProfileConfig.getAttributes();
  }
}
