// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyList;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import java.time.Clock;
import java.time.Instant;
import java.time.ZoneOffset;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.keycloak.authentication.ValidationContext;
import org.keycloak.events.Errors;
import org.keycloak.events.EventBuilder;
import org.keycloak.http.HttpRequest;
import org.keycloak.models.RealmModel;
import org.keycloak.models.utils.FormMessage;
import org.mockito.ArgumentCaptor;

class EnrollmentWindowCheckTest {

  /** A configuration: one Post, its zone, the primary zone and the instants windmill wrote. */
  record Config(String post, String zone, String primary, Instant opens, Instant closes) {
    String attribute() {
      return "{\""
          + post
          + "\":{\"election_id\":\"e1\",\"opens_at\":\""
          + opens
          + "\",\"closes_at\":\""
          + closes
          + "\",\"time_zone\":\""
          + zone
          + "\",\"close_time_zone\":\""
          + primary
          + "\"},\"Other Post\":{\"election_id\":\"e2\",\"opens_at\":null,"
          + "\"closes_at\":null,\"time_zone\":\"UTC\",\"close_time_zone\":\"UTC\"},"
          + "\"Ambiguous Post\":{\"problem\":\"ambiguous-post\"},"
          + "\"Broken Post\":{\"election_id\":\"e3\",\"opens_at\":\"yesterday\","
          + "\"closes_at\":null,\"time_zone\":\"UTC\"}}";
    }
  }

  /** COMELEC preset: Dubai opens at local midnight, every Post closes at 18:00 PhST. */
  private static final Config COMELEC =
      new Config(
          "Dubai PCG",
          "Asia/Dubai",
          "Asia/Manila",
          Instant.parse("2028-02-08T20:00:00Z"),
          Instant.parse("2028-05-08T10:00:00Z"));

  /** Madrid association: the Canary office opens at 09:00 local, closes at 20:00 Madrid. */
  private static final Config MADRID =
      new Config(
          "Canary office",
          "Atlantic/Canary",
          "Europe/Madrid",
          Instant.parse("2028-03-01T09:00:00Z"),
          Instant.parse("2028-03-31T18:00:00Z"));

  @Test
  void refusesBeforeTheWindowAndAfterItAndAcceptsInside() {
    for (Config config : List.of(COMELEC, MADRID)) {
      assertRefused(config, config.post(), config.opens().minusSeconds(1));
      assertAccepted(config, config.post(), config.opens());
      assertAccepted(config, config.post(), config.closes().minusSeconds(1));
      assertRefused(config, config.post(), config.closes());
      assertRefused(config, config.post(), config.closes().plusSeconds(3600));
    }
  }

  @Test
  void aPostWithoutAWindowOrAnUnboundedOneIsAccepted() {
    for (Config config : List.of(COMELEC, MADRID)) {
      Instant before = config.opens().minusSeconds(86400);
      assertAccepted(config, "Unknown Post", before);
      assertAccepted(config, "Other Post", before);
      assertAccepted(config, null, before);
    }
  }

  @Test
  void aRealmWithoutTheAttributeAcceptsEveryPost() {
    TestContext test = context(null, COMELEC.post());
    new EnrollmentWindowCheck(clockAt(Instant.EPOCH)).validate(test.context);
    verify(test.context).success();
    verify(test.context, never()).validationError(any(), anyList());
  }

  @Test
  void aPostThatCantBeCheckedIsRefused() {
    for (Config config : List.of(COMELEC, MADRID)) {
      Instant inside = config.opens();
      // Windmill couldn't map the option to one Post.
      assertRefused(
          config.attribute(), "Ambiguous Post", inside, EnrollmentWindows.NOT_CONFIGURED_MESSAGE);
      // An entry this extension can't read.
      assertRefused(
          config.attribute(), "Broken Post", inside, EnrollmentWindows.NOT_CONFIGURED_MESSAGE);
    }
  }

  @Test
  void anUnreadableAttributeRefusesEveryPost() {
    for (String attribute : List.of("not json", "[]", "\"text\"")) {
      assertFalse(EnrollmentWindows.parse(attribute).isEmpty());
      assertRefused(
          attribute, COMELEC.post(), COMELEC.opens(), EnrollmentWindows.NOT_CONFIGURED_MESSAGE);
      assertRefused(
          attribute, "Any Post", COMELEC.opens(), EnrollmentWindows.NOT_CONFIGURED_MESSAGE);
    }
    assertTrue(EnrollmentWindows.parse(null).isEmpty());
    assertTrue(EnrollmentWindows.parse(" ").isEmpty());
  }

  @Test
  void aWindowWrittenBeforeTheCloseZoneClosesInThePostZone() {
    EnrollmentWindows.Window window =
        EnrollmentWindows.parse(
                "{\"A\":{\"election_id\":\"e1\",\"opens_at\":null,"
                    + "\"closes_at\":null,\"time_zone\":\"Asia/Dubai\"}}")
            .lookup("A");
    assertEquals("Asia/Dubai", window.closeZone().getId());
    assertEquals(EnrollmentWindows.State.OPEN, window.stateAt(Instant.EPOCH));
  }

  @Test
  void theStateFollowsTheWindow() {
    EnrollmentWindows.Window window =
        EnrollmentWindows.parse(COMELEC.attribute()).lookup(COMELEC.post());
    assertEquals(EnrollmentWindows.State.BEFORE, window.stateAt(COMELEC.opens().minusMillis(1)));
    assertEquals(EnrollmentWindows.State.OPEN, window.stateAt(COMELEC.opens()));
    assertEquals(EnrollmentWindows.State.CLOSED, window.stateAt(COMELEC.closes()));
  }

  @Test
  void thePageShowsTheOpeningInThePostZoneAndTheCloseInThePrimary() {
    Map<String, String> dubai =
        pageEntry(COMELEC, COMELEC.opens().minusSeconds(60), Locale.ENGLISH);
    assertEquals("before", dubai.get("state"));
    assertEquals("Asia/Dubai", dubai.get("zone"));
    assertEquals("Asia/Manila", dubai.get("closeZone"));
    // 2028-02-08T20:00Z is midnight in Dubai (UTC+4); 10:00Z is 18:00 in Manila (UTC+8).
    assertTrue(dubai.get("opens").startsWith("Feb 9, 2028, 12:00"), dubai.get("opens"));
    assertTrue(dubai.get("closes").startsWith("May 8, 2028, 6:00"), dubai.get("closes"));
    assertEquals("Gulf Standard Time", dubai.get("opensZoneName"));
    assertEquals("Philippine Standard Time", dubai.get("closesZoneName"));

    Map<String, String> canary = pageEntry(MADRID, MADRID.opens(), Locale.ENGLISH);
    assertEquals("open", canary.get("state"));
    assertTrue(canary.get("opens").startsWith("Mar 1, 2028, 9:00"), canary.get("opens"));
    // Summer time has started by the close: 18:00Z is 20:00 in Madrid.
    assertTrue(canary.get("closes").startsWith("Mar 31, 2028, 8:00"), canary.get("closes"));
    assertEquals("Western European Standard Time", canary.get("opensZoneName"));
    assertEquals("Central European Summer Time", canary.get("closesZoneName"));

    Map<String, String> ambiguous =
        EnrollmentWindowCheck.pageModel(
                EnrollmentWindows.parse(MADRID.attribute()).byPost(),
                MADRID.opens(),
                Locale.ENGLISH)
            .stream()
            .filter(entry -> entry.get("embassy").equals("Ambiguous Post"))
            .findFirst()
            .orElseThrow();
    assertEquals("not-configured", ambiguous.get("state"));

    Map<String, String> closed = pageEntry(MADRID, MADRID.closes(), Locale.ENGLISH);
    assertEquals("closed", closed.get("state"));
  }

  @Test
  void anUnboundedSideHasNoTime() {
    List<Map<String, String>> page =
        EnrollmentWindowCheck.pageModel(
            EnrollmentWindows.parse(COMELEC.attribute()).byPost(), Instant.EPOCH, Locale.ENGLISH);
    Map<String, String> other =
        page.stream().filter(e -> e.get("embassy").equals("Other Post")).findFirst().get();
    assertEquals("open", other.get("state"));
    assertNull(other.get("opens"));
    assertNull(other.get("closes"));
  }

  private static Map<String, String> pageEntry(Config config, Instant now, Locale locale) {
    return EnrollmentWindowCheck.pageModel(
            EnrollmentWindows.parse(config.attribute()).byPost(), now, locale)
        .stream()
        .filter(entry -> entry.get("embassy").equals(config.post()))
        .findFirst()
        .orElseThrow();
  }

  private static void assertAccepted(Config config, String post, Instant now) {
    TestContext test = context(config.attribute(), post);
    new EnrollmentWindowCheck(clockAt(now)).validate(test.context);
    verify(test.context).success();
    verify(test.context, never()).validationError(any(), anyList());
  }

  private static void assertRefused(Config config, String post, Instant now) {
    assertRefused(config.attribute(), post, now, EnrollmentWindows.NOT_OPEN_MESSAGE);
  }

  @SuppressWarnings("unchecked")
  private static void assertRefused(String attribute, String post, Instant now, String message) {
    TestContext test = context(attribute, post);
    new EnrollmentWindowCheck(clockAt(now)).validate(test.context);
    verify(test.context, never()).success();
    verify(test.context).error(Errors.INVALID_REGISTRATION);
    ArgumentCaptor<List<FormMessage>> errors = ArgumentCaptor.forClass(List.class);
    verify(test.context).validationError(eq(test.formData), errors.capture());
    assertEquals(1, errors.getValue().size());
    FormMessage error = errors.getValue().get(0);
    assertEquals(EnrollmentWindows.POST_FIELD, error.getField());
    assertEquals(message, error.getMessage());
    assertEquals(post, error.getParameters()[0]);
    assertFalse(error.getParameters().length > 1);
  }

  private record TestContext(ValidationContext context, MultivaluedMap<String, String> formData) {}

  private static TestContext context(String attribute, String post) {
    ValidationContext context = mock(ValidationContext.class);
    RealmModel realm = mock(RealmModel.class);
    HttpRequest request = mock(HttpRequest.class);
    EventBuilder event = mock(EventBuilder.class);
    MultivaluedMap<String, String> formData = new MultivaluedHashMap<>();
    if (post != null) {
      formData.putSingle(EnrollmentWindows.POST_FIELD, post);
    }
    when(context.getRealm()).thenReturn(realm);
    when(realm.getAttribute(EnrollmentWindows.REALM_ATTRIBUTE)).thenReturn(attribute);
    when(context.getHttpRequest()).thenReturn(request);
    when(request.getDecodedFormParameters()).thenReturn(formData);
    when(context.getEvent()).thenReturn(event);
    when(event.detail(anyString(), anyString())).thenReturn(event);
    return new TestContext(context, formData);
  }

  private static Clock clockAt(Instant now) {
    return Clock.fixed(now, ZoneOffset.UTC);
  }
}
