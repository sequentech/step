// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import freemarker.cache.FileTemplateLoader;
import freemarker.cache.MultiTemplateLoader;
import freemarker.cache.StringTemplateLoader;
import freemarker.cache.TemplateLoader;
import freemarker.core.HTMLOutputFormat;
import freemarker.template.Configuration;
import freemarker.template.TemplateMethodModelEx;
import java.io.File;
import java.io.StringWriter;
import java.text.MessageFormat;
import java.time.Instant;
import java.util.HashMap;
import java.util.Locale;
import java.util.Map;
import org.junit.jupiter.api.Test;

class EnrollmentReplyByTest {

  /** The COMELEC preset's Dubai Post and the Madrid association's Canary office. */
  private static final String WINDOWS =
      "{\"Dubai PCG\":{\"election_id\":\"e1\",\"opens_at\":\"2028-02-08T20:00:00Z\","
          + "\"closes_at\":\"2028-05-08T10:00:00Z\",\"time_zone\":\"Asia/Dubai\"},"
          + "\"Canary office\":{\"election_id\":\"e2\",\"opens_at\":\"2028-03-01T09:00:00Z\","
          + "\"closes_at\":\"2028-03-31T18:00:00Z\",\"time_zone\":\"Atlantic/Canary\"}}";

  private static final Instant SUBMITTED = Instant.parse("2028-03-02T06:20:00Z");

  @Test
  void theReplyByIsTheSubmissionPlusTheConfiguredHoursAndIsKept() {
    Instant replyBy = EnrollmentWindows.replyBy(null, "72", SUBMITTED);
    assertEquals(Instant.parse("2028-03-05T06:20:00Z"), replyBy);
    // A later render of the same enrollment keeps the stored time.
    assertEquals(
        replyBy, EnrollmentWindows.replyBy(replyBy.toString(), "72", SUBMITTED.plusSeconds(600)));
  }

  @Test
  void noOrABadConfigurationShowsNoReplyBy() {
    assertNull(EnrollmentWindows.replyBy(null, null, SUBMITTED));
    assertNull(EnrollmentWindows.replyBy(null, " ", SUBMITTED));
    assertNull(EnrollmentWindows.replyBy(null, "three days", SUBMITTED));
    assertNull(EnrollmentWindows.replyBy(null, "0", SUBMITTED));
  }

  @Test
  void unrepresentablePositiveHoursShowNoReplyByInsteadOfWrappingOrCrashing() {
    for (String hours : new String[] {Long.toString(Long.MAX_VALUE), "3000000000000000"}) {
      assertNull(EnrollmentWindows.replyBy(null, hours, SUBMITTED));
    }
    assertNull(EnrollmentWindows.replyBy(null, "1", Instant.MAX.minusSeconds(60)));
  }

  @Test
  void theReplyByShowsInThePostZone() {
    EnrollmentWindows.Windows windows = EnrollmentWindows.parse(WINDOWS);
    Instant replyBy = Instant.parse("2028-03-05T06:20:00Z");

    Map<String, String> dubai =
        EnrollmentWindows.replyByPage(
            replyBy, windows.lookup("Dubai PCG"), "Dubai PCG", Locale.ENGLISH);
    assertTrue(dubai.get("dateTime").startsWith("Mar 5, 2028, 10:20"), dubai.get("dateTime"));
    assertEquals("Asia/Dubai", dubai.get("zone"));
    assertEquals("Gulf Standard Time", dubai.get("zoneName"));

    Map<String, String> canary =
        EnrollmentWindows.replyByPage(
            replyBy, windows.lookup("Canary office"), "Canary office", Locale.ENGLISH);
    assertTrue(canary.get("dateTime").startsWith("Mar 5, 2028, 6:20"), canary.get("dateTime"));
    assertEquals("Western European Standard Time", canary.get("zoneName"));

    Map<String, String> unknown =
        EnrollmentWindows.replyByPage(replyBy, null, "Oslo PE", Locale.ENGLISH);
    assertEquals("UTC", unknown.get("zone"));
    assertEquals("Coordinated Universal Time", unknown.get("zoneName"));
  }

  @Test
  void theReviewPageShowsTheReplyByWithTheZoneName() throws Exception {
    Map<String, String> messages =
        Map.of(
            "enrollment.replyBy", "Reply by {0}",
            "timezones.voterDateTimeZone", "{0} {1}");
    Map<String, Object> model = model(messages);
    model.put(
        LookupAndUpdateUser.REPLY_BY_PAGE_ATTRIBUTE,
        Map.of(
            "dateTime",
            "Mar 5, 2028, 10:20 AM",
            "zone",
            "Asia/Dubai",
            "zoneName",
            "Gulf Standard Time"));
    assertTrue(
        render(model)
            .contains(
                "<p id=\"enrollment-reply-by\" class=\"enrollment-reply-by\">"
                    + "Reply by Mar 5, 2028, 10:20 AM Gulf Standard Time</p>"));

    Map<String, String> overridden = new HashMap<>(messages);
    overridden.put("timezones.name.Asia/Dubai", "Dubai time");
    Map<String, Object> overriddenModel = model(overridden);
    overriddenModel.put(
        LookupAndUpdateUser.REPLY_BY_PAGE_ATTRIBUTE, model.get("enrollmentReplyBy"));
    assertTrue(render(overriddenModel).contains("Reply by Mar 5, 2028, 10:20 AM Dubai time</p>"));

    assertFalse(render(model(messages)).contains("enrollment-reply-by"));
  }

  @Test
  void brokenRealmTextCannotHideTheReplyByTimeOrZone() throws Exception {
    Map<String, Object> model =
        model(
            Map.of(
                "enrollment.replyBy", "Reply by {0}",
                "timezones.voterDateTimeZone", "{0}",
                "timezones.defaultVoterDateTimeZone", "{0} {1}"));
    model.put(
        "enrollmentReplyBy",
        Map.of(
            "dateTime",
            "Mar 5, 2028, 10:20 AM",
            "zone",
            "Asia/Dubai",
            "zoneName",
            "Gulf Standard Time"));
    assertTrue(render(model).contains("Reply by Mar 5, 2028, 10:20 AM Gulf Standard Time</p>"));
  }

  @Test
  void malformedRealmMessageUsesTheDefaultBeforeFreeMarkerCallsMessageFormat() throws Exception {
    for (String pattern : java.util.List.of("At {0", "{{dateTime}} {{zoneName}}", "{0}", "{1}")) {
      org.keycloak.models.RealmModel realm =
          org.mockito.Mockito.mock(org.keycloak.models.RealmModel.class);
      org.mockito.Mockito.when(realm.getRealmLocalizationTexts())
          .thenReturn(Map.of("en", Map.of("timezones.voterDateTimeZone", pattern)));
      String key = EnrollmentWindows.dateTimeZoneMessageKey(realm, Locale.ENGLISH);
      assertEquals("timezones.defaultVoterDateTimeZone", key);
      Map<String, Object> model =
          model(
              Map.of(
                  "enrollment.replyBy", "Reply by {0}",
                  "timezones.voterDateTimeZone", pattern,
                  "timezones.defaultVoterDateTimeZone", "{0} {1}"));
      model.put("enrollmentTimezoneMessageKey", key);
      model.put(
          "enrollmentReplyBy",
          Map.of(
              "dateTime",
              "Mar 5, 2028, 10:20 AM",
              "zone",
              "Asia/Dubai",
              "zoneName",
              "Gulf Standard Time"));
      assertTrue(render(model).contains("Reply by Mar 5, 2028, 10:20 AM Gulf Standard Time</p>"));
    }
  }

  @Test
  void validRegionalTimezoneTextKeepsItsOverride() {
    org.keycloak.models.RealmModel realm =
        org.mockito.Mockito.mock(org.keycloak.models.RealmModel.class);
    org.mockito.Mockito.when(realm.getDefaultLocale()).thenReturn("en");
    org.mockito.Mockito.when(realm.getRealmLocalizationTexts())
        .thenReturn(
            Map.of(
                "en", Map.of("timezones.voterDateTimeZone", "{0} {1}"),
                "es", Map.of("timezones.voterDateTimeZone", "{0}"),
                "es-MX", Map.of("timezones.voterDateTimeZone", "{1}: {0}")));
    assertEquals(
        "timezones.voterDateTimeZone",
        EnrollmentWindows.dateTimeZoneMessageKey(realm, Locale.forLanguageTag("es-MX")));
    assertEquals(
        "timezones.defaultVoterDateTimeZone",
        EnrollmentWindows.dateTimeZoneMessageKey(realm, Locale.forLanguageTag("es")));
  }

  private static Map<String, Object> model(Map<String, String> messages) {
    TemplateMethodModelEx msg =
        arguments -> {
          String key = arguments.get(0).toString();
          String text = messages.get(key);
          if (text == null) {
            return key;
          }
          Object[] parameters =
              arguments.subList(1, arguments.size()).stream().map(Object::toString).toArray();
          return new MessageFormat(text, Locale.ENGLISH).format(parameters);
        };
    Map<String, Object> model = new HashMap<>();
    model.put("msg", msg);
    model.put("url", Map.of("loginRestartFlowUrl", "/restart"));
    return model;
  }

  private static String render(Map<String, Object> model) throws Exception {
    StringTemplateLoader layout = new StringTemplateLoader();
    layout.putTemplate(
        "template.ftl",
        "<#macro registrationLayout displayMessage=true><#nested \"form\"></#macro>");
    Configuration configuration = new Configuration(Configuration.VERSION_2_3_34);
    configuration.setOutputFormat(HTMLOutputFormat.INSTANCE);
    configuration.setTemplateLoader(
        new MultiTemplateLoader(
            new TemplateLoader[] {
              new FileTemplateLoader(new File("src/main/resources/theme-resources/templates")),
              layout
            }));
    StringWriter rendered = new StringWriter();
    configuration.getTemplate("registration-manual-finish.ftl").process(model, rendered);
    return rendered.toString();
  }
}
