// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;

import java.io.IOException;
import java.io.Reader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Properties;
import java.util.stream.Stream;
import org.junit.jupiter.api.Test;

class MessagingMessagesTest {
  private static final Path MESSAGES = Path.of("src/main/resources/theme-resources/messages");
  private static final List<String> LANGUAGES =
      List.of("en", "es", "ca", "eu", "fr", "gl", "nl", "tl");

  static final List<String> KEYS =
      Stream.concat(
              Stream.of(MessageChannel.values()).map(MessageChannel::labelKey),
              Stream.of(
                  "messageOtp.sendCode.message.text",
                  "messageOtp.auth.sentTo",
                  "messageOtp.auth.sentToChannel",
                  "messageOtp.auth.openApp",
                  "messageOtp.auth.noChannel",
                  "messageOtp.delivery.unknown",
                  "messageOtp.delivery.failed",
                  "messageOtp.messenger.title",
                  "messageOtp.messenger.intro",
                  "messageOtp.messenger.connect",
                  "messageOtp.messenger.connectHelp",
                  "messageOtp.messenger.step1",
                  "messageOtp.messenger.step2",
                  "messageOtp.messenger.step3",
                  "messageOtp.messenger.scan",
                  "messageOtp.messenger.check",
                  "messageOtp.messenger.pending",
                  "messageOtp.messenger.codeSent",
                  "messageOtp.messenger.expired",
                  "messageOtp.messenger.notConfirmed",
                  "messageOtp.pending.channel",
                  "messageOtp.pending.channelAddress",
                  "messageOtp.auth.tooManyAttempts",
                  "messageOtp.auth.restart",
                  "messageOtp.auth.noCode",
                  "messageOtp.auth.channelNotOffered",
                  "messageOtp.auth.channelUnavailable",
                  "messageOtp.messenger.open",
                  "messageOtp.messenger.scanStep1",
                  "messageOtp.messenger.scanStep2",
                  "messageOtp.messenger.scanStep3",
                  "messageOtp.messenger.getStarted",
                  "messaging.number.SMS",
                  "messaging.email.account",
                  "forgotPassword.success.contact.message",
                  "newPassword.success.neutral.message",
                  "messaging.channelChoice.label",
                  "messaging.channelChoice.help",
                  "messaging.channelChoice.required",
                  "messaging.channelChoice.notAvailable",
                  "messaging.number.WHATSAPP",
                  "messaging.number.VIBER",
                  "messaging.number.help",
                  "messaging.number.invalid",
                  "messaging.consent",
                  "messaging.consent.required",
                  "newPassword.message.text",
                  "resetAppOtp.auth.enterContactTitle",
                  "resetAppOtp.auth.sendCodeButton",
                  "resetAppOtp.auth.enterOtpTitle",
                  "resetAppOtp.auth.sentToContact",
                  "resetAppOtp.auth.changeContact",
                  "resetAppOtp.auth.verifyButton",
                  "resetAppOtp.auth.resendTextPrefix.question",
                  "resetAppOtp.auth.ttlTime",
                  "resetAppOtp.auth.resend.timer",
                  "resetAppOtp.auth.resend.button.link",
                  "resetAppOtp.auth.error.invalidInput",
                  "resetAppOtp.auth.error.sendError",
                  "resetAppOtp.auth.error.resendTimer",
                  "resetAppOtp.auth.error.codeExpired",
                  "resetAppOtp.auth.error.codeInvalid",
                  "resetAppOtp.auth.error.maxReceiverReuse",
                  "resetAppOtp.auth.error.invalidCountry"))
          .toList();

  private static Properties load(String language) throws IOException {
    Properties properties = new Properties();
    try (Reader reader =
        Files.newBufferedReader(
            MESSAGES.resolve("messages_" + language + ".properties"), StandardCharsets.UTF_8)) {
      properties.load(reader);
    }
    return properties;
  }

  @Test
  void everyMessagingTextIsTranslatedInEveryLanguage() throws IOException {
    for (String language : LANGUAGES) {
      Properties properties = load(language);
      List<String> missing = new ArrayList<>();
      for (String key : KEYS) {
        if (properties.getProperty(key, "").isBlank()) {
          missing.add(key);
        }
      }
      assertEquals(List.of(), missing, language);
    }
  }

  @Test
  void theEnglishTextsFollowTheDesign() throws IOException {
    Properties english = load("en");
    assertEquals(
        "Delivery is not confirmed yet.", english.getProperty("messageOtp.delivery.unknown"));
    assertEquals("We sent a code to your {0}, {1}.", english.getProperty("messageOtp.auth.sentTo"));
    assertEquals(
        "Too many attempts. Start again.", english.getProperty("messageOtp.auth.tooManyAttempts"));
    assertEquals("Get codes and updates by", english.getProperty("messaging.channelChoice.label"));
    assertEquals(
        "Open {0} and tap Get Started to get your code.",
        english.getProperty("messageOtp.messenger.connectHelp"));
    assertEquals(
        "We will send the result to your {0}.", english.getProperty("messageOtp.pending.channel"));
    assertEquals(
        "If an account exists, we sent the link to its contact.",
        english.getProperty("forgotPassword.success.contact.message"));
  }

  /** No "another way", no picker, no linking word and no notice preference in any language. */
  @Test
  void removedTextsAreGoneEverywhere() throws IOException {
    for (String language : LANGUAGES) {
      for (String key : load(language).stringPropertyNames()) {
        assertFalse(
            key.startsWith("messageOtp.otherWay.")
                || key.startsWith("messageOtp.choose.")
                || key.equals("messageOtp.messenger.word")
                || key.equals("messaging.noticeChannel")
                || key.equals("forgotPassword.success.channel.message"),
            language + ": " + key);
      }
      for (String key :
          List.of("messageOtp.messenger.expired", "messageOtp.messenger.notConfirmed")) {
        assertFalse(load("en").getProperty(key).contains("another way"), key);
      }
    }
  }
}
