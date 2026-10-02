// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import java.time.Instant;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import org.junit.jupiter.api.Test;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;
import sequent.keycloak.authenticator.messaging.MessageChannel;
import sequent.keycloak.authenticator.messaging.MessageSenderProvider;
import sequent.keycloak.authenticator.messaging.MessagingAttributes;
import sequent.keycloak.authenticator.messaging.PublicMessagingChannels;

class EnrollmentChannelsTest {
  private static final String MOBILE = "sequent.read-only.mobile-number";
  private static final String PROJECTION =
      """
      {"version": 1, "channels": [
        {"channel": "EMAIL", "purposes": ["OTP", "NOTICE"]},
        {"channel": "SMS", "purposes": ["OTP"]},
        {"channel": "WHATSAPP", "purposes": ["OTP", "NOTICE"]},
        {"channel": "MESSENGER", "purposes": ["OTP"]}],
       "election_channels": {"election-a": ["EMAIL", "WHATSAPP"]}}
      """;
  private static final List<MessageChannel> OFFERED =
      List.of(MessageChannel.EMAIL, MessageChannel.SMS, MessageChannel.WHATSAPP);

  private final MultivaluedMap<String, String> form = new MultivaluedHashMap<>();

  private List<String> errors() {
    return EnrollmentChannels.validate(OFFERED, form, MOBILE, "3").stream()
        .map(error -> error.getField() + ":" + error.getMessage())
        .collect(Collectors.toList());
  }

  private KeycloakSession session(Set<MessageChannel> delivered) {
    KeycloakSession session = mock(KeycloakSession.class);
    MessageSenderProvider sender = mock(MessageSenderProvider.class);
    when(sender.getChannels()).thenReturn(delivered);
    when(sender.delivers(org.mockito.ArgumentMatchers.any())).thenCallRealMethod();
    when(session.getProvider(MessageSenderProvider.class)).thenReturn(sender);
    return session;
  }

  @Test
  void thePostsChannelsAreOfferedOnlyWhenTheSenderDeliversThem() {
    RealmModel realm = mock(RealmModel.class);
    when(realm.getAttribute(PublicMessagingChannels.REALM_ATTRIBUTE)).thenReturn(PROJECTION);
    assertEquals(
        List.of(MessageChannel.EMAIL, MessageChannel.WHATSAPP),
        EnrollmentChannels.offered(
            session(Set.of(MessageChannel.WHATSAPP, MessageChannel.MESSENGER)),
            realm,
            Set.of("election-a")));
    assertEquals(
        List.of(MessageChannel.EMAIL, MessageChannel.SMS),
        EnrollmentChannels.offered(session(Set.of()), realm, Set.of("election-b")));
  }

  @Test
  void aChannelMustBeChosenAmongTheOfferedOnes() {
    assertEquals(
        List.of(MessagingAttributes.FORM_OTP_CHANNEL + ":messaging.channelChoice.required"),
        errors());
    form.putSingle(MessagingAttributes.FORM_OTP_CHANNEL, "MESSENGER");
    assertEquals(
        List.of(MessagingAttributes.FORM_OTP_CHANNEL + ":messaging.channelChoice.notAvailable"),
        errors());
  }

  @Test
  void messagingAppsNeedAnE164NumberAndConsent() {
    form.putSingle(MessagingAttributes.FORM_OTP_CHANNEL, "WHATSAPP");
    form.putSingle(MessagingAttributes.FORM_WHATSAPP_NUMBER, "0917 123 4567");
    assertEquals(
        List.of(
            MessagingAttributes.FORM_WHATSAPP_NUMBER + ":messaging.number.invalid",
            MessagingAttributes.FORM_MESSAGE_CONSENT + ":messaging.consent.required"),
        errors());

    form.putSingle(MessagingAttributes.FORM_WHATSAPP_NUMBER, "+639171234567");
    form.putSingle(MessagingAttributes.FORM_MESSAGE_CONSENT, "3:WHATSAPP");
    assertEquals(List.of(), errors());
  }

  @Test
  void consentToAnOlderWordingIsNotConsent() {
    form.putSingle(MessagingAttributes.FORM_OTP_CHANNEL, "WHATSAPP");
    form.putSingle(MessagingAttributes.FORM_WHATSAPP_NUMBER, "+639171234567");
    form.putSingle(MessagingAttributes.FORM_MESSAGE_CONSENT, "2:WHATSAPP");
    assertEquals(
        List.of(MessagingAttributes.FORM_MESSAGE_CONSENT + ":messaging.consent.required"),
        errors());
  }

  @Test
  void consentToAnotherChannelIsNotConsent() {
    form.putSingle(MessagingAttributes.FORM_OTP_CHANNEL, "WHATSAPP");
    form.putSingle(MessagingAttributes.FORM_WHATSAPP_NUMBER, "+639171234567");
    form.putSingle(MessagingAttributes.FORM_MESSAGE_CONSENT, "3:VIBER");
    assertEquals(
        List.of(MessagingAttributes.FORM_MESSAGE_CONSENT + ":messaging.consent.required"),
        errors());
    assertEquals("3:WHATSAPP", EnrollmentChannels.consentValue("3", MessageChannel.WHATSAPP));
  }

  @Test
  void smsUsesTheMobileNumberAndEmailTheEmail() {
    form.putSingle(MessagingAttributes.FORM_OTP_CHANNEL, "SMS");
    form.putSingle(MOBILE, "12345");
    assertEquals(List.of(MOBILE + ":messaging.number.invalid"), errors());
    form.putSingle(MOBILE, "+15550123456");
    assertEquals(List.of(), errors());

    form.putSingle(MessagingAttributes.FORM_OTP_CHANNEL, "EMAIL");
    assertEquals(List.of("email:missingEmailMessage"), errors());
    form.putSingle("email", "voter@example.test");
    assertEquals(List.of(), errors());
  }

  @Test
  void theVerifiedContactIsSavedWithTheExplicitNoticeChoiceAndConsent() {
    UserModel user = mock(UserModel.class);
    Map<String, String> notes = new HashMap<>();
    AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
    when(authSession.getAuthNote(anyString())).thenAnswer(i -> notes.get(i.getArgument(0)));
    when(user.getAttributeStream(MessagingAttributes.VERIFIED_CHANNELS))
        .thenAnswer(i -> Stream.of("EMAIL"));
    notes.put(MessagingAttributes.NOTE_VERIFIED_CHANNEL, "WHATSAPP");
    notes.put(MessagingAttributes.FORM_WHATSAPP_NUMBER, "+639171234567");
    notes.put(MessagingAttributes.FORM_VIBER_NUMBER, "+639170000000");
    notes.put(MessagingAttributes.FORM_NOTICE_CHANNEL, "WHATSAPP");
    notes.put(MessagingAttributes.FORM_MESSAGE_CONSENT, "3:WHATSAPP");

    EnrollmentChannels.persist(
        user, authSession, MOBILE, "3", Instant.parse("2026-10-02T10:00:00Z"));

    verify(user).setSingleAttribute(MessagingAttributes.WHATSAPP_NUMBER, "+639171234567");
    verify(user, never()).setSingleAttribute(eq(MessagingAttributes.VIBER_NUMBER), anyString());
    verify(user).setAttribute(MessagingAttributes.VERIFIED_CHANNELS, List.of("EMAIL", "WHATSAPP"));
    verify(user).setSingleAttribute(MessagingAttributes.MESSAGE_CHANNEL, "WHATSAPP");
    verify(user)
        .setSingleAttribute(
            MessagingAttributes.MESSAGE_CONSENT,
            "{\"time\":\"2026-10-02T10:00:00Z\",\"wording_version\":\"3\",\"channel\":\"WHATSAPP\"}");
  }

  @Test
  void anOtpChoiceDoesNotChangeTheNoticeChannel() {
    UserModel user = mock(UserModel.class);
    Map<String, String> notes = new HashMap<>();
    AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
    when(authSession.getAuthNote(anyString())).thenAnswer(i -> notes.get(i.getArgument(0)));
    when(user.getAttributeStream(MessagingAttributes.VERIFIED_CHANNELS))
        .thenAnswer(i -> Stream.of());
    notes.put(MessagingAttributes.NOTE_VERIFIED_CHANNEL, "MESSENGER");
    notes.put(MessagingAttributes.NOTE_VERIFIED_MESSENGER_ID, "psid-1");
    notes.put(MessagingAttributes.NOTE_VERIFIED_MESSENGER_PAGE, "page-1");
    notes.put(MessagingAttributes.FORM_NOTICE_CHANNEL, "EMAIL");

    EnrollmentChannels.persist(user, authSession, MOBILE, "3", Instant.now());

    verify(user).setSingleAttribute(MessagingAttributes.MESSENGER_ID, "psid-1");
    verify(user).setSingleAttribute(MessagingAttributes.MESSENGER_PAGE, "page-1");
    verify(user).setAttribute(MessagingAttributes.VERIFIED_CHANNELS, List.of("MESSENGER"));
    verify(user, never()).setSingleAttribute(eq(MessagingAttributes.MESSAGE_CHANNEL), anyString());
    verify(user, never()).setSingleAttribute(eq(MessagingAttributes.MESSAGE_CONSENT), anyString());
  }

  @Test
  void withoutAVerifiedChannelNothingIsSaved() {
    UserModel user = mock(UserModel.class);
    AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
    EnrollmentChannels.persist(user, authSession, MOBILE, "3", Instant.now());
    verify(user, never()).setAttribute(anyString(), org.mockito.ArgumentMatchers.anyList());
    verify(user, never()).setSingleAttribute(anyString(), anyString());
  }

  @Test
  void aSubmittedFormCannotSetKeycloaksOwnNotes() {
    List<String> stored = new ArrayList<>();
    for (String key :
        List.of(
            "code",
            "ttl",
            "code-attempts",
            "Email verified",
            MessagingAttributes.NOTE_VERIFIED_CHANNEL,
            MessagingAttributes.NOTE_VERIFIED_MESSENGER_ID,
            MessagingAttributes.NOTE_OFFERED_CHANNELS,
            "firstName",
            MessagingAttributes.FORM_OTP_CHANNEL)) {
      if (Utils.isFormNote(key)) {
        stored.add(key);
      }
    }
    assertEquals(List.of("firstName", MessagingAttributes.FORM_OTP_CHANNEL), stored);
    assertTrue(Utils.isFormNote("email"));
  }
}
