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
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.RequiredActionConfigModel;
import org.keycloak.models.UserModel;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.sessions.AuthenticationSessionModel;
import sequent.keycloak.authenticator.credential.MessageOTPCredentialProvider;
import sequent.keycloak.authenticator.messaging.MessageAttemptState;
import sequent.keycloak.authenticator.messaging.MessageChannel;
import sequent.keycloak.authenticator.messaging.MessageSenderProvider;
import sequent.keycloak.authenticator.messaging.MessagingAttributes;
import sequent.keycloak.authenticator.messaging.MessengerLinkStatus;
import sequent.keycloak.authenticator.messaging.PublicMessagingChannels;
import sequent.keycloak.authenticator.messaging.VerifiedContacts;
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
      super.requiredActionChallenge(context);
    }
  }

  @Override
  public void processAction(RequiredActionContext context) {
    if (canRun(context)) {
      super.processAction(context);
    }
  }

  private boolean canRun(RequiredActionContext context) {
    if (context.getUser() == null) {
      context.failure();
      return false;
    }
    if (offered(context).isEmpty()) {
      log.info("No contact channel is offered for codes to this voter; nothing to change");
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
    MessageSenderProvider sender = VoterChannels.sender(context.getSession());
    UserModel user = context.getUser();
    Optional<PublicMessagingChannels> projection =
        PublicMessagingChannels.fromRealm(context.getRealm());
    if (projection.isEmpty()) {
      return List.of();
    }
    List<MessageChannel> offered = new ArrayList<>();
    for (MessageChannel channel :
        VoterChannels.offeredForOtp(projection, VoterChannels.elections(user))) {
      if (!VoterChannels.deliverable(sender, channel)) {
        continue;
      }
      if (channel == MessageChannel.EMAIL && blank(user.getEmail())) {
        continue;
      }
      offered.add(channel);
    }
    return offered;
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
    switch (channel) {
      case WHATSAPP -> authSession.setAuthNote(MessagingAttributes.FORM_WHATSAPP_NUMBER, number);
      case VIBER -> authSession.setAuthNote(MessagingAttributes.FORM_VIBER_NUMBER, number);
      case SMS -> authSession.setAuthNote(Utils.telUserAttribute(config), number);
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
    for (String note :
        List.of(
            noteKey,
            MessagingAttributes.FORM_OTP_CHANNEL,
            MessagingAttributes.FORM_WHATSAPP_NUMBER,
            MessagingAttributes.FORM_VIBER_NUMBER,
            MessagingAttributes.FORM_MESSAGE_CONSENT,
            MessageOTPAuthenticator.MOBILE_NUMBER_FIELD,
            "email",
            MessagingAttributes.NOTE_VERIFIED_CONSENT,
            MessagingAttributes.NOTE_VERIFIED_CHANNEL,
            MessagingAttributes.NOTE_VERIFIED_DESTINATION,
            MessagingAttributes.NOTE_VERIFIED_MESSENGER_ID,
            MessagingAttributes.NOTE_VERIFIED_MESSENGER_PAGE,
            MessagingAttributes.NOTE_CODE_CHANNEL,
            MessagingAttributes.NOTE_CODE_DESTINATION)) {
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

  /** Replaces the voter's channel with the one the code verified. */
  @Override
  protected void saveVerifiedValue(RequiredActionContext context, String value) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    RealmModel realm = context.getRealm();
    MessageChannel.parse(value)
        .ifPresent(channel -> MessageOTPAuthenticator.recordVerified(authSession, channel));
    VerifiedContacts.persist(
        context.getUser(),
        authSession,
        Utils.getRealmName(realm),
        VerifiedContacts.consentVersion(realm),
        Instant.now());
    clearEntry(authSession, getNoteKey(authSession));
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
        VoterChannels.mask(
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

  /** A recent sign-in is required; the realm entry's {@code max_auth_age} may change it. */
  @Override
  public int getMaxAuthAge() {
    return MAX_AUTH_AGE_SECONDS;
  }

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
