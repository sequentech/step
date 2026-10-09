// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.stream.Stream;
import org.junit.jupiter.api.Test;
import sequent.keycloak.authenticator.Utils;

/** The same cases as sequent-core's {@code voter_channel_tests.rs}. */
class VoterChannelTest {
  private static Optional<VoterChannel> resolve(String email, Map<String, List<String>> attrs) {
    return VoterChannel.resolve(
        email, name -> attrs.containsKey(name) ? attrs.get(name).stream() : Stream.empty());
  }

  @Test
  void recordedPhoneChannel() {
    VoterChannel found =
        resolve(
                "voter@example.org",
                Map.of(
                    MessagingAttributes.MESSAGE_CHANNEL, List.of("VIBER"),
                    MessagingAttributes.MESSAGE_NUMBER, List.of("+639171234567"),
                    Utils.PHONE_NUMBER_ATTRIBUTE, List.of("+639170000000")))
            .orElseThrow();
    assertEquals(MessageChannel.VIBER, found.channel());
    assertEquals("+639171234567", found.destination());
    assertEquals(VoterChannel.Source.RECORDED, found.source());
  }

  @Test
  void recordedMessengerNeedsItsPage() {
    VoterChannel found =
        resolve(
                null,
                Map.of(
                    MessagingAttributes.MESSAGE_CHANNEL, List.of("MESSENGER"),
                    MessagingAttributes.MESSENGER_ID, List.of("psid-1"),
                    MessagingAttributes.MESSENGER_PAGE, List.of("page-1")))
            .orElseThrow();
    assertEquals(MessageChannel.MESSENGER, found.channel());
    assertEquals(Optional.of("page-1"), found.messengerPage());

    assertTrue(
        resolve(
                null,
                Map.of(
                    MessagingAttributes.MESSAGE_CHANNEL, List.of("MESSENGER"),
                    MessagingAttributes.MESSENGER_ID, List.of("psid-1")))
            .isEmpty());
  }

  @Test
  void firstReleaseRecordPrefersItsVerifiedPreference() {
    VoterChannel found =
        resolve(
                "voter@example.org",
                Map.of(
                    MessagingAttributes.MESSAGE_CHANNEL, List.of("VIBER"),
                    MessagingAttributes.VERIFIED_CHANNELS, List.of("WHATSAPP", "VIBER"),
                    MessagingAttributes.WHATSAPP_NUMBER, List.of("+15550000001"),
                    MessagingAttributes.VIBER_NUMBER, List.of("+15550000002")))
            .orElseThrow();
    assertEquals(MessageChannel.VIBER, found.channel());
    assertEquals("+15550000002", found.destination());
    assertEquals(VoterChannel.Source.VERIFIED_CHANNELS, found.source());
  }

  @Test
  void firstReleaseRecordIgnoresAnUnverifiedPreference() {
    VoterChannel found =
        resolve(
                "voter@example.org",
                Map.of(
                    MessagingAttributes.MESSAGE_CHANNEL, List.of("EMAIL"),
                    MessagingAttributes.VERIFIED_CHANNELS, List.of("SMS", "WHATSAPP"),
                    MessagingAttributes.WHATSAPP_NUMBER, List.of("+15550000001"),
                    Utils.PHONE_NUMBER_ATTRIBUTE, List.of("+15550000003")))
            .orElseThrow();
    assertEquals(MessageChannel.WHATSAPP, found.channel());
  }

  @Test
  void legacyEmailThenMobile() {
    Map<String, List<String>> mobile =
        Map.of(Utils.PHONE_NUMBER_ATTRIBUTE, List.of("+15550000003"));
    VoterChannel email = resolve("voter@example.org", mobile).orElseThrow();
    assertEquals(MessageChannel.EMAIL, email.channel());
    assertEquals(VoterChannel.Source.LEGACY, email.source());

    VoterChannel sms = resolve("", mobile).orElseThrow();
    assertEquals(MessageChannel.SMS, sms.channel());
    assertEquals("+15550000003", sms.destination());

    assertTrue(resolve(null, Map.of()).isEmpty());
  }

  @Test
  void recordedChannelWithoutDestinationIsLegacy() {
    VoterChannel found =
        resolve(
                "voter@example.org",
                Map.of(MessagingAttributes.MESSAGE_CHANNEL, List.of("WHATSAPP")))
            .orElseThrow();
    assertEquals(MessageChannel.EMAIL, found.channel());
    assertEquals(VoterChannel.Source.LEGACY, found.source());
  }
}
