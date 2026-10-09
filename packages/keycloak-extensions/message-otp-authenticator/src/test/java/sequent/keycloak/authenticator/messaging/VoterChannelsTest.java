// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import java.util.EnumSet;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import org.junit.jupiter.api.Test;
import org.keycloak.models.UserModel;

class VoterChannelsTest {
  private static final String MOBILE = "sequent.read-only.mobile-number";
  private static final Optional<PublicMessagingChannels> PROJECTION =
      PublicMessagingChannels.parse(
          """
          {"version": 1, "channels": [
            {"channel": "EMAIL", "purposes": ["OTP"]},
            {"channel": "SMS", "purposes": ["OTP"]},
            {"channel": "WHATSAPP", "purposes": ["OTP"]},
            {"channel": "VIBER", "purposes": ["NOTICE"]},
            {"channel": "MESSENGER", "purposes": ["OTP"]}],
           "election_channels": {"election-a": ["EMAIL", "WHATSAPP", "MESSENGER"]}}
          """);
  private static final MessageSenderProvider HARVEST =
      new HarvestMessageSenderProvider(
          java.net.URI.create("http://harvest"),
          EnumSet.of(MessageChannel.WHATSAPP, MessageChannel.VIBER, MessageChannel.MESSENGER),
          null,
          java.time.Duration.ofSeconds(1));
  private static final MessageSenderProvider DEFAULT = new DefaultMessageSenderProvider();

  private static final Map<MessageChannel, String> ALL_CONTACTS =
      Map.of(
          MessageChannel.EMAIL, "voter@example.test",
          MessageChannel.SMS, "+15550123456",
          MessageChannel.WHATSAPP, "+15550123456");

  @Test
  void withoutAProjectionOnlyEmailAndSmsAreOffered() {
    assertEquals(
        List.of(MessageChannel.EMAIL, MessageChannel.SMS),
        VoterChannels.eligibleForOtp(
            VoterChannels.offeredForOtp(Optional.empty(), Set.of()),
            HARVEST,
            ALL_CONTACTS,
            Optional.empty()));
  }

  @Test
  void theDefaultSenderOffersNoMessagingAppChannels() {
    assertEquals(
        List.of(MessageChannel.EMAIL, MessageChannel.SMS),
        VoterChannels.eligibleForOtp(
            VoterChannels.offeredForOtp(PROJECTION, Set.of("election-other")),
            DEFAULT,
            ALL_CONTACTS,
            Optional.empty()));
  }

  @Test
  void enrollmentOffersThePostsChannelsAndMessengerNeedsNoContactYet() {
    assertEquals(
        List.of(MessageChannel.EMAIL, MessageChannel.WHATSAPP, MessageChannel.MESSENGER),
        VoterChannels.eligibleForOtp(
            VoterChannels.offeredForOtp(PROJECTION, Set.of("election-a")),
            HARVEST,
            ALL_CONTACTS,
            Optional.empty()));
  }

  @Test
  void signInOffersOnlyVerifiedContacts() {
    assertEquals(
        List.of(MessageChannel.WHATSAPP),
        VoterChannels.eligibleForOtp(
            VoterChannels.offeredForOtp(PROJECTION, Set.of("election-a")),
            HARVEST,
            ALL_CONTACTS,
            Optional.of(Set.of(MessageChannel.WHATSAPP, MessageChannel.MESSENGER))));
  }

  @Test
  void aChannelWithoutAContactIsNotOffered() {
    assertEquals(
        List.of(MessageChannel.EMAIL),
        VoterChannels.eligibleForOtp(
            VoterChannels.offeredForOtp(PROJECTION, Set.of("election-a")),
            HARVEST,
            Map.of(MessageChannel.EMAIL, "voter@example.test"),
            Optional.of(Set.of(MessageChannel.EMAIL, MessageChannel.WHATSAPP))));
  }

  @Test
  void votersFromBeforeVerifiedChannelsKeepTheirEmailAndMobile() {
    UserModel user = mock(UserModel.class);
    when(user.getEmail()).thenReturn("voter@example.test");
    when(user.getFirstAttribute(MOBILE)).thenReturn("+15550123456");
    when(user.getAttributeStream(MessagingAttributes.VERIFIED_CHANNELS))
        .thenAnswer(i -> java.util.stream.Stream.of());
    assertEquals(
        Set.of(MessageChannel.EMAIL, MessageChannel.SMS), VoterChannels.verified(user, MOBILE));
  }

  @Test
  void verifiedChannelsAreReadAndUnknownValuesIgnored() {
    UserModel user = mock(UserModel.class);
    when(user.getAttributeStream(MessagingAttributes.VERIFIED_CHANNELS))
        .thenAnswer(i -> java.util.stream.Stream.of("VIBER", "smoke-signal"));
    assertEquals(Set.of(MessageChannel.VIBER), VoterChannels.verified(user, MOBILE));
  }

  @Test
  void contactsComeFromTheVoter() {
    UserModel user = mock(UserModel.class);
    when(user.getEmail()).thenReturn("voter@example.test");
    when(user.getFirstAttribute(MOBILE)).thenReturn(" ");
    when(user.getFirstAttribute(MessagingAttributes.VIBER_NUMBER)).thenReturn("+15550100");
    when(user.getFirstAttribute(MessagingAttributes.MESSENGER_ID)).thenReturn("psid-1");
    assertEquals(
        Map.of(
            MessageChannel.EMAIL, "voter@example.test",
            MessageChannel.VIBER, "+15550100",
            MessageChannel.MESSENGER, "psid-1"),
        VoterChannels.contacts(user, MOBILE));
  }

  @Test
  void phoneNumbersMustBeE164() {
    assertEquals(true, VoterChannels.isE164("+639171234567"));
    assertEquals(false, VoterChannels.isE164("09171234567"));
    assertEquals(false, VoterChannels.isE164("+0123456789"));
    assertEquals(false, VoterChannels.isE164("+1 555 0123"));
    assertEquals(false, VoterChannels.isE164(null));
  }

  @Test
  void destinationsAreMaskedForDisplay() {
    assertEquals("+155*****456", VoterChannels.mask(MessageChannel.WHATSAPP, "+15550123456"));
    assertEquals(
        "vo***@*****le.test", VoterChannels.mask(MessageChannel.EMAIL, "voter@example.test"));
    assertEquals("****", VoterChannels.mask(MessageChannel.SMS, "+123"));
    assertEquals("", VoterChannels.mask(MessageChannel.MESSENGER, "psid-1"));
  }
}
