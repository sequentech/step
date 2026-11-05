// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.theme;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.io.Reader;
import java.nio.file.Files;
import java.nio.file.Path;
import java.text.MessageFormat;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Properties;
import org.junit.jupiter.api.Test;

class MessageBundleTest {

  private static final Path MESSAGES =
      Path.of("src/main/resources/theme/sequent.admin-portal/login/messages");

  @Test
  void catalanApostrophesSurviveKeycloakMessageFormatting() throws IOException {
    Properties messages = load("ca");

    assertEquals(
        "Introduïu el PIN de la vostra carta d'informació electoral.",
        format(messages, "structuredCredentialHint", Locale.forLanguageTag("ca")));
    assertEquals(
        "El nom d'usuari o el PIN són incorrectes.",
        format(messages, "structuredCredentialError", Locale.forLanguageTag("ca")));
  }

  @Test
  void frenchApostrophesSurviveKeycloakMessageFormatting() throws IOException {
    Properties messages = load("fr");
    Locale locale = Locale.forLanguageTag("fr");

    assertEquals("Connectez-vous pour continuer", format(messages, "loginAccountTitle", locale));
    assertEquals("RETOUR À L'APPLICATION", format(messages, "backToApplication", locale));
    assertEquals(
        "Saisissez le code PIN figurant sur votre lettre d'information électorale.",
        format(messages, "structuredCredentialHint", locale));
    assertEquals(
        "Le nom d'utilisateur ou le code PIN est incorrect.",
        format(messages, "structuredCredentialError", locale));
  }

  @Test
  void everyLocaleDefinesTheGenericCredentialsMessage() throws IOException {
    // MultiAttributePasswordAuthenticator uses this instead of Keycloak's invalidUserMessage,
    // which reads "Invalid username or password" on a form that has no username field. A locale
    // missing the key would fall back to showing the key name to the voter.
    // One stem per language, so a translation naming the username fails in its own locale.
    Map<String, List<String>> forbidden =
        Map.of(
            "ca", List.of("usuari"),
            "en", List.of("username"),
            "es", List.of("usuari"),
            "eu", List.of("erabiltzaile"),
            "fr", List.of("utilisateur"),
            "gl", List.of("usuari"),
            "nl", List.of("gebruikersnaam"),
            "tl", List.of("username", "gumagamit"));
    for (String language : forbidden.keySet()) {
      String message = load(language).getProperty("invalidCredentialsMessage");
      assertTrue(message != null && !message.isBlank(), language + " is missing the key");
      for (String term : forbidden.get(language)) {
        assertFalse(
            message.toLowerCase(Locale.ROOT).contains(term),
            language + " still names a username: " + message);
      }
    }
  }

  @Test
  void everyLocalePreservesAllGroupStatusPlaceholders() throws IOException {
    for (String language : List.of("ca", "en", "es", "eu", "fr", "gl", "nl", "tl")) {
      String status = load(language).getProperty("structuredCredentialGroupStatus");
      for (int index = 0; index < 4; index += 1) {
        assertTrue(status.contains("{" + index + "}"), language + " is missing {" + index + "}");
      }
    }
  }

  @Test
  void everyLocaleDefinesTheEnrollmentWindowTextsWithTheirArguments() throws IOException {
    // Key -> how many {n} arguments it takes (VOTE-LIFECYCLE design §4).
    Map<String, Integer> keys =
        Map.of(
            "enrollment.opensOn", 3,
            "enrollment.openUntil", 2,
            "enrollment.replyBy", 1,
            "enrollment.postNotOpen", 1,
            "enrollment.postNotConfigured", 1,
            "timezones.voterDateTimeZone", 2);
    for (String language : List.of("ca", "en", "es", "eu", "fr", "gl", "nl", "tl")) {
      Properties messages = load(language);
      for (Map.Entry<String, Integer> key : keys.entrySet()) {
        String text = messages.getProperty(key.getKey());
        assertTrue(text != null && !text.isBlank(), language + " is missing " + key.getKey());
        Object[] arguments = new Object[key.getValue()];
        for (int index = 0; index < arguments.length; index += 1) {
          arguments[index] = "<" + index + ">";
        }
        String formatted =
            new MessageFormat(text, Locale.forLanguageTag(language)).format(arguments);
        for (Object argument : arguments) {
          assertTrue(
              formatted.contains((String) argument),
              language + " " + key.getKey() + " drops " + argument + ": " + formatted);
        }
        // An apostrophe left single would swallow the rest of the text.
        assertFalse(formatted.contains("{"), language + " " + key.getKey() + ": " + formatted);
        assertEquals(text.contains("''"), formatted.contains("'"), language + " " + key.getKey());
      }
    }
  }

  private static Properties load(String language) throws IOException {
    Properties messages = new Properties();
    try (Reader reader =
        Files.newBufferedReader(MESSAGES.resolve("messages_" + language + ".properties"))) {
      messages.load(reader);
    }
    return messages;
  }

  private static String format(Properties messages, String key, Locale locale) {
    return new MessageFormat(messages.getProperty(key), locale).format(new Object[0]);
  }
}
