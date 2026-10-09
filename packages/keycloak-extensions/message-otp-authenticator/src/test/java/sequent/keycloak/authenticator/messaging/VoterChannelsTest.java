// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.util.EnumSet;
import java.util.List;
import java.util.Optional;
import java.util.Set;
import org.junit.jupiter.api.Test;

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

  @Test
  void withoutAProjectionOnlyEmailAndSmsAreOffered() {
    assertEquals(
        List.of(MessageChannel.EMAIL, MessageChannel.SMS),
        VoterChannels.offeredForOtp(Optional.empty(), Set.of()));
  }

  @Test
  void theDefaultSenderDeliversNoMessagingApp() {
    assertEquals(true, VoterChannels.deliverable(DEFAULT, MessageChannel.EMAIL));
    assertEquals(true, VoterChannels.deliverable(DEFAULT, MessageChannel.SMS));
    assertEquals(false, VoterChannels.deliverable(DEFAULT, MessageChannel.WHATSAPP));
    assertEquals(true, VoterChannels.deliverable(HARVEST, MessageChannel.MESSENGER));
  }

  @Test
  void aPostsChannelsComeInTheTicketsOrder() {
    assertEquals(
        List.of(MessageChannel.WHATSAPP, MessageChannel.MESSENGER, MessageChannel.EMAIL),
        VoterChannels.offeredForOtp(PROJECTION, Set.of("election-a")));
    assertEquals(
        List.of(
            MessageChannel.WHATSAPP,
            MessageChannel.MESSENGER,
            MessageChannel.SMS,
            MessageChannel.EMAIL),
        VoterChannels.offeredForOtp(PROJECTION, Set.of("election-other")));
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
  void destinationsAreMaskedLikeTheTicket() {
    assertEquals(
        "+966 \u2022\u2022\u2022 \u2022\u2022\u2022 567",
        VoterChannels.mask(MessageChannel.WHATSAPP, "+966501234567"));
    assertEquals(
        "+34 \u2022\u2022\u2022 \u2022\u2022\u2022 214",
        VoterChannels.mask(MessageChannel.VIBER, "+34612345214"));
    assertEquals(
        "+1 \u2022 \u2022\u2022\u2022 \u2022\u2022\u2022 456",
        VoterChannels.mask(MessageChannel.SMS, "+15550123456"));
    assertEquals(
        "+63 \u2022 \u2022\u2022\u2022 \u2022\u2022\u2022 567",
        VoterChannels.mask(MessageChannel.SMS, "+639171234567"));
    assertEquals(
        "ma\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022@example.com",
        VoterChannels.mask(MessageChannel.EMAIL, "maria.cruz@example.com"));
    assertEquals("", VoterChannels.mask(MessageChannel.MESSENGER, "psid-1"));
    assertEquals("", VoterChannels.mask(MessageChannel.SMS, null));
  }
}
