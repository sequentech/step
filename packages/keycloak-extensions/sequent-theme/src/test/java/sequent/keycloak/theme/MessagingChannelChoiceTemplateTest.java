// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.theme;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import freemarker.cache.FileTemplateLoader;
import freemarker.cache.MultiTemplateLoader;
import freemarker.cache.StringTemplateLoader;
import freemarker.cache.TemplateLoader;
import freemarker.core.HTMLOutputFormat;
import freemarker.template.Configuration;
import freemarker.template.TemplateMethodModelEx;
import java.io.IOException;
import java.io.StringWriter;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;
import org.junit.jupiter.api.Test;

class MessagingChannelChoiceTemplateTest {
  private static final Path LOGIN = Path.of("src/main/resources/theme/sequent.admin-portal/login");

  private static String render(Map<String, Object> model) throws Exception {
    StringTemplateLoader page = new StringTemplateLoader();
    page.putTemplate(
        "page.ftl", "<#import \"messaging-channel-choice.ftl\" as choice><@choice.render/>");
    Configuration configuration = new Configuration(Configuration.VERSION_2_3_34);
    configuration.setOutputFormat(HTMLOutputFormat.INSTANCE);
    configuration.setAutoEscapingPolicy(Configuration.ENABLE_IF_SUPPORTED_AUTO_ESCAPING_POLICY);
    configuration.setTemplateLoader(
        new MultiTemplateLoader(
            new TemplateLoader[] {page, new FileTemplateLoader(LOGIN.toFile())}));
    StringWriter out = new StringWriter();
    configuration.getTemplate("page.ftl").process(model, out);
    return out.toString();
  }

  private static Map<String, Object> model(Map<String, String> formData, String errorField) {
    TemplateMethodModelEx msg =
        arguments ->
            arguments.stream().map(Object::toString).collect(Collectors.joining("|", "[", "]"));
    TemplateMethodModelEx exists = arguments -> arguments.get(0).toString().equals(errorField);
    TemplateMethodModelEx get = arguments -> "error-for-" + arguments.get(0);
    TemplateMethodModelEx sanitize = arguments -> arguments.get(0).toString();
    Map<String, Object> model = new HashMap<>();
    model.put("messagingChannels", List.of("EMAIL", "WHATSAPP", "MESSENGER"));
    model.put("messagingOrganization", "<b>Commission</b>");
    model.put("messagingConsentVersion", "3");
    model.put("messagingPage", "Synthetic Page");
    model.put("msg", msg);
    model.put("kcSanitize", sanitize);
    model.put("messagesPerField", Map.of("existsError", exists, "get", get));
    model.put("register", Map.of("formData", formData));
    model.put("properties", Map.of());
    model.put("url", Map.of("resourcesPath", "/resources"));
    return model;
  }

  @Test
  void offersOnlyThePostsChannelsWithTheirNumberConsentAndNoticeChoice() throws Exception {
    String html = render(model(Map.of("sequent.otp-channel", "WHATSAPP"), ""));

    assertTrue(html.contains("name=\"sequent.otp-channel\" value=\"EMAIL\""));
    assertTrue(html.contains("name=\"sequent.otp-channel\" value=\"WHATSAPP\""));
    assertFalse(html.contains("value=\"VIBER\""));
    assertTrue(
        html.contains(
            "type=\"tel\" id=\"sequent.whatsapp-number\" name=\"sequent.whatsapp-number\""));
    assertTrue(html.contains("value=\"3:WHATSAPP\""));
    assertTrue(html.contains("value=\"3:MESSENGER\""));
    assertTrue(html.contains("name=\"sequent.notice-channel\" value=\"EMAIL\""));
    assertTrue(
        html.contains(
            "[messaging.consent|&lt;b&gt;Commission&lt;/b&gt;|[messageChannel.WHATSAPP]]"));
    assertTrue(html.contains("[messageOtp.messenger.connectHelp|Synthetic Page]"));
    assertTrue(html.contains("src=\"/resources/js/messaging-channel-choice.js\""));
    assertEquals(2, html.split("\\bhidden>", -1).length - 1);
    assertFalse(html.contains("<b>Commission</b>"));
  }

  @Test
  void validationErrorsAreShownOnTheirFields() throws Exception {
    String html = render(model(Map.of(), "sequent.otp-channel"));
    assertTrue(html.contains("id=\"input-error-otp-channel\""));
    assertTrue(html.contains("aria-invalid=\"true\" aria-describedby=\"input-error-otp-channel\""));
  }

  @Test
  void nothingIsRenderedWithoutChannels() throws Exception {
    Map<String, Object> model = model(Map.of(), "");
    model.remove("messagingChannels");
    assertEquals("", render(model).trim());
  }

  @Test
  void registerIncludesTheChoiceBeforeTheTerms() throws IOException {
    String register = Files.readString(LOGIN.resolve("register.ftl"));
    assertTrue(
        register.contains("<#import \"messaging-channel-choice.ftl\" as messagingChannelChoice>"));
    assertTrue(
        register.indexOf("<@messagingChannelChoice.render/>")
            < register.indexOf("<@registerCommons.termsAcceptance/>"));
  }
}
