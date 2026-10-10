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

  private static final String B = "\u2022";
  private static final String HIDDEN = B.repeat(3);

  /**
   * R7-11: the same vectors as Rust's {@code messaging::destination} tests. Fixed groups ({@code
   * ••• •••}) and eight email bullets, so the mask never reveals the length; the calling code comes
   * from sequent-core's table.
   */
  @Test
  void destinationsAreMaskedExactlyLikeRust() {
    assertEquals(
        "+63 " + HIDDEN + " " + HIDDEN + " 567",
        VoterChannels.mask(MessageChannel.WHATSAPP, "+63 (917) 123-4567"));
    assertEquals(
        "+1 " + HIDDEN + " " + HIDDEN + " 456",
        VoterChannels.mask(MessageChannel.SMS, "+15550123456"));
    assertEquals(
        "+44 " + HIDDEN + " " + HIDDEN + " 123",
        VoterChannels.mask(MessageChannel.SMS, "+447700900123"));
    assertEquals(
        "+971 " + HIDDEN + " " + HIDDEN + " 567",
        VoterChannels.mask(MessageChannel.VIBER, "+971501234567"));
    assertEquals(
        "+966 " + HIDDEN + " " + HIDDEN + " 567",
        VoterChannels.mask(MessageChannel.WHATSAPP, "+966501234567"));
    assertEquals(
        "vo" + B.repeat(8) + "@example.org",
        VoterChannels.mask(MessageChannel.EMAIL, " Voter@Example.ORG "));
    assertEquals(
        "a" + B.repeat(8) + "@example.org",
        VoterChannels.mask(MessageChannel.EMAIL, "ab@example.org"));
    assertEquals(
        B.repeat(8) + "@example.org", VoterChannels.mask(MessageChannel.EMAIL, "a@example.org"));
    assertEquals(
        "ma" + B.repeat(8) + "@example.com",
        VoterChannels.mask(MessageChannel.EMAIL, "maria.cruz@example.com"));
    assertEquals(HIDDEN + " 321", VoterChannels.mask(MessageChannel.MESSENGER, "6543210987654321"));
    assertEquals("", VoterChannels.mask(MessageChannel.SMS, null));
    assertEquals("", VoterChannels.mask(MessageChannel.EMAIL, " "));
  }

  /** What Rust refuses to parse is never shown: only bullets. */
  @Test
  void anAddressRustWouldRefuseShowsNothing() {
    for (String number : List.of("09171234567", "+63abc", "+1234", "+0123456789")) {
      assertEquals(HIDDEN, VoterChannels.mask(MessageChannel.SMS, number), number);
    }
    for (String email : List.of("voter", "a b@example.org", "@example.org", "voter@example")) {
      assertEquals(HIDDEN, VoterChannels.mask(MessageChannel.EMAIL, email), email);
    }
    assertEquals(HIDDEN, VoterChannels.mask(MessageChannel.MESSENGER, "+639171234567"));
  }

  /** Voter pages name the Messenger Page instead: a Page-scoped ID means nothing to the voter. */
  @Test
  void votersSeeNoMessengerId() {
    assertEquals("", VoterChannels.display(MessageChannel.MESSENGER, "6543210987654321"));
    assertEquals(
        "+63 " + HIDDEN + " " + HIDDEN + " 567",
        VoterChannels.display(MessageChannel.SMS, "+639171234567"));
  }

  /** The calling codes are sequent-core's table, read from the Rust source. */
  @Test
  void callingCodesAreSequentCoresTable() throws Exception {
    String rust =
        java.nio.file.Files.readString(
            java.nio.file.Path.of("../../sequent-core/src/types/dial_codes.rs"));
    java.util.Set<String> codes = new java.util.TreeSet<>();
    for (String table : List.of("pub const CALLING_CODES", "pub const NON_GEOGRAPHIC_CODES")) {
      int from = rust.indexOf(table);
      String block = rust.substring(from, rust.indexOf("];", from));
      java.util.regex.Matcher code =
          java.util.regex.Pattern.compile("\"([0-9]{1,3})\"").matcher(block);
      while (code.find()) {
        codes.add(code.group(1));
      }
    }
    assertEquals(216, codes.size());
    assertEquals(codes, new java.util.TreeSet<>(VoterChannels.CALLING_CODES));
  }
}
