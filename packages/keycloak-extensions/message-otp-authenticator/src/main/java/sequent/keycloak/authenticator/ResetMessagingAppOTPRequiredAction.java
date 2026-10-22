// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.Response;
import java.time.Instant;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.function.Consumer;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.InitiatedActionSupport;
import org.keycloak.authentication.RequiredActionContext;
import org.keycloak.authentication.RequiredActionFactory;
import org.keycloak.authentication.RequiredActionProvider;
import org.keycloak.email.EmailTemplateProvider;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.RequiredActionConfigModel;
import org.keycloak.models.UserModel;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.sessions.AuthenticationSessionModel;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialProvider;
import sequent.keycloak.authenticator.gateway.SmsSenderProvider;
import sequent.keycloak.authenticator.messaging.MessageAttemptState;
import sequent.keycloak.authenticator.messaging.MessageChannel;
import sequent.keycloak.authenticator.messaging.MessageReason;
import sequent.keycloak.authenticator.messaging.MessageRecipient;
import sequent.keycloak.authenticator.messaging.MessagingAttributes;
import sequent.keycloak.authenticator.messaging.MessengerLinkStatus;
import sequent.keycloak.authenticator.messaging.NoticeRecipient;
import sequent.keycloak.authenticator.messaging.PublicMessagingChannels;
import sequent.keycloak.authenticator.messaging.VerifiedContacts;
import sequent.keycloak.authenticator.messaging.VoterChannel;
import sequent.keycloak.authenticator.messaging.VoterChannels;

/**
 * "Change how we contact you" ({@code message-channel-ra}): an authenticated voter replaces their
 * one channel with another of their Post's channels.
 *
 * <ol>
 *   <li>The voter picks the channel, enters the number (WhatsApp, Viber, SMS) and agrees to its
 *       messages. Email uses the account's address; Messenger needs no number.
 *   <li>A code goes to that contact; Messenger uses a one-time link and Get Started.
 *   <li>A correct code replaces all five channel attributes, the way enrollment saves them.
 * </ol>
 *
 * The action needs a recent sign-in (max auth age, 300 s by default), so holding an open session is
 * not enough to take over a voter's codes. Codes follow this action's own configuration: lifetime,
 * resend timer and the attempt limit, which a resend never resets.
 */
@AutoService(RequiredActionFactory.class)
@JBossLog
public class ResetMessagingAppOTPRequiredAction extends BaseResetMessageOTPRequiredAction
    implements RequiredActionFactory {
  public static final String PROVIDER_ID = "message-channel-ra";

  /** The first release's id, still served (see {@link LegacyMessagingAppOTPRequiredAction}). */
  public static final String LEGACY_PROVIDER_ID = "messaging-app-otp-ra";

  /** Seconds since sign-in after which the voter must sign in again to change the contact. */
  public static final int MAX_AUTH_AGE_SECONDS = 300;

  static final String CHANNEL_PARAM = "channel";
  static final String CONTACT_PARAM = "contact";
  static final String ENTRY_FTL = "message-otp.enter-app-contact.ftl";

  @Override
  protected String getProviderId() {
    return getId();
  }

  /** The channel whose code is pending; no note means the voter is still choosing. */
  @Override
  protected String getNoteKey(AuthenticationSessionModel authSession) {
    return MessagingAttributes.NOTE_RESET_CHANNEL;
  }

  @Override
  protected Utils.MessageCourier getCourier() {
    return Utils.MessageCourier.CHOSEN;
  }

  @Override
  protected String getI18nPrefix() {
    return "resetAppOtp";
  }

  @Override
  protected String getEntryFtl() {
    return ENTRY_FTL;
  }

  /**
   * This action's own code settings: the message OTP defaults, overridden by the configuration of
   * this action in the realm. They never come from a sign-in or enrollment step.
   */
  @Override
  protected AuthenticatorConfigModel codeConfig(RequiredActionContext context) {
    Map<String, String> config = new HashMap<>(MessageOTPAuthenticatorFactory.getConfigMap(null));
    RequiredActionConfigModel own =
        context.getRealm() == null
            ? null
            : context.getRealm().getRequiredActionConfigByAlias(getProviderId());
    if (own != null && own.getConfig() != null) {
      config.putAll(own.getConfig());
    }
    config.put(Utils.MESSAGE_COURIER_ATTRIBUTE, Utils.MessageCourier.CHOSEN.name());
    config.putIfAbsent(Utils.TEL_USER_ATTRIBUTE, MessageOTPAuthenticator.MOBILE_NUMBER_FIELD);
    AuthenticatorConfigModel model = new AuthenticatorConfigModel();
    model.setAlias(getProviderId());
    model.setConfig(config);
    return model;
  }

  @Override
  public void requiredActionChallenge(RequiredActionContext context) {
    if (canRun(context)) {
      takeOver(context.getAuthenticationSession());
      super.requiredActionChallenge(context);
    }
  }

  @Override
  public void processAction(RequiredActionContext context) {
    if (canRun(context)) {
      takeOver(context.getAuthenticationSession());
      super.processAction(context);
    }
  }

  /**
   * The first time the action runs in an authentication session, it forgets the sign-in's code: its
   * resend timer, delivery state and destination belong to the sign-in challenge (already passed),
   * not to the new contact's. The action's wrong codes have their own count ({@link #challenge()}).
   */
  static void takeOver(AuthenticationSessionModel authSession) {
    if (authSession.getAuthNote(MessagingAttributes.NOTE_CONTACT_CHANGE_STARTED) != null) {
      return;
    }
    for (String note :
        List.of(
            Utils.CODE,
            Utils.CODE_TTL,
            MessagingAttributes.NOTE_CODE_ID,
            MessagingAttributes.NOTE_CODE_CHANNEL,
            MessagingAttributes.NOTE_CODE_DESTINATION,
            MessagingAttributes.NOTE_DELIVERY_STATE)) {
      authSession.removeAuthNote(note);
    }
    Utils.clearMessengerLink(authSession);
    authSession.setAuthNote(MessagingAttributes.NOTE_CONTACT_CHANGE_STARTED, "true");
  }

  /**
   * Runs only when {@link ContactChange#forVoter} offers the change: the same decision the portal's
   * entry shows (R7-14). A refusal is logged and goes into the event with its reason, and the
   * action {@code ignore()}s: for an application-initiated action Keycloak then reports {@code
   * kc_action_status=error} to the portal, which tells the voter it didn't work (checked against
   * Keycloak 26.6's {@code AuthenticationManager}); {@code failure()} would answer the portal with
   * an OAuth error instead.
   */
  private boolean canRun(RequiredActionContext context) {
    if (context.getUser() == null) {
      context.failure();
      return false;
    }
    ContactChange.Decision decision =
        ContactChange.forVoter(context.getSession(), context.getRealm(), context.getUser());
    if (!decision.offered()) {
      String reason =
          decision.refusal().orElse(MessageReason.CHANNEL_NOT_OFFERED_FOR_CODES).reason();
      log.infov("{0}: not run: {1}", getProviderId(), reason);
      context.getEvent().detail("reason", reason);
      context.getAuthenticationSession().removeRequiredAction(getProviderId());
      context.ignore();
      return false;
    }
    return true;
  }

  /**
   * The channels the voter's Post offers codes on, in the ticket's order, that something delivers.
   * Email is the account's own address, so it is offered only when the account has one.
   */
  static List<MessageChannel> offered(RequiredActionContext context) {
    return ContactChange.offered(
        context.getSession(),
        PublicMessagingChannels.fromRealm(context.getRealm()),
        context.getUser());
  }

  @Override
  protected void handleEntry(
      RequiredActionContext context, AuthenticatorConfigModel config, String noteKey) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    MultivaluedMap<String, String> parameters = context.getHttpRequest().getDecodedFormParameters();
    Optional<MessageChannel> chosen =
        MessageChannel.parse(parameters.getFirst(CHANNEL_PARAM)).filter(offered(context)::contains);
    if (chosen.isEmpty()) {
      entryError(context, config, form -> form.setError("messaging.channelChoice.required"));
      return;
    }
    MessageChannel channel = chosen.get();
    String number = null;
    if (VerifiedContacts.NUMBER_CHANNELS.contains(channel)) {
      String entered = parameters.getFirst(CONTACT_PARAM);
      number = entered == null ? null : entered.trim();
      if (!VoterChannels.isE164(number)) {
        entryError(context, config, form -> form.setError("messaging.number.invalid"));
        return;
      }
      if (!isValidMobileNumber(number, config)) {
        entryError(
            context,
            config,
            form -> form.setError(ErrorType.INVALID_COUNTRY.toString(getI18nPrefix())));
        return;
      }
    }
    String consent =
        VerifiedContacts.consentValue(VerifiedContacts.consentVersion(context.getRealm()), channel);
    if (!consent.equals(parameters.getFirst(MessagingAttributes.FORM_MESSAGE_CONSENT))) {
      String label = context.form().getMessage(channel.labelKey());
      entryError(context, config, form -> form.setError("messaging.consent.required", label));
      return;
    }
    if (!isSendAllowed(authSession, config)) {
      entryError(
          context, config, form -> form.setError(ErrorType.RESEND_TIMER.toString(getI18nPrefix())));
      return;
    }

    clearEntry(authSession, noteKey);
    authSession.setAuthNote(MessagingAttributes.FORM_OTP_CHANNEL, channel.name());
    authSession.setAuthNote(
        MessagingAttributes.NOTE_OFFERED_CHANNELS,
        String.join(",", VoterChannels.names(offered(context))));
    switch (channel) {
      case WHATSAPP -> authSession.setAuthNote(MessagingAttributes.FORM_WHATSAPP_NUMBER, number);
      case VIBER -> authSession.setAuthNote(MessagingAttributes.FORM_VIBER_NUMBER, number);
      case SMS -> {
        String note = Utils.telUserAttribute(config);
        authSession.setAuthNote(note, number);
        authSession.setAuthNote(MessagingAttributes.NOTE_RESET_NUMBER_NOTE, note);
      }
      case EMAIL -> authSession.setAuthNote("email", context.getUser().getEmail());
      case MESSENGER -> {}
    }
    authSession.setAuthNote(MessagingAttributes.NOTE_VERIFIED_CONSENT, consent);
    authSession.setAuthNote(noteKey, channel.name());

    MessageAttemptState state;
    try {
      state =
          Utils.sendCode(
              config,
              context.getSession(),
              context.getUser(),
              authSession,
              getCourier(),
              /* deferredUser */ true,
              /* isOtl */ false,
              new String[0],
              context);
    } catch (Exception e) {
      log.warn("The code for the new contact could not be sent");
      state = MessageAttemptState.FAILED;
    }
    authSession.setAuthNote(MessagingAttributes.NOTE_DELIVERY_STATE, state.name());
    if (state == MessageAttemptState.FAILED) {
      clearEntry(authSession, noteKey);
      entryError(
          context, config, form -> form.setError(ErrorType.SEND_ERROR.toString(getI18nPrefix())));
      return;
    }
    context.challenge(
        createOTPForm(context, form -> form.setAttribute("resendTimerStarted", true), config));
  }

  private void entryError(
      RequiredActionContext context,
      AuthenticatorConfigModel config,
      Consumer<LoginFormsProvider> error) {
    context.challenge(createEntryForm(context, error, config));
  }

  @Override
  protected void clearEntry(AuthenticationSessionModel authSession, String noteKey) {
    String numberNote = authSession.getAuthNote(MessagingAttributes.NOTE_RESET_NUMBER_NOTE);
    if (numberNote != null && !MessagingAttributes.KEYCLOAK_NOTES.contains(numberNote)) {
      authSession.removeAuthNote(numberNote);
    }
    for (String note :
        List.of(
            noteKey,
            MessagingAttributes.FORM_OTP_CHANNEL,
            MessagingAttributes.FORM_WHATSAPP_NUMBER,
            MessagingAttributes.FORM_VIBER_NUMBER,
            MessagingAttributes.FORM_MESSAGE_CONSENT,
            MessagingAttributes.NOTE_OFFERED_CHANNELS,
            MessageOTPAuthenticator.MOBILE_NUMBER_FIELD,
            "email",
            MessagingAttributes.NOTE_VERIFIED_CONSENT,
            MessagingAttributes.NOTE_VERIFIED_CHANNEL,
            MessagingAttributes.NOTE_VERIFIED_DESTINATION,
            MessagingAttributes.NOTE_VERIFIED_MESSENGER_ID,
            MessagingAttributes.NOTE_VERIFIED_MESSENGER_PAGE,
            MessagingAttributes.NOTE_CODE_CHANNEL,
            MessagingAttributes.NOTE_CODE_DESTINATION,
            MessagingAttributes.NOTE_RESET_NUMBER_NOTE)) {
      authSession.removeAuthNote(note);
    }
    Utils.clearMessengerLink(authSession);
  }

  /**
   * The code must be the one sent to the pending channel. Messenger: harvest confirms the link of
   * this session and code, and names the account.
   */
  @Override
  protected Optional<String> confirmVerified(RequiredActionContext context, String value) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    if (!value.equals(authSession.getAuthNote(MessagingAttributes.NOTE_CODE_CHANNEL))) {
      return Optional.of(ErrorType.CODE_INVALID.toString(getI18nPrefix()));
    }
    if (!MessageChannel.MESSENGER.name().equals(value)) {
      return Optional.empty();
    }
    Optional<MessengerLinkStatus> confirmed =
        Utils.confirmedMessengerLink(context.getSession(), authSession);
    if (confirmed.isEmpty() || confirmed.get().pageId() == null) {
      Utils.clearMessengerLink(authSession);
      return Optional.of("messageOtp.messenger.notConfirmed");
    }
    authSession.setAuthNote(
        MessagingAttributes.NOTE_VERIFIED_MESSENGER_ID, confirmed.get().pageScopedId());
    authSession.setAuthNote(
        MessagingAttributes.NOTE_VERIFIED_MESSENGER_PAGE, confirmed.get().pageId());
    return Optional.empty();
  }

  /** Other voters who already use the verified contact. */
  @Override
  protected int usersWithSameValue(RequiredActionContext context, String value) {
    Optional<MessageChannel> channel = MessageChannel.parse(value);
    if (channel.isEmpty()) {
      return 0;
    }
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    String attribute =
        switch (channel.get()) {
          case WHATSAPP, VIBER, SMS -> MessagingAttributes.MESSAGE_NUMBER;
          case MESSENGER -> MessagingAttributes.MESSENGER_ID;
          case EMAIL -> null;
        };
    String contact =
        channel.get() == MessageChannel.MESSENGER
            ? authSession.getAuthNote(MessagingAttributes.NOTE_VERIFIED_MESSENGER_ID)
            : authSession.getAuthNote(MessagingAttributes.NOTE_CODE_DESTINATION);
    if (attribute == null || contact == null) {
      return 0;
    }
    String voterId = context.getUser().getId();
    return (int)
        context
            .getSession()
            .users()
            .searchForUserByUserAttributeStream(context.getRealm(), attribute, contact)
            .filter(other -> !other.getId().equals(voterId))
            .count();
  }

  @Override
  protected void createCredential(RequiredActionContext context) {
    MessageOTPCredentialProvider credentials =
        new MessageOTPCredentialProvider(context.getSession());
    if (!credentials.isConfiguredFor(
        context.getRealm(), context.getUser(), credentials.getType())) {
      super.createCredential(context);
    }
  }

  /**
   * Replaces the voter's channel with the one the code verified, then tells the old channel about
   * the change.
   */
  @Override
  protected void saveVerifiedValue(RequiredActionContext context, String value) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    RealmModel realm = context.getRealm();
    UserModel user = context.getUser();
    Optional<VoterChannel> previous = VoterChannel.of(user);
    MessageChannel.parse(value)
        .ifPresent(channel -> MessageOTPAuthenticator.recordVerified(authSession, channel));
    Optional<VoterChannel> verified = VerifiedContacts.verified(authSession, user);
    VerifiedContacts.persist(
        user,
        authSession,
        Utils.getRealmName(realm),
        VerifiedContacts.consentVersion(realm),
        Instant.now());
    clearEntry(authSession, getNoteKey(authSession));
    previous
        .filter(old -> verified.map(now -> !sameContact(old, now)).orElse(false))
        .ifPresent(old -> notifyOldChannel(context, old, verified.get().channel()));
  }

  private static boolean sameContact(VoterChannel a, VoterChannel b) {
    return a.channel() == b.channel()
        && a.destination().equals(b.destination())
        && a.messengerPage().equals(b.messengerPage());
  }

  /** Message key of the security notice sent to the old channel after a change. */
  public static final String CONTACT_CHANGED_KEY = "contactChanged.notice.text";

  /** Email subject of the same notice, when the old channel is email. */
  static final String CONTACT_CHANGED_SUBJECT_KEY = "contactChanged.notice.subject";

  static final String CONTACT_CHANGED_EMAIL_FTL = "contact-changed-email.ftl";

  /**
   * "Your contact for {organization} was changed to {channel}. If this wasn't you, contact
   * support." Sent to the old channel, which needs no code: it is a notice about that channel
   * itself, never a fallback. Like every notice it goes through harvest, which records it; email
   * and SMS harvest doesn't carry use Keycloak's providers. A failure is logged with its reason and
   * never undoes the change.
   */
  private void notifyOldChannel(
      RequiredActionContext context, VoterChannel old, MessageChannel newChannel) {
    KeycloakSession session = context.getSession();
    RealmModel realm = context.getRealm();
    UserModel user = context.getUser();
    String organization = Utils.getRealmName(realm);
    try {
      String channelLabel =
          Utils.formatLoginMessage(session, realm, user, newChannel.labelKey(), List.of());
      List<String> parameters = List.of(organization, channelLabel);
      Utils.NoticeOutcome outcome =
          Utils.sendNotice(
              session,
              realm,
              user,
              new NoticeRecipient(Optional.of(old), VoterChannels.elections(user), user.getId()),
              Utils.MessageCourier.CHOSEN,
              user.getEmail(),
              null,
              CONTACT_CHANGED_KEY,
              () -> parameters,
              context,
              // Keycloak already saved the new channel: harvest must use the old one.
              MessageRecipient.PREVIOUS_CHANNEL);
      if (outcome.keycloak().isPresent()) {
        Utils.KeycloakNotice notice = outcome.keycloak().get();
        SmsSenderProvider sms = session.getProvider(SmsSenderProvider.class);
        if (notice.courier() == Utils.MessageCourier.SMS && sms.sendsOnlyCodes()) {
          log.warnv(
              "{0}: the old channel {1} was not told about the change: {2}",
              getProviderId(), old.channel(), MessageReason.SMS_PROVIDER_SENDS_ONLY_CODES.reason());
        } else if (notice.courier() == Utils.MessageCourier.SMS) {
          sms.send(notice.mobileNumber(), CONTACT_CHANGED_KEY, parameters, realm, user, session);
          Utils.keycloakSent(context, MessageChannel.SMS, notice.mobileNumber());
        } else if (notice.courier() == Utils.MessageCourier.EMAIL) {
          Map<String, Object> body = new HashMap<>();
          body.put("organization", organization);
          body.put("channel", channelLabel);
          session
              .getProvider(EmailTemplateProvider.class)
              .setRealm(realm)
              .setUser(user)
              .send(
                  CONTACT_CHANGED_SUBJECT_KEY,
                  List.of(organization),
                  CONTACT_CHANGED_EMAIL_FTL,
                  body);
          Utils.keycloakSent(context, MessageChannel.EMAIL, notice.email());
        }
      } else if (!outcome.handedOver()) {
        log.warnv(
            "{0}: the old channel {1} was not told about the change: {2}",
            getProviderId(), old.channel(), outcome.failureReason());
      }
    } catch (Exception e) {
      log.warnv(
          "{0}: the old channel {1} was not told about the change: {2}",
          getProviderId(), old.channel(), e.getClass().getSimpleName());
    }
  }

  @Override
  protected Response createEntryForm(
      RequiredActionContext context,
      Consumer<LoginFormsProvider> formConsumer,
      AuthenticatorConfigModel config) {
    RealmModel realm = context.getRealm();
    List<MessageChannel> offered = offered(context);
    LoginFormsProvider form = context.form();
    form.setAttribute("messagingChannels", VoterChannels.names(offered));
    form.setAttribute("messagingOrganization", Utils.getRealmName(realm));
    form.setAttribute("messagingConsentVersion", VerifiedContacts.consentVersion(realm));
    if (offered.contains(MessageChannel.EMAIL)) {
      form.setAttribute(
          "messagingEmail", VoterChannels.mask(MessageChannel.EMAIL, context.getUser().getEmail()));
    }
    if (offered.contains(MessageChannel.MESSENGER)) {
      PublicMessagingChannels.fromRealm(realm)
          .flatMap(PublicMessagingChannels::messengerPage)
          .ifPresent(page -> form.setAttribute("messagingPage", page.displayName()));
    }
    return super.createEntryForm(context, formConsumer, config);
  }

  /** The code page names the channel and shows the destination masked, never in full. */
  @Override
  protected void decorateOtpForm(RequiredActionContext context, LoginFormsProvider form) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    Optional<MessageChannel> pending =
        MessageChannel.parse(authSession.getAuthNote(getNoteKey(authSession)));
    if (pending.isEmpty()) {
      form.setAttribute("contact", "");
      return;
    }
    MessageChannel channel = pending.get();
    Optional<PublicMessagingChannels> projection =
        PublicMessagingChannels.fromRealm(context.getRealm());
    String masked =
        VoterChannels.display(
            channel, authSession.getAuthNote(MessagingAttributes.NOTE_CODE_DESTINATION));
    form.setAttribute("contact", masked);
    form.setAttribute("address", masked);
    form.setAttribute("channel", channel.name());
    projection
        .flatMap(channels -> channels.senderLabel(channel))
        .ifPresent(label -> form.setAttribute("senderLabel", label));
    String deliveryState = authSession.getAuthNote(MessagingAttributes.NOTE_DELIVERY_STATE);
    if (deliveryState != null) {
      form.setAttribute("deliveryState", deliveryState);
    }
    if (channel == MessageChannel.MESSENGER) {
      MessageOTPAuthenticator.setMessengerAttributes(
          form, authSession, projection, context.getRealm());
    }
  }

  @Override
  public InitiatedActionSupport initiatedActionSupport() {
    return InitiatedActionSupport.SUPPORTED;
  }

  /**
   * A recent sign-in is required: Keycloak calls this for an application-initiated action ({@code
   * kc_action}) and signs the voter in again when the last sign-in is older. The realm entry's
   * {@code max_auth_age} may change it; without one (or with an invalid one) it is {@value
   * #MAX_AUTH_AGE_SECONDS} s.
   */
  @Override
  public int getMaxAuthAge(KeycloakSession session) {
    RealmModel realm =
        session == null || session.getContext() == null ? null : session.getContext().getRealm();
    RequiredActionConfigModel config =
        realm == null ? null : realm.getRequiredActionConfigByAlias(getId());
    if (config != null && config.containsConfigKey(MAX_AUTH_AGE_KEY)) {
      try {
        int configured = RequiredActionFactory.parseMaxAuthAge(config);
        if (configured >= 0) {
          return configured;
        }
      } catch (NumberFormatException e) {
        log.warnv("{0}: invalid max_auth_age; {1} s applies", getId(), MAX_AUTH_AGE_SECONDS);
      }
    }
    return MAX_AUTH_AGE_SECONDS;
  }

  /** The realm entry's config key Keycloak reads for the max auth age. */
  static final String MAX_AUTH_AGE_KEY = "max_auth_age";

  @Override
  public boolean isConfigurable() {
    return true;
  }

  /** {@code max_auth_age}, then this action's own code settings. */
  @Override
  public List<ProviderConfigProperty> getConfigMetadata() {
    List<ProviderConfigProperty> properties =
        new ArrayList<>(RequiredActionFactory.MAX_AUTH_AGE_CONFIG_PROPERTIES);
    List<String> codeSettings =
        List.of(
            Utils.CODE_TTL,
            Utils.CODE_LENGTH,
            Utils.RESEND_ACTIVATION_TIMER,
            Utils.MAX_CODE_ATTEMPTS,
            Utils.VALID_COUNTRY_CODES,
            Utils.MAX_RECEIVER_REUSE);
    for (ProviderConfigProperty property :
        new MessageOTPAuthenticatorFactory().getConfigProperties()) {
      if (codeSettings.contains(property.getName())) {
        properties.add(property);
      }
    }
    return properties;
  }

  @Override
  public void evaluateTriggers(RequiredActionContext context) {}

  @Override
  public String getDisplayText() {
    return "Change how we contact you";
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

  private static boolean blank(String value) {
    return value == null || value.isBlank();
  }
}
