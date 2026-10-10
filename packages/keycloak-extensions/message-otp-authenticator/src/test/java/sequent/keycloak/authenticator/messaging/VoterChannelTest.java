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
  void recordedChannelWithoutDestinationHasNoChannel() {
    // Never another channel: the voter's channel can't be used, so none is.
    assertTrue(
        resolve(
                "voter@example.org",
                Map.of(MessagingAttributes.MESSAGE_CHANNEL, List.of("WHATSAPP")))
            .isEmpty());
    assertTrue(
        resolve(
                "voter@example.org",
                Map.of(
                    MessagingAttributes.MESSAGE_CHANNEL, List.of("MESSENGER"),
                    MessagingAttributes.MESSENGER_ID, List.of("psid-1"),
                    Utils.PHONE_NUMBER_ATTRIBUTE, List.of("+15550000003")))
            .isEmpty());
  }

  @Test
  void recordedSmsReadsTheMobileNumberWithoutAMessageNumber() {
    VoterChannel found =
        resolve(
                "voter@example.org",
                Map.of(
                    MessagingAttributes.MESSAGE_CHANNEL, List.of("SMS"),
                    Utils.PHONE_NUMBER_ATTRIBUTE, List.of("+15550000003")))
            .orElseThrow();
    assertEquals(MessageChannel.SMS, found.channel());
    assertEquals("+15550000003", found.destination());
    assertEquals(VoterChannel.Source.RECORDED, found.source());
  }

  @Test
  void firstReleaseRecordWithoutDestinationHasNoChannel() {
    assertTrue(
        resolve(
                "voter@example.org",
                Map.of(
                    MessagingAttributes.VERIFIED_CHANNELS, List.of("WHATSAPP"),
                    Utils.PHONE_NUMBER_ATTRIBUTE, List.of("+15550000003")))
            .isEmpty());
  }

  @Test
  void emptyVerifiedChannelsIsNotAFirstReleaseRecord() {
    VoterChannel found =
        resolve("voter@example.org", Map.of(MessagingAttributes.VERIFIED_CHANNELS, List.of()))
            .orElseThrow();
    assertEquals(MessageChannel.EMAIL, found.channel());
    assertEquals(VoterChannel.Source.LEGACY, found.source());
  }

  /**
   * R7-12: Rust's rule ({@code voter_channel::parse_channel_value}): Unicode whitespace trimmed
   * ({@code str::trim}), ASCII case only ({@code to_ascii_uppercase}). Shared vectors with Rust's
   * {@code channel_values_are_read_trimmed_and_in_any_case} and the Admin Portal.
   */
  @Test
  void channelValuesAreReadLikeRust() {
    Map<String, Optional<MessageChannel>> vectors = new java.util.LinkedHashMap<>();
    vectors.put("whatsapp", Optional.of(MessageChannel.WHATSAPP));
    vectors.put(" SMS ", Optional.of(MessageChannel.SMS));
    vectors.put(" Viber", Optional.of(MessageChannel.VIBER));
    vectors.put("Messenger\t", Optional.of(MessageChannel.MESSENGER));
    // Unicode whitespace is trimmed, as Rust's str::trim does.
    vectors.put("\u00A0SMS", Optional.of(MessageChannel.SMS));
    vectors.put("\u3000sms\u2029", Optional.of(MessageChannel.SMS));
    vectors.put("email\u0085", Optional.of(MessageChannel.EMAIL));
    // Control characters that aren't whitespace, and zero-width spaces, are not trimmed.
    vectors.put("\u001Csms", Optional.empty());
    vectors.put("sms\u200B", Optional.empty());
    // Only ASCII letters change case.
    vectors.put("\u017Fms", Optional.empty());
    vectors.put("v\u0131ber", Optional.empty());
    vectors.put("\uFF33\uFF2D\uFF33", Optional.empty());
    vectors.put("", Optional.empty());
    vectors.put("   ", Optional.empty());
    vectors.put("FAX", Optional.empty());
    for (Map.Entry<String, Optional<MessageChannel>> vector : vectors.entrySet()) {
      assertEquals(
          vector.getValue(),
          MessageChannel.parse(vector.getKey()),
          () -> "parse(" + vector.getKey().codePoints().boxed().toList() + ")");
    }
    assertEquals(Optional.empty(), MessageChannel.parse(null));
  }

  /** The record's shape, whether or not its channel can be used (R7-01). */
  @Test
  void theRecordsShapeIsReadWithoutItsDestination() {
    assertEquals(
        VoterChannel.Source.RECORDED,
        VoterChannel.shape(
            name ->
                name.equals(MessagingAttributes.MESSAGE_CHANNEL)
                    ? java.util.stream.Stream.of(" whatsapp ")
                    : java.util.stream.Stream.empty()));
    assertEquals(
        VoterChannel.Source.VERIFIED_CHANNELS,
        VoterChannel.shape(
            name ->
                name.equals(MessagingAttributes.VERIFIED_CHANNELS)
                    ? java.util.stream.Stream.of("VIBER")
                    : java.util.stream.Stream.empty()));
    assertEquals(
        VoterChannel.Source.LEGACY,
        VoterChannel.shape(
            name ->
                name.equals(MessagingAttributes.MESSAGE_CHANNEL)
                    ? java.util.stream.Stream.of("FAX")
                    : java.util.stream.Stream.empty()));
    assertEquals(VoterChannel.Source.LEGACY, VoterChannel.shape(name -> null));
  }
}
