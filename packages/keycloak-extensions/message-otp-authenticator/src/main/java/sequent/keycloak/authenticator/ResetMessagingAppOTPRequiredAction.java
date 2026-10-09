// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.MultivaluedMap;
import jakarta.ws.rs.core.Response;
import java.time.Instant;
import java.util.List;
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
 * Lets an authenticated voter add or replace a WhatsApp number, a Viber number or a Messenger
 * connection, among the messaging apps the realm uses for codes.
 *
 * <ol>
 *   <li>The voter picks the app, enters the number (WhatsApp, Viber) and agrees to its messages.
 *   <li>A code goes to that contact through the message sender; Messenger uses a one-time link.
 *   <li>A correct code saves the contact the way enrollment does.
 * </ol>
 *
 * A required action only runs for a voter who already signed in, so holding a new number is never
 * enough to attach it to an account. Codes follow the resend timer and the attempt limit of the
 * message OTP authenticator.
 */
@AutoService(RequiredActionFactory.class)
@JBossLog
public class ResetMessagingAppOTPRequiredAction extends BaseResetMessageOTPRequiredAction
    implements RequiredActionFactory {
  public static final String PROVIDER_ID = "messaging-app-otp-ra";
  static final String CONTACT_PARAM = "contact";
  static final String ENTRY_FTL = "message-otp.enter-app-contact.ftl";

  @Override
  protected String getProviderId() {
    return PROVIDER_ID;
  }

  /** The app whose code is pending; no note means the voter is still choosing. */
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
      log.info("No messaging app is used for codes in this realm");
      context.getAuthenticationSession().removeRequiredAction(PROVIDER_ID);
      context.ignore();
      return false;
    }
    return true;
  }

  /** The messaging apps the voter's Post offers codes on that the sender delivers. */
  static List<MessageChannel> offered(RequiredActionContext context) {
    MessageSenderProvider sender = VoterChannels.sender(context.getSession());
    return VoterChannels.offeredForOtp(
            PublicMessagingChannels.fromRealm(context.getRealm()),
            VoterChannels.elections(context.getUser()))
        .stream()
        .filter(VerifiedContacts.MESSAGING_APPS::contains)
        .filter(sender::delivers)
        .toList();
  }

  @Override
  protected void handleEntry(
      RequiredActionContext context, AuthenticatorConfigModel config, String noteKey) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    MultivaluedMap<String, String> parameters = context.getHttpRequest().getDecodedFormParameters();
    Optional<MessageChannel> chosen =
        MessageChannel.parse(parameters.getFirst(MessageOTPAuthenticator.CHANNEL_PARAM))
            .filter(offered(context)::contains);
    if (chosen.isEmpty()) {
      entryError(context, config, form -> form.setError("messaging.channelChoice.required"));
      return;
    }
    MessageChannel channel = chosen.get();
    String number = null;
    if (channel != MessageChannel.MESSENGER) {
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
    if (channel == MessageChannel.WHATSAPP) {
      authSession.setAuthNote(MessagingAttributes.FORM_WHATSAPP_NUMBER, number);
    } else if (channel == MessageChannel.VIBER) {
      authSession.setAuthNote(MessagingAttributes.FORM_VIBER_NUMBER, number);
    }
    authSession.setAuthNote(MessagingAttributes.FORM_MESSAGE_CONSENT, consent);
    if (channel.name().equals(parameters.getFirst(MessagingAttributes.FORM_NOTICE_CHANNEL))) {
      authSession.setAuthNote(MessagingAttributes.FORM_NOTICE_CHANNEL, channel.name());
    }
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
    if (state == MessageAttemptState.FAILED) {
      clearEntry(authSession, noteKey);
      entryError(
          context, config, form -> form.setError(ErrorType.SEND_ERROR.toString(getI18nPrefix())));
      return;
    }
    context.challenge(createOTPForm(context, null, config));
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
            MessagingAttributes.FORM_NOTICE_CHANNEL,
            MessagingAttributes.NOTE_VERIFIED_CHANNEL,
            MessagingAttributes.NOTE_VERIFIED_MESSENGER_ID,
            MessagingAttributes.NOTE_VERIFIED_MESSENGER_PAGE)) {
      authSession.removeAuthNote(note);
    }
    Utils.clearMessengerLink(authSession);
  }

  /** Messenger: harvest confirms the link of this session and code, and names the account. */
  @Override
  protected Optional<String> confirmVerified(RequiredActionContext context, String value) {
    if (!MessageChannel.MESSENGER.name().equals(value)) {
      return Optional.empty();
    }
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    Optional<MessengerLinkStatus> confirmed =
        Utils.confirmedMessengerLink(context.getSession(), authSession);
    if (confirmed.isEmpty()) {
      Utils.clearMessengerLink(authSession);
      return Optional.of("messageOtp.messenger.notConfirmed");
    }
    authSession.setAuthNote(
        MessagingAttributes.NOTE_VERIFIED_MESSENGER_ID, confirmed.get().pageScopedId());
    if (confirmed.get().pageId() != null) {
      authSession.setAuthNote(
          MessagingAttributes.NOTE_VERIFIED_MESSENGER_PAGE, confirmed.get().pageId());
    }
    return Optional.empty();
  }

  /** Other voters who already use the verified contact on the same app. */
  @Override
  protected int usersWithSameValue(RequiredActionContext context, String value) {
    Optional<MessageChannel> channel = MessageChannel.parse(value);
    if (channel.isEmpty()) {
      return 0;
    }
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    String attribute = userAttribute(channel.get());
    String contact = pendingContact(authSession, channel.get());
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

  private static String userAttribute(MessageChannel channel) {
    return switch (channel) {
      case WHATSAPP -> MessagingAttributes.WHATSAPP_NUMBER;
      case VIBER -> MessagingAttributes.VIBER_NUMBER;
      case MESSENGER -> MessagingAttributes.MESSENGER_ID;
      case EMAIL, SMS -> null;
    };
  }

  private static String pendingContact(
      AuthenticationSessionModel authSession, MessageChannel channel) {
    return switch (channel) {
      case WHATSAPP -> authSession.getAuthNote(MessagingAttributes.FORM_WHATSAPP_NUMBER);
      case VIBER -> authSession.getAuthNote(MessagingAttributes.FORM_VIBER_NUMBER);
      case MESSENGER -> authSession.getAuthNote(MessagingAttributes.NOTE_VERIFIED_MESSENGER_ID);
      case EMAIL, SMS -> null;
    };
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

  @Override
  protected void saveVerifiedValue(RequiredActionContext context, String value) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    RealmModel realm = context.getRealm();
    authSession.setAuthNote(MessagingAttributes.NOTE_VERIFIED_CHANNEL, value);
    VerifiedContacts.persist(
        context.getUser(),
        authSession,
        Utils.telUserAttribute(Utils.getConfig(realm).orElse(null)),
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
    if (offered.contains(MessageChannel.MESSENGER)) {
      PublicMessagingChannels.fromRealm(realm)
          .flatMap(PublicMessagingChannels::messengerPage)
          .ifPresent(page -> form.setAttribute("messagingPage", page.displayName()));
    }
    return super.createEntryForm(context, formConsumer, config);
  }

  @Override
  protected void decorateOtpForm(RequiredActionContext context, LoginFormsProvider form) {
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    Optional<MessageChannel> pending =
        MessageChannel.parse(authSession.getAuthNote(getNoteKey(authSession)));
    if (pending.isEmpty()) {
      return;
    }
    MessageChannel channel = pending.get();
    Optional<PublicMessagingChannels> projection =
        PublicMessagingChannels.fromRealm(context.getRealm());
    String contact =
        channel == MessageChannel.MESSENGER ? null : pendingContact(authSession, channel);
    form.setAttribute("contact", contact == null ? "" : contact);
    form.setAttribute("channel", channel.name());
    projection
        .flatMap(channels -> channels.senderLabel(channel))
        .ifPresent(label -> form.setAttribute("senderLabel", label));
    setNote(form, "deliveryState", authSession, MessagingAttributes.NOTE_DELIVERY_STATE);
    if (channel == MessageChannel.MESSENGER) {
      projection
          .flatMap(PublicMessagingChannels::messengerPage)
          .ifPresent(page -> form.setAttribute("messengerPage", page.displayName()));
      setNote(form, "messengerLink", authSession, MessagingAttributes.NOTE_MESSENGER_LINK);
      setNote(form, "messengerWord", authSession, MessagingAttributes.NOTE_MESSENGER_WORD);
      setNote(form, "messengerState", authSession, MessagingAttributes.NOTE_MESSENGER_STATE);
    }
  }

  private static void setNote(
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
  public InitiatedActionSupport initiatedActionSupport() {
    return InitiatedActionSupport.SUPPORTED;
  }

  @Override
  public void evaluateTriggers(RequiredActionContext context) {}

  @Override
  public String getDisplayText() {
    return "Add or replace a messaging app for codes";
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
