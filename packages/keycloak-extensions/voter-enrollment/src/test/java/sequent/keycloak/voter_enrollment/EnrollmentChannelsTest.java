// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.anyString;
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
      List.of(MessageChannel.WHATSAPP, MessageChannel.SMS, MessageChannel.EMAIL);

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
        List.of(MessageChannel.WHATSAPP, MessageChannel.EMAIL),
        EnrollmentChannels.offered(
            session(Set.of(MessageChannel.WHATSAPP, MessageChannel.MESSENGER)),
            realm,
            Set.of("election-a")));
    assertEquals(
        List.of(MessageChannel.EMAIL),
        EnrollmentChannels.offered(session(Set.of()), realm, Set.of("election-a")));
  }

  @Test
  void aPostWithoutRestrictionIsOfferedEveryDeliverableChannelOnceItIsLabelled() {
    RealmModel realm = mock(RealmModel.class);
    when(realm.getAttribute(PublicMessagingChannels.REALM_ATTRIBUTE))
        .thenReturn(
            PROJECTION.replace(
                "\"election_channels\"",
                "\"election_labels\": {\"election-b\": [\"Open Post\"]}, \"election_channels\""));
    assertEquals(
        List.of(MessageChannel.SMS, MessageChannel.EMAIL),
        EnrollmentChannels.offered(session(Set.of()), realm, Set.of("open post")));
    assertEquals(
        List.of(MessageChannel.EMAIL),
        EnrollmentChannels.offered(session(Set.of()), realm, Set.of("election-unknown")));
  }

  @Test
  void thePostFieldMatchesItsElectionByLabel() {
    RealmModel realm = mock(RealmModel.class);
    when(realm.getAttribute(PublicMessagingChannels.REALM_ATTRIBUTE))
        .thenReturn(
            """
            {"version": 1, "channels": [
              {"channel": "EMAIL", "purposes": ["OTP"]},
              {"channel": "SMS", "purposes": ["OTP"]},
              {"channel": "WHATSAPP", "purposes": ["OTP"]}],
             "election_channels": {"election-a": ["EMAIL", "WHATSAPP"], "election-b": ["EMAIL"]},
             "election_labels": {"election-a": ["Synthetic Embassy"], "election-b": ["Other Post"]}}
            """);
    KeycloakSession session = session(Set.of(MessageChannel.WHATSAPP));
    assertEquals(
        List.of(MessageChannel.WHATSAPP, MessageChannel.EMAIL),
        EnrollmentChannels.offered(session, realm, Set.of(" synthetic EMBASSY ")));
    assertEquals(
        List.of(MessageChannel.EMAIL),
        EnrollmentChannels.offered(session, realm, Set.of("Unlisted Embassy")));
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
    form.putSingle(MessagingAttributes.FORM_MESSAGE_CONSENT, "3:SMS");
    assertEquals(List.of(MOBILE + ":messaging.number.invalid"), errors());
    form.putSingle(MOBILE, "+15550123456");
    assertEquals(List.of(), errors());

    form.putSingle(MessagingAttributes.FORM_OTP_CHANNEL, "EMAIL");
    form.putSingle(MessagingAttributes.FORM_MESSAGE_CONSENT, "3:EMAIL");
    assertEquals(List.of("email:missingEmailMessage"), errors());
    form.putSingle("email", "voter@example.test");
    assertEquals(List.of(), errors());
  }

  /** KC-R5, plan §2.5: consent names the channel and is needed for every channel. */
  @Test
  void emailAndSmsNeedConsentToo() {
    form.putSingle(MessagingAttributes.FORM_OTP_CHANNEL, "EMAIL");
    form.putSingle("email", "voter@example.test");
    assertEquals(
        List.of(MessagingAttributes.FORM_MESSAGE_CONSENT + ":messaging.consent.required"),
        errors());
    form.putSingle(MessagingAttributes.FORM_OTP_CHANNEL, "SMS");
    form.putSingle(MOBILE, "+15550123456");
    form.putSingle(MessagingAttributes.FORM_MESSAGE_CONSENT, "3:EMAIL");
    assertEquals(
        List.of(MessagingAttributes.FORM_MESSAGE_CONSENT + ":messaging.consent.required"),
        errors());
  }

  private final Map<String, String> notes = new HashMap<>();

  private AuthenticationSessionModel notes() {
    AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
    when(authSession.getAuthNote(anyString())).thenAnswer(i -> notes.get(i.getArgument(0)));
    org.mockito.Mockito.doAnswer(i -> notes.put(i.getArgument(0), i.getArgument(1)))
        .when(authSession)
        .setAuthNote(anyString(), anyString());
    org.mockito.Mockito.doAnswer(i -> notes.remove(i.getArgument(0)))
        .when(authSession)
        .removeAuthNote(anyString());
    return authSession;
  }

  @Test
  void theVerifiedContactReplacesTheVotersChannelWithConsent() {
    UserModel user = mock(UserModel.class);
    AuthenticationSessionModel authSession = notes();
    notes.put(MessagingAttributes.NOTE_VERIFIED_CHANNEL, "WHATSAPP");
    notes.put(MessagingAttributes.NOTE_VERIFIED_DESTINATION, "+639171234567");
    notes.put(MessagingAttributes.NOTE_VERIFIED_CONSENT, "3:WHATSAPP");
    // Form fields never decide what is saved.
    notes.put(MessagingAttributes.FORM_WHATSAPP_NUMBER, "+639170000000");
    notes.put(MessagingAttributes.FORM_MESSAGE_CONSENT, "3:VIBER");

    EnrollmentChannels.persist(
        user, authSession, "Synthetic Commission", "3", Instant.parse("2026-10-02T10:00:00Z"));

    verify(user).setSingleAttribute(MessagingAttributes.MESSAGE_CHANNEL, "WHATSAPP");
    verify(user).setSingleAttribute(MessagingAttributes.MESSAGE_NUMBER, "+639171234567");
    verify(user).removeAttribute(MessagingAttributes.MESSENGER_ID);
    verify(user).removeAttribute(MessagingAttributes.MESSENGER_PAGE);
    verify(user).removeAttribute(MessagingAttributes.VERIFIED_CHANNELS);
    verify(user).removeAttribute(MessagingAttributes.WHATSAPP_NUMBER);
    verify(user).removeAttribute(MessagingAttributes.VIBER_NUMBER);
    verify(user)
        .setSingleAttribute(
            MessagingAttributes.MESSAGE_CONSENT,
            "{\"at\":\"2026-10-02T10:00:00Z\",\"version\":\"3\","
                + "\"organization\":\"Synthetic Commission\",\"channel\":\"WHATSAPP\"}");
    verify(user, never()).setAttribute(anyString(), org.mockito.ArgumentMatchers.anyList());
  }

  @Test
  void messengerSavesItsAccountAndPageAndDropsTheNumber() {
    UserModel user = mock(UserModel.class);
    AuthenticationSessionModel authSession = notes();
    notes.put(MessagingAttributes.NOTE_VERIFIED_CHANNEL, "MESSENGER");
    notes.put(MessagingAttributes.NOTE_VERIFIED_MESSENGER_ID, "psid-1");
    notes.put(MessagingAttributes.NOTE_VERIFIED_MESSENGER_PAGE, "page-1");

    EnrollmentChannels.persist(user, authSession, "Synthetic Commission", "3", Instant.now());

    verify(user).setSingleAttribute(MessagingAttributes.MESSAGE_CHANNEL, "MESSENGER");
    verify(user).setSingleAttribute(MessagingAttributes.MESSENGER_ID, "psid-1");
    verify(user).setSingleAttribute(MessagingAttributes.MESSENGER_PAGE, "page-1");
    verify(user).removeAttribute(MessagingAttributes.MESSAGE_NUMBER);
    verify(user).removeAttribute(MessagingAttributes.MESSAGE_CONSENT);
  }

  @Test
  void withoutAVerifiedChannelNothingIsSaved() {
    UserModel user = mock(UserModel.class);
    AuthenticationSessionModel authSession = notes();
    EnrollmentChannels.persist(user, authSession, "Synthetic Commission", "3", Instant.now());
    notes.put(MessagingAttributes.NOTE_VERIFIED_CHANNEL, "VIBER");
    EnrollmentChannels.persist(user, authSession, "Synthetic Commission", "3", Instant.now());
    verify(user, never()).setSingleAttribute(anyString(), anyString());
    verify(user, never()).removeAttribute(anyString());
  }

  @Test
  void theValidatedConsentIsKeptInAKeycloakNote() {
    AuthenticationSessionModel authSession = notes();
    form.putSingle(MessagingAttributes.FORM_OTP_CHANNEL, "SMS");
    form.putSingle(MessagingAttributes.FORM_MESSAGE_CONSENT, "3:SMS");
    EnrollmentChannels.recordConsent(authSession, form, "3");
    assertEquals("3:SMS", notes.get(MessagingAttributes.NOTE_VERIFIED_CONSENT));

    form.putSingle(MessagingAttributes.FORM_MESSAGE_CONSENT, "3:EMAIL");
    EnrollmentChannels.recordConsent(authSession, form, "3");
    assertEquals(null, notes.get(MessagingAttributes.NOTE_VERIFIED_CONSENT));
  }

  /** KC-F5/K16: a reviewer's decision carries the verified channel from Keycloak's notes. */
  @Test
  void applicantDataCarriesTheVerifiedChannelNeverFormFields() {
    AuthenticationSessionModel authSession = notes();
    notes.put(MessagingAttributes.NOTE_VERIFIED_CHANNEL, "VIBER");
    notes.put(MessagingAttributes.NOTE_VERIFIED_DESTINATION, "+639171234567");
    notes.put(MessagingAttributes.NOTE_VERIFIED_CONSENT, "3:VIBER");
    notes.put(MessagingAttributes.FORM_VIBER_NUMBER, "+639170000000");
    notes.put(MessagingAttributes.MESSAGE_CHANNEL, "EMAIL");

    Map<String, String> data =
        Utils.verifiedChannelData(
            authSession, "Synthetic Commission", "3", Instant.parse("2026-10-02T10:00:00Z"));
    assertEquals(
        Map.of(
            MessagingAttributes.MESSAGE_CHANNEL, "VIBER",
            MessagingAttributes.MESSAGE_NUMBER, "+639171234567",
            MessagingAttributes.MESSAGE_CONSENT,
                "{\"at\":\"2026-10-02T10:00:00Z\",\"version\":\"3\","
                    + "\"organization\":\"Synthetic Commission\",\"channel\":\"VIBER\"}"),
        data);

    notes.clear();
    assertEquals(Map.of(), Utils.verifiedChannelData(authSession, "x", "3", Instant.now()));
  }

  @Test
  void aSubmittedFormCanOnlySetAllowedNotes() {
    Set<String> profile =
        Set.of("firstName", "embassy", MOBILE, MessagingAttributes.MESSAGE_CHANNEL);
    List<String> stored = new ArrayList<>();
    for (String key :
        List.of(
            "code",
            "ttl",
            "code-attempts",
            "Email verified",
            MessagingAttributes.NOTE_VERIFIED_CHANNEL,
            MessagingAttributes.NOTE_VERIFIED_DESTINATION,
            MessagingAttributes.NOTE_VERIFIED_CONSENT,
            MessagingAttributes.NOTE_VERIFIED_MESSENGER_ID,
            MessagingAttributes.NOTE_OFFERED_CHANNELS,
            MessagingAttributes.NOTE_CODE_DESTINATION,
            MessagingAttributes.MESSAGE_CHANNEL,
            MessagingAttributes.MESSAGE_NUMBER,
            "verificationStatus",
            "planted-field",
            "firstName",
            "embassy",
            MOBILE,
            MessagingAttributes.FORM_OTP_CHANNEL,
            MessagingAttributes.FORM_MESSAGE_CONSENT)) {
      if (Utils.isFormNote(key, profile)) {
        stored.add(key);
      }
    }
    assertEquals(
        List.of(
            "firstName",
            "embassy",
            MOBILE,
            MessagingAttributes.FORM_OTP_CHANNEL,
            MessagingAttributes.FORM_MESSAGE_CONSENT),
        stored);
    assertTrue(Utils.isFormNote("email"));
    assertTrue(Utils.isFormNote("password"));
  }

  /**
   * R7-09: a form that doesn't ask for a channel (or a LOGIN form) can't set one: the channel, its
   * numbers and the consent never become notes, and stale ones are removed.
   */
  @Test
  void aFormThatDoesNotAskForAChannelCannotSetOne() {
    Set<String> profile = Set.of("firstName", MOBILE);
    for (String key : Utils.CHANNEL_FIELDS) {
      assertFalse(Utils.isFormNote(key, profile, EnrollmentChannels.ChannelChoicePolicy.NONE), key);
      assertTrue(
          Utils.isFormNote(key, profile, EnrollmentChannels.ChannelChoicePolicy.OTP_CHANNELS), key);
    }
    assertEquals(
        Set.of(
            MessagingAttributes.FORM_OTP_CHANNEL,
            MessagingAttributes.FORM_WHATSAPP_NUMBER,
            MessagingAttributes.FORM_VIBER_NUMBER,
            MessagingAttributes.FORM_MESSAGE_CONSENT),
        Utils.CHANNEL_FIELDS);
    assertTrue(Utils.isFormNote("firstName", profile, EnrollmentChannels.ChannelChoicePolicy.NONE));
    assertTrue(Utils.isFormNote(MOBILE, profile, EnrollmentChannels.ChannelChoicePolicy.NONE));

    Map<String, String> configMap = new java.util.HashMap<>();
    assertEquals(
        EnrollmentChannels.ChannelChoicePolicy.NONE,
        DeferredRegistrationUserCreation.formChannelChoice(configMap));
    configMap.put(
        DeferredRegistrationUserCreation.CHANNEL_CHOICE_POLICY,
        EnrollmentChannels.ChannelChoicePolicy.OTP_CHANNELS.name());
    assertEquals(
        EnrollmentChannels.ChannelChoicePolicy.OTP_CHANNELS,
        DeferredRegistrationUserCreation.formChannelChoice(configMap));
    configMap.put(DeferredRegistrationUserCreation.FORM_MODE, "LOGIN");
    assertEquals(
        EnrollmentChannels.ChannelChoicePolicy.NONE,
        DeferredRegistrationUserCreation.formChannelChoice(configMap));
  }

  /** KC-F9/KC-R8: each Post's channels, keyed by the Post field's options, in ticket order. */
  @Test
  void eachPostsChannelsAreMappedByThePostOptions() {
    RealmModel realm = mock(RealmModel.class);
    when(realm.getAttribute(PublicMessagingChannels.REALM_ATTRIBUTE))
        .thenReturn(
            """
            {"version": 1, "channels": [
              {"channel": "EMAIL", "purposes": ["OTP"]},
              {"channel": "VIBER", "purposes": ["OTP"]},
              {"channel": "WHATSAPP", "purposes": ["OTP"]}],
             "election_channels": {"election-a": ["EMAIL", "WHATSAPP"], "election-b": ["EMAIL", "VIBER"]},
             "election_labels": {"election-a": ["Madrid"], "election-b": ["Riyadh"]}}
            """);
    KeycloakSession session = session(Set.of(MessageChannel.WHATSAPP, MessageChannel.VIBER));
    org.keycloak.userprofile.UserProfileProvider profiles =
        mock(org.keycloak.userprofile.UserProfileProvider.class);
    when(session.getProvider(org.keycloak.userprofile.UserProfileProvider.class))
        .thenReturn(profiles);
    org.keycloak.representations.userprofile.config.UPConfig config =
        new org.keycloak.representations.userprofile.config.UPConfig();
    org.keycloak.representations.userprofile.config.UPAttribute embassy =
        new org.keycloak.representations.userprofile.config.UPAttribute("embassy");
    embassy.setValidations(Map.of("options", Map.of("options", List.of("Madrid", "Riyadh"))));
    config.addOrReplaceAttribute(embassy);
    when(profiles.getConfiguration()).thenReturn(config);

    Map<String, List<String>> byPost = EnrollmentChannels.channelsByPost(session, realm, "embassy");
    assertEquals(List.of("Madrid", "Riyadh"), List.copyOf(byPost.keySet()));
    assertEquals(List.of("WHATSAPP", "EMAIL"), byPost.get("Madrid"));
    assertEquals(List.of("VIBER", "EMAIL"), byPost.get("Riyadh"));
    assertEquals(Map.of(), EnrollmentChannels.channelsByPost(session, realm, null));
  }
}
