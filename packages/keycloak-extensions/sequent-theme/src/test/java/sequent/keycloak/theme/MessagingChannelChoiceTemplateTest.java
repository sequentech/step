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
  void offersOnlyThePostsChannelsWithTheirNumberAndConsentAndNoNoticeChoice() throws Exception {
    String html = render(model(Map.of("sequent.otp-channel", "WHATSAPP"), ""));

    assertTrue(html.contains("name=\"sequent.otp-channel\" value=\"EMAIL\""));
    assertTrue(html.contains("name=\"sequent.otp-channel\" value=\"WHATSAPP\""));
    assertFalse(html.contains("value=\"VIBER\""));
    assertTrue(
        html.contains(
            "type=\"tel\" id=\"sequent.whatsapp-number\" name=\"sequent.whatsapp-number\""));
    assertTrue(html.contains("value=\"3:WHATSAPP\""));
    assertTrue(html.contains("value=\"3:MESSENGER\""));
    assertFalse(html.contains("sequent.notice-channel"));
    assertTrue(
        html.contains(
            "[messaging.consent|&lt;b&gt;Commission&lt;/b&gt;|[messageChannel.WHATSAPP]]"));
    assertTrue(html.contains("[messageOtp.messenger.connectHelp|Synthetic Page]"));
    assertTrue(html.contains("src=\"/resources/js/messaging-channel-choice.js\""));
    // The two channels not selected keep their details hidden, and so does the Post help.
    assertEquals(3, html.split("\\bhidden>", -1).length - 1);
    assertFalse(html.contains("<b>Commission</b>"));
  }

  /** KC-R5 and plan §2.5: consent for every channel, email and SMS included. */
  /** R3-22: the Messenger hint has its "Connect Messenger" label, as the React form shows. */
  @Test
  void theMessengerHintIsLabelledConnectMessenger() throws Exception {
    String html = render(model(Map.of("sequent.otp-channel", "MESSENGER"), ""));
    int label = html.indexOf("[messageOtp.messenger.connect]");
    assertTrue(label >= 0, html);
    assertTrue(label < html.indexOf("[messageOtp.messenger.connectHelp|Synthetic Page]"));
  }

  @Test
  void everyChannelAsksForConsent() throws Exception {
    Map<String, Object> model = model(Map.of(), "");
    model.put("messagingChannels", List.of("SMS", "EMAIL"));
    String html = render(model);
    assertTrue(html.contains("value=\"3:SMS\""));
    assertTrue(html.contains("value=\"3:EMAIL\""));
  }

  /** KC-F9: the page carries each Post's channels and refreshes them when the Post changes. */
  @Test
  void thePageCarriesEachPostsChannels() throws Exception {
    Map<String, Object> model = model(Map.of(), "");
    model.put("messagingAllChannels", List.of("WHATSAPP", "VIBER", "MESSENGER", "EMAIL"));
    model.put("messagingPostAttribute", "embassy");
    model.put("messagingChannelsByPostJson", "{\"Madrid\":[\"VIBER\",\"EMAIL\"]}");
    String html = render(model);
    assertTrue(html.contains("data-post-attribute=\"embassy\""));
    assertTrue(
        html.contains(
            "data-channels-by-post=\"{&quot;Madrid&quot;:[&quot;VIBER&quot;,&quot;EMAIL&quot;]}\""));
    // Viber is rendered for the script, hidden until a Post that offers it is selected.
    assertTrue(html.contains("data-messaging-option=\"VIBER\"\n                    hidden"));
    assertTrue(html.contains("[messaging.channelChoice.help|__post__]"));
    String script = Files.readString(LOGIN.resolve("resources/js/messaging-channel-choice.js"));
    assertTrue(script.contains("field.addEventListener(\"change\", showPostChannels)"));
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
