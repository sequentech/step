// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.io.Reader;
import java.nio.file.Files;
import java.nio.file.Path;
import java.text.MessageFormat;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.Properties;
import java.util.regex.Pattern;
import java.util.stream.Stream;
import org.junit.jupiter.api.Test;

class ThemeResourceMessagesTest {

  private static final Path MESSAGES = Path.of("src/main/resources/theme-resources/messages");

  /** The Sequent login, account and email themes' own messages. */
  private static final Path SEQUENT_THEMES = Path.of("../sequent-theme/src/main/resources/theme");

  /** An apostrophe on its own, which MessageFormat reads as the start of a quote. */
  private static final Pattern LONE_APOSTROPHE = Pattern.compile("(?<!')'(?!')");

  private static Properties load(Path file) throws IOException {
    Properties messages = new Properties();
    try (Reader reader = Files.newBufferedReader(file)) {
      messages.load(reader);
    }
    return messages;
  }

  /** What Keycloak's `msg()` shows: every message goes through MessageFormat. */
  private static String shown(Properties messages, String key) {
    return new MessageFormat(messages.getProperty(key), Locale.ROOT).format(new Object[0]);
  }

  @Test
  void profileDisplayNameAliasesUseTheExistingFieldTranslations() throws IOException {
    for (String language : List.of("ca", "en", "es", "eu", "gl", "tl")) {
      Properties messages = new Properties();
      try (Reader reader =
          Files.newBufferedReader(MESSAGES.resolve("messages_" + language + ".properties"))) {
        messages.load(reader);
      }

      assertEquals(
          messages.getProperty("dateOfBirth"),
          messages.getProperty("profile.attributes.dateOfBirth"),
          language);
      assertEquals(
          messages.getProperty("nationalId"),
          messages.getProperty("profile.attributes.nationalId"),
          language);
    }
  }

  @Test
  void theResendQuestionReadsAsASentence() throws IOException {
    // It said "Didnt receive the Code yet?": MessageFormat ate the lone apostrophe.
    Properties english = load(MESSAGES.resolve("messages_en.properties"));
    for (String flow : List.of("resetEmailOtp", "resetMobileOtp")) {
      assertEquals(
          "Didn't receive the code yet?",
          shown(english, flow + ".auth.resendTextPrefix.question"),
          flow);
    }
    assertEquals(
        "Didn't receive the code yet? Click here to resend",
        shown(english, "emailOtp.auth.resend.button"));
  }

  @Test
  void theResendQuestionSaysCodeInLowerCaseInEveryLanguage() throws IOException {
    for (String language : List.of("ca", "en", "es", "eu", "gl", "tl")) {
      Properties messages = load(MESSAGES.resolve("messages_" + language + ".properties"));
      for (String key : messages.stringPropertyNames()) {
        if (key.endsWith("resendTextPrefix.question")
            || key.equals("emailOtp.auth.resend.button")) {
          String text = shown(messages, key);
          assertFalse(
              text.matches(".*\\b(Code|Codi|Código|Kodea)\\b.*"),
              language + " " + key + ": " + text);
        }
      }
    }
  }

  @Test
  void everyApostropheIsEscapedForMessageFormat() throws IOException {
    List<Path> files = new ArrayList<>();
    for (Path root : List.of(MESSAGES, SEQUENT_THEMES)) {
      try (Stream<Path> walk = Files.walk(root)) {
        walk.filter(path -> path.getFileName().toString().matches("messages_.*\\.properties"))
            .forEach(files::add);
      }
    }
    assertTrue(files.size() > 10, "found " + files);
    List<String> lone = new ArrayList<>();
    for (Path file : files) {
      Properties messages = load(file);
      for (String key : messages.stringPropertyNames()) {
        if (LONE_APOSTROPHE.matcher(messages.getProperty(key)).find()) {
          lone.add(file + ": " + key);
        }
      }
    }
    assertEquals(List.of(), lone);
  }
}
