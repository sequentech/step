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

  private static final String LABELLED =
      """
      {"version": 1,
       "channels": [
         {"channel": "EMAIL", "purposes": ["OTP"]},
         {"channel": "WHATSAPP", "purposes": ["OTP"]},
         {"channel": "MESSENGER", "purposes": ["OTP"]}],
       "election_channels": {
         "11111111-aaaa-4bbb-8ccc-000000000001": ["EMAIL", "WHATSAPP"],
         "11111111-aaaa-4bbb-8ccc-000000000002": ["EMAIL", "MESSENGER"]},
       "election_labels": {
         "11111111-aaaa-4bbb-8ccc-000000000001": ["Synthetic Post North", "POST-N", "ext-17"],
         "11111111-aaaa-4bbb-8ccc-000000000002": ["Synthetic Post South"],
         "11111111-aaaa-4bbb-8ccc-000000000003": ["Synthetic Post East"]}}
      """;

  private final PublicMessagingChannels labelled = PublicMessagingChannels.parse(LABELLED).get();

  @Test
  void aPostIsMatchedToItsElectionByIdOrByAnyLabel() {
    List<MessageChannel> north = List.of(MessageChannel.EMAIL, MessageChannel.WHATSAPP);
    for (String post :
        List.of(
            "11111111-aaaa-4bbb-8ccc-000000000001",
            "11111111-AAAA-4BBB-8CCC-000000000001",
            "Synthetic Post North",
            "  synthetic post NORTH ",
            "post-n",
            "ext-17")) {
      assertEquals(north, labelled.channelsFor(MessagePurpose.OTP, Set.of(post)), post);
    }
    assertEquals(
        List.of(MessageChannel.EMAIL, MessageChannel.MESSENGER),
        labelled.channelsFor(MessagePurpose.OTP, Set.of("synthetic post south")));
  }

  @Test
  void aLabelledElectionWithoutRestrictionOffersEveryEnabledChannel() {
    assertEquals(
        List.of(MessageChannel.EMAIL, MessageChannel.WHATSAPP, MessageChannel.MESSENGER),
        labelled.channelsFor(MessagePurpose.OTP, Set.of("Synthetic Post East")));
  }

  @Test
  void anEnteredPostThatMatchesNoElectionOnlyGetsChannelsEveryRestrictedPostOffers() {
    PublicMessagingChannels.UnmatchedPostPolicy common =
        PublicMessagingChannels.UnmatchedPostPolicy.COMMON_CHANNELS;
    assertEquals(
        List.of(MessageChannel.EMAIL),
        labelled.channelsFor(MessagePurpose.OTP, Set.of("Synthetic Post Nowhere"), common));
    assertEquals(
        List.of(MessageChannel.EMAIL, MessageChannel.WHATSAPP),
        labelled.channelsFor(
            MessagePurpose.OTP, Set.of("Synthetic Post Nowhere", "Synthetic Post North"), common));
    assertEquals(
        List.of(MessageChannel.EMAIL, MessageChannel.WHATSAPP),
        labelled.channelsFor(MessagePurpose.OTP, Set.of("post-n"), common));
    assertEquals(
        List.of(MessageChannel.EMAIL, MessageChannel.WHATSAPP, MessageChannel.MESSENGER),
        labelled.channelsFor(MessagePurpose.OTP, Set.of("Synthetic Post East"), common));
  }

  @Test
  void aSavedVotersUnlistedElectionKeepsEveryEnabledChannel() {
    assertEquals(
        List.of(MessageChannel.EMAIL, MessageChannel.WHATSAPP, MessageChannel.MESSENGER),
        labelled.channelsFor(MessagePurpose.OTP, Set.of("11111111-aaaa-4bbb-8ccc-000000000009")));
  }

  @Test
  void aLabelTwoElectionsShareOnlyGetsWhatBothOffer() {
    PublicMessagingChannels shared =
        PublicMessagingChannels.parse(
                """
                {"version": 1,
                 "channels": [
                   {"channel": "EMAIL", "purposes": ["OTP"]},
                   {"channel": "WHATSAPP", "purposes": ["OTP"]}],
                 "election_channels": {"a": ["EMAIL", "WHATSAPP"], "b": ["EMAIL"]},
                 "election_labels": {"a": ["Synthetic Post"], "b": ["synthetic post"]}}
                """)
            .get();
    assertEquals(
        List.of(MessageChannel.EMAIL),
        shared.channelsFor(MessagePurpose.OTP, Set.of("Synthetic Post")));
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
