// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.List;
import java.util.Optional;
import java.util.Set;
import org.junit.jupiter.api.Test;
import sequent.keycloak.authenticator.CapturedLogs;

class PublicMessagingChannelsTest {
  private static final String PROJECTION =
      """
      {"version": 1,
       "channels": [
         {"channel": "EMAIL", "purposes": ["OTP", "NOTICE"], "sender_label": "Synthetic Commission"},
         {"channel": "WHATSAPP", "purposes": ["OTP", "NOTICE"], "sender_label": "+1 555 0100"},
         {"channel": "VIBER", "purposes": ["NOTICE"], "sender_label": "Synthetic"},
         {"channel": "MESSENGER", "purposes": ["OTP"], "sender_label": "Synthetic Page",
          "messenger_page": {"page_id": "100", "username": "synthetic.page", "name": "Synthetic Page"}},
         {"channel": "CARRIER_PIGEON", "purposes": ["OTP"]}
       ],
       "election_channels": {"election-a": ["EMAIL", "WHATSAPP"], "election-b": ["EMAIL", "MESSENGER"]}}
      """;

  private final PublicMessagingChannels channels = PublicMessagingChannels.parse(PROJECTION).get();

  @Test
  void otpChannelsFollowTheVotersElection() {
    assertEquals(
        List.of(MessageChannel.EMAIL, MessageChannel.WHATSAPP),
        channels.channelsFor(MessagePurpose.OTP, Set.of("election-a")));
    assertEquals(
        List.of(MessageChannel.EMAIL, MessageChannel.MESSENGER),
        channels.channelsFor(MessagePurpose.OTP, Set.of("election-b")));
  }

  @Test
  void anElectionWithoutRestrictionOffersEveryEnabledChannel() {
    assertEquals(
        List.of(MessageChannel.EMAIL, MessageChannel.WHATSAPP, MessageChannel.MESSENGER),
        channels.channelsFor(MessagePurpose.OTP, Set.of("election-unlisted")));
  }

  @Test
  void anUnknownPostOnlyGetsChannelsEveryRestrictedPostOffers() {
    assertEquals(List.of(MessageChannel.EMAIL), channels.channelsFor(MessagePurpose.OTP, Set.of()));
  }

  @Test
  void noticesAreSeparateFromCodes() {
    assertEquals(
        List.of(MessageChannel.EMAIL, MessageChannel.WHATSAPP, MessageChannel.VIBER),
        channels.channelsFor(MessagePurpose.NOTICE, Set.of("election-unlisted")));
  }

  @Test
  void labelsAndThePageArePublic() {
    assertEquals(Optional.of("+1 555 0100"), channels.senderLabel(MessageChannel.WHATSAPP));
    PublicMessagingChannels.MessengerPage page = channels.messengerPage().get();
    assertEquals("100", page.pageId());
    assertEquals("Synthetic Page", page.displayName());
    assertEquals(Optional.empty(), channels.senderLabel(MessageChannel.SMS));
  }

  @Test
  void aMissingOrBrokenProjectionOffersNothingAndLogsNoContent() {
    assertFalse(PublicMessagingChannels.parse(null).isPresent());
    assertFalse(PublicMessagingChannels.parse(" ").isPresent());
    try (CapturedLogs logs = new CapturedLogs(PublicMessagingChannels.class)) {
      assertFalse(PublicMessagingChannels.parse("{\"channels\": \"synthetic-secret\"").isPresent());
      assertFalse(logs.text().contains("synthetic-secret"));
    }
  }

  @Test
  void anotherVersionIsNotGuessed() {
    assertTrue(PublicMessagingChannels.parse("{\"version\": 1, \"channels\": []}").isPresent());
    assertFalse(PublicMessagingChannels.parse("{\"version\": 2, \"channels\": []}").isPresent());
  }
}
