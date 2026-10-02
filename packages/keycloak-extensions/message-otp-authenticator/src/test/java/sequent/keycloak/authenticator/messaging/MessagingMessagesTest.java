// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import static org.junit.jupiter.api.Assertions.assertEquals;

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
  private static final List<String> LANGUAGES = List.of("en", "es", "ca", "eu", "gl", "tl");

  static final List<String> KEYS =
      Stream.concat(
              Stream.of(MessageChannel.values()).map(MessageChannel::labelKey),
              Stream.of(
                  "messageOtp.sendCode.message.text",
                  "messageOtp.auth.sentTo",
                  "messageOtp.auth.sentToChannel",
                  "messageOtp.auth.openApp",
                  "messageOtp.auth.noChannel",
                  "messageOtp.otherWay.title",
                  "messageOtp.otherWay.help",
                  "messageOtp.otherWay.send",
                  "messageOtp.choose.title",
                  "messageOtp.choose.help",
                  "messageOtp.choose.option",
                  "messageOtp.delivery.unknown",
                  "messageOtp.delivery.failed",
                  "messageOtp.messenger.title",
                  "messageOtp.messenger.intro",
                  "messageOtp.messenger.connect",
                  "messageOtp.messenger.connectHelp",
                  "messageOtp.messenger.step1",
                  "messageOtp.messenger.step2",
                  "messageOtp.messenger.step3",
                  "messageOtp.messenger.word",
                  "messageOtp.messenger.scan",
                  "messageOtp.messenger.check",
                  "messageOtp.messenger.pending",
                  "messageOtp.messenger.codeSent",
                  "messageOtp.messenger.expired",
                  "messageOtp.messenger.notConfirmed",
                  "messageOtp.pending.channel",
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
                  "messaging.noticeChannel",
                  "newPassword.message.text"))
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
    assertEquals("Get the code another way", english.getProperty("messageOtp.otherWay.title"));
    assertEquals(
        "Choose an available method. Requesting a new code replaces the previous code.",
        english.getProperty("messageOtp.otherWay.help"));
    assertEquals(
        "Delivery is not confirmed yet.", english.getProperty("messageOtp.delivery.unknown"));
    assertEquals("We sent a code to your {0}, {1}.", english.getProperty("messageOtp.auth.sentTo"));
    assertEquals("Connect Messenger", english.getProperty("messageOtp.messenger.connect"));
    assertEquals(
        "We will send the result to your {0}.", english.getProperty("messageOtp.pending.channel"));
  }
}
