// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import freemarker.cache.FileTemplateLoader;
import freemarker.core.HTMLOutputFormat;
import freemarker.template.Configuration;
import freemarker.template.Template;
import java.io.IOException;
import java.io.Reader;
import java.io.StringWriter;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.Test;

class MessageOtpTemplateTest {
  private static final Path TEMPLATES = Path.of("src/main/resources/theme-resources/templates");
  private static final Path TEMPLATE = TEMPLATES.resolve("message-otp.login.ftl");
  private static final Path BASE_LOGIN = Path.of("src/main/resources/theme/base/login");

  private static String parsed(Path path) throws IOException {
    Configuration configuration = new Configuration(Configuration.VERSION_2_3_32);
    configuration.setOutputFormat(HTMLOutputFormat.INSTANCE);
    try (Reader reader = Files.newBufferedReader(path)) {
      new Template(path.getFileName().toString(), reader, configuration);
    }
    return Files.readString(path);
  }

  @Test
  void theCodePageParsesAndOffersNoOtherChannel() throws IOException {
    String template = parsed(TEMPLATE);
    assertFalse(template.contains("name=\"channel\""));
    assertFalse(template.contains("otherWay"));
    assertFalse(template.contains("messageOtp.choose."));
    assertFalse(template.contains("messengerWord"));
    assertTrue(template.contains("name=\"messengerStatus\" value=\"true\""));
    assertTrue(template.contains("msg(\"messageOtp.delivery.unknown\")"));
  }

  @Test
  void theCodePagesOfferOnlyARestartOnceTheAttemptsAreUsedUp() throws IOException {
    for (Path page : new Path[] {TEMPLATE, BASE_LOGIN.resolve("message-otp.enter-otp.ftl")}) {
      String template = parsed(page);
      assertTrue(template.contains("(attemptsExhausted!false)"), page.toString());
      assertTrue(template.contains("href=\"${url.loginRestartFlowUrl}\""), page.toString());
      assertTrue(template.contains("msg(\"messageOtp.auth.restart\")"), page.toString());
    }
  }

  @Test
  void theMessengerPagesShowTheQrCodeOpenMessengerAndGetStarted() throws IOException {
    for (Path page : new Path[] {TEMPLATE, BASE_LOGIN.resolve("message-otp.enter-otp.ftl")}) {
      String template = parsed(page);
      assertTrue(template.contains("src=\"${messengerQr}\""), page.toString());
      assertTrue(template.contains("msg(\"messageOtp.messenger.open\")"), page.toString());
      assertTrue(template.contains("msg(\"messageOtp.messenger.getStarted\")"), page.toString());
      assertTrue(template.contains("msg(\"messageOtp.messenger.scanStep1\")"), page.toString());
    }
  }

  /** R3-15: the Messenger intro is the ticket's, with the organization and the Page. */
  @Test
  void theMessengerIntroNamesTheOrganizationAndThePage() throws IOException {
    for (Path page : new Path[] {TEMPLATE, BASE_LOGIN.resolve("message-otp.enter-otp.ftl")}) {
      assertTrue(
          parsed(page)
              .contains(
                  "msg(\"messageOtp.messenger.introPage\", messagingOrganization, messengerPage)"),
          page.toString());
    }
  }

  /** R3-04: the neutral code page says only that a code went to the account's contact, if any. */
  @Test
  void theNeutralCodePageNamesNoChannel() throws IOException {
    String template = parsed(TEMPLATE);
    int neutral = template.indexOf("(codeDisclosure!'') == 'NEUTRAL'");
    assertTrue(neutral > 0);
    assertTrue(template.indexOf("msg(\"messageOtp.auth.neutralSent\")") > neutral);
    assertTrue(neutral < template.indexOf("messageOtp.auth.sentTo"));
  }

  /** Lead decision: a neutral page shows the Messenger box to everyone, without link state. */
  @Test
  void theNeutralPageShowsTheMessengerBoxToEveryone() throws IOException {
    String template = parsed(TEMPLATE);
    assertTrue(template.contains("(channel?? && channel == 'MESSENGER') || neutralMessenger"));
    assertTrue(template.contains("msg(\"messageOtp.messenger.ifMessenger\")"));
  }

  /** The QR code is drawn on the server, as an SVG data URI of the m.me link. */
  @Test
  void theQrCodeIsRendered() {
    String link = "https://m.me/synthetic?ref=ref-1";
    String uri = MessengerQr.dataUri(link).orElseThrow();
    assertTrue(uri.startsWith(MessengerQr.DATA_URI_PREFIX));
    String svg =
        new String(
            java.util.Base64.getDecoder()
                .decode(uri.substring(MessengerQr.DATA_URI_PREFIX.length())),
            java.nio.charset.StandardCharsets.UTF_8);
    assertTrue(svg.startsWith("<svg xmlns=\"http://www.w3.org/2000/svg\""));
    assertTrue(svg.contains("<path fill=\"#000\" d=\"M"));
    assertFalse(svg.contains(link));
    assertTrue(MessengerQr.dataUri("javascript:alert(1)").isEmpty());
    assertTrue(MessengerQr.dataUri(null).isEmpty());
  }

  @Test
  void theCodePagesCheckMessengerAgainByThemselves() throws IOException {
    assertTrue(parsed(TEMPLATE).contains("<#include \"messenger-status-poll.ftl\">"));
    String contactCode = parsed(BASE_LOGIN.resolve("message-otp.enter-otp.ftl"));
    assertTrue(contactCode.contains("<#include \"messenger-status-poll.ftl\">"));
    assertTrue(contactCode.contains("name=\"messengerStatus\" value=\"true\""));
  }

  /**
   * R7-17: the contact is escaped inside the "sent to" message, whose own markup (the bold contact)
   * stays. The page's own expression is rendered, with a contact that carries markup.
   */
  @Test
  void theContactIsEscapedInTheSentToMessage() throws Exception {
    String page = parsed(BASE_LOGIN.resolve("message-otp.enter-otp.ftl"));
    java.util.regex.Matcher line =
        java.util.regex.Pattern.compile("\\$\\{[^\\n]*\\.auth\\.sentToContact\"[^\\n]*\\}")
            .matcher(page);
    assertTrue(line.find());
    Configuration configuration = new Configuration(Configuration.VERSION_2_3_32);
    configuration.setOutputFormat(HTMLOutputFormat.INSTANCE);
    Template template =
        new Template("sentToContact", new java.io.StringReader(line.group()), configuration);
    freemarker.template.TemplateMethodModelEx msg =
        arguments ->
            new java.text.MessageFormat("Verification code sent to <strong>{0}</strong>")
                .format(
                    new Object[] {
                      ((freemarker.template.TemplateScalarModel) arguments.get(1)).getAsString()
                    });
    java.io.StringWriter out = new java.io.StringWriter();
    template.process(
        java.util.Map.of(
            "msg", msg, "i18nPrefix", "resetAppOtp", "contact", "<img src=x onerror=alert(1)>"),
        out);
    assertEquals(
        "Verification code sent to <strong>&lt;img src=x onerror=alert(1)&gt;</strong>",
        out.toString());
  }

  /** R7-10: the Messenger box says why a voter connects Messenger again after a Page change. */
  @Test
  void theMessengerPagesExplainAReconnect() throws IOException {
    for (Path page : new Path[] {TEMPLATE, BASE_LOGIN.resolve("message-otp.enter-otp.ftl")}) {
      String template = parsed(page);
      assertTrue(template.contains("<#if (messengerReconnect!false)>"), page.toString());
      assertTrue(template.contains("msg(\"messageOtp.messenger.reconnect\")"), page.toString());
    }
  }

  /** KC-R4: every send starts the countdown on the code pages, a failed one too. */
  @Test
  void aFailedSendStartsTheCountdownOnTheCodePages() throws IOException {
    for (Path page : new Path[] {TEMPLATE, BASE_LOGIN.resolve("message-otp.enter-otp.ftl")}) {
      String template = parsed(page);
      assertFalse(template.contains("sendFailed"), page.toString());
      assertTrue(template.contains("resendTimerStarted!codeJustSent"), page.toString());
    }
  }

  @Test
  void theMessagingAppFormAsksForTheAppItsContactAndConsent() throws IOException {
    String template = parsed(BASE_LOGIN.resolve("message-otp.enter-app-contact.ftl"));
    assertTrue(template.contains("name=\"channel\""));
    assertTrue(template.contains("name=\"contact\""));
    assertTrue(template.contains("name=\"sequent.message-consent\""));
    assertTrue(template.contains("value=\"${messagingConsentVersion}:${option}\""));
    assertFalse(template.contains("sequent.notice-channel"));
  }

  private static String poll(String state, String link) throws Exception {
    Configuration configuration = new Configuration(Configuration.VERSION_2_3_32);
    configuration.setOutputFormat(HTMLOutputFormat.INSTANCE);
    configuration.setAutoEscapingPolicy(Configuration.ENABLE_IF_SUPPORTED_AUTO_ESCAPING_POLICY);
    configuration.setTemplateLoader(new FileTemplateLoader(TEMPLATES.toFile()));
    Map<String, Object> model = new HashMap<>();
    model.put("url", Map.of("loginAction", "https://keycloak.test/action?a=1&b=2"));
    model.put("ttl", "300");
    if (state != null) {
      model.put("messengerState", state);
    }
    if (link != null) {
      model.put("messengerLink", link);
    }
    StringWriter out = new StringWriter();
    configuration.getTemplate("messenger-status-poll.ftl").process(model, out);
    return out.toString();
  }

  @Test
  void theMessengerCheckIsLightAndBoundedByTheCodeLifetime() throws Exception {
    String page = poll("PENDING", "https://m.me/synthetic?ref=\"ref-1");
    assertTrue(page.contains("action=\"https://keycloak.test/action?a=1&amp;b=2\""));
    assertTrue(page.contains("name=\"messengerStatus\" value=\"true\""));
    assertTrue(page.contains("var state = \"PENDING\";"));
    assertTrue(page.contains("var lifetimeMs = 300 * 1000;"));
    assertTrue(page.contains("var intervalMs = 5000;"));
    assertTrue(page.contains("if (state !== \"PENDING\" || !link)"));
    assertTrue(page.contains("Date.now() - started > lifetimeMs"));
    assertTrue(page.contains("document.hidden"));
    assertFalse(page.contains("ref=\"ref-1"));

    assertTrue(poll(null, null).contains("var state = \"\";"));
  }
}
