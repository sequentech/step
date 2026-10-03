// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

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
  void theCodePageParsesAndOffersTheChannelControls() throws IOException {
    String template = parsed(TEMPLATE);
    assertTrue(template.contains("name=\"channel\" value=\"${option}\""));
    assertTrue(template.contains("name=\"messengerStatus\" value=\"true\""));
    assertTrue(template.contains("msg(\"messageOtp.delivery.unknown\")"));
    assertTrue(template.contains("msg(\"messageOtp.otherWay.title\")"));
  }

  @Test
  void theCodePagesCheckMessengerAgainByThemselves() throws IOException {
    assertTrue(parsed(TEMPLATE).contains("<#include \"messenger-status-poll.ftl\">"));
    String contactCode = parsed(BASE_LOGIN.resolve("message-otp.enter-otp.ftl"));
    assertTrue(contactCode.contains("<#include \"messenger-status-poll.ftl\">"));
    assertTrue(contactCode.contains("name=\"messengerStatus\" value=\"true\""));
  }

  @Test
  void aFailedSendDoesNotStartTheCountdownOnTheCodePages() throws IOException {
    for (Path page : new Path[] {TEMPLATE, BASE_LOGIN.resolve("message-otp.enter-otp.ftl")}) {
      String template = parsed(page);
      assertTrue(template.contains("(deliveryState!'') == 'FAILED'"), page.toString());
      assertTrue(template.contains("if (sendFailed === \"true\")"), page.toString());
    }
  }

  @Test
  void theMessagingAppFormAsksForTheAppItsContactAndConsent() throws IOException {
    String template = parsed(BASE_LOGIN.resolve("message-otp.enter-app-contact.ftl"));
    assertTrue(template.contains("name=\"channel\""));
    assertTrue(template.contains("name=\"contact\""));
    assertTrue(template.contains("name=\"sequent.message-consent\""));
    assertTrue(template.contains("value=\"${messagingConsentVersion}:${option}\""));
    assertTrue(template.contains("name=\"sequent.notice-channel\""));
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
