// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.time.DateTimeException;
import java.time.Instant;
import java.time.ZoneId;
import java.time.format.DateTimeFormatter;
import java.time.format.FormatStyle;
import java.util.Collections;
import java.util.Iterator;
import java.util.LinkedHashMap;
import java.util.Locale;
import java.util.Map;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.models.RealmModel;

/**
 * Per-Post enrollment windows (VOTE-LIFECYCLE §8). Windmill writes the event realm attribute
 * {@value #REALM_ATTRIBUTE}: a JSON object keyed by the {@value #POST_FIELD} user-profile option,
 * each value either a window {@code {election_id, opens_at, closes_at, time_zone, close_time_zone}}
 * with RFC 3339 instants ({@code null} = that side is not limited), or {@code {problem}} for an
 * option windmill couldn't map to one Post.
 *
 * <p>Nothing passes unchecked: a problem entry, an entry that can't be read and an unreadable
 * attribute all refuse the registration ({@value #NOT_CONFIGURED_MESSAGE}), logged at ERROR.
 */
@JBossLog
public final class EnrollmentWindows {

  public static final String REALM_ATTRIBUTE = "enrollment_windows";
  public static final String POST_FIELD = "embassy";

  /** Field error when the chosen Post is not open: {@code {0}} = the Post. */
  public static final String NOT_OPEN_MESSAGE = "enrollment.postNotOpen";

  /** Field error when the chosen Post's window can't be checked: {@code {0}} = the Post. */
  public static final String NOT_CONFIGURED_MESSAGE = "enrollment.postNotConfigured";

  /** The problem of an entry this extension can't read. */
  public static final String UNREADABLE = "unreadable";

  private static final ObjectMapper MAPPER = new ObjectMapper();
  private static final ZoneId UTC = ZoneId.of("UTC");

  private EnrollmentWindows() {}

  /** Where a Post's enrollment stands at an instant. */
  public enum State {
    BEFORE,
    OPEN,
    CLOSED,
    /** The window can't be checked: registration is refused. */
    NOT_CONFIGURED;

    /** The value register.ftl and enrollment-window.js compare against. */
    public String wireValue() {
      return name().toLowerCase(Locale.ROOT).replace('_', '-');
    }
  }

  /**
   * One Post's window: the opening shows in the Post's {@code zone}, the common close in {@code
   * closeZone} (the event's primary zone). {@code problem} is set when the window can't be checked.
   */
  public record Window(
      String electionId,
      Instant opensAt,
      Instant closesAt,
      ZoneId zone,
      ZoneId closeZone,
      String problem) {

    static Window problem(String problem) {
      return new Window(null, null, null, null, null, problem);
    }

    /** Open from {@code opensAt} (inclusive) until {@code closesAt} (exclusive). */
    public State stateAt(Instant now) {
      if (problem != null) {
        return State.NOT_CONFIGURED;
      }
      if (opensAt != null && now.isBefore(opensAt)) {
        return State.BEFORE;
      }
      if (closesAt != null && !now.isBefore(closesAt)) {
        return State.CLOSED;
      }
      return State.OPEN;
    }
  }

  /**
   * The realm's windows. {@code unreadable} means the attribute is there but isn't a JSON object:
   * every Post is then refused.
   */
  public record Windows(Map<String, Window> byPost, boolean unreadable) {

    static final Windows NONE = new Windows(Collections.emptyMap(), false);

    /** Nothing to enforce: the realm has no windows. */
    public boolean isEmpty() {
      return byPost.isEmpty() && !unreadable;
    }

    /** The Post's window; {@code null} when the Post has none (nothing to enforce). */
    public Window lookup(String post) {
      if (unreadable) {
        return Window.problem(UNREADABLE);
      }
      return post == null ? null : byPost.get(post);
    }
  }

  /** The realm's windows. */
  public static Windows read(RealmModel realm) {
    return parse(realm.getAttribute(REALM_ATTRIBUTE));
  }

  /** Parses the attribute; see the class comment for what can't be read. */
  public static Windows parse(String json) {
    if (json == null || json.isBlank()) {
      return Windows.NONE;
    }
    JsonNode root;
    try {
      root = MAPPER.readTree(json);
    } catch (Exception e) {
      root = null;
    }
    if (root == null || !root.isObject()) {
      log.errorv(
          "Realm attribute {0} is not a JSON object: every registration is refused until the"
              + " enrollment windows are written again",
          REALM_ATTRIBUTE);
      return new Windows(Collections.emptyMap(), true);
    }
    Map<String, Window> windows = new LinkedHashMap<>();
    Iterator<Map.Entry<String, JsonNode>> fields = root.fields();
    while (fields.hasNext()) {
      Map.Entry<String, JsonNode> field = fields.next();
      windows.put(field.getKey(), entry(field.getKey(), field.getValue()));
    }
    return new Windows(windows, false);
  }

  private static Window entry(String post, JsonNode value) {
    String problem = text(value, "problem");
    if (problem != null) {
      log.errorv(
          "Enrollment for {0} is refused: windmill couldn''t map it to one Post ({1})",
          post, problem);
      return Window.problem(problem);
    }
    try {
      String electionId = text(value, "election_id");
      if (electionId == null) {
        throw new IllegalArgumentException("no election_id");
      }
      ZoneId zone = ZoneId.of(required(value, "time_zone"));
      String closeZone = text(value, "close_time_zone");
      return new Window(
          electionId,
          instant(value, "opens_at"),
          instant(value, "closes_at"),
          zone,
          // Windows written before close_time_zone existed close in the Post's zone.
          closeZone == null ? zone : ZoneId.of(closeZone),
          null);
    } catch (RuntimeException e) {
      log.errorv(e, "Enrollment window for {0} can''t be read: enrollment there is refused", post);
      return Window.problem(UNREADABLE);
    }
  }

  /** The instant as the voter reads it in the Post's zone: the locale's date and time. */
  public static String formatDateTime(Instant instant, ZoneId zone, Locale locale) {
    return DateTimeFormatter.ofLocalizedDateTime(FormatStyle.MEDIUM, FormatStyle.SHORT)
        .withLocale(locale)
        .format(instant.atZone(zone));
  }

  /**
   * The zone's long name at the instant (CLDR, as Intl's {@code timeZoneName: "long"}): the default
   * when the {@code timezones.name.<zone>} message has no text.
   */
  public static String zoneName(Instant instant, ZoneId zone, Locale locale) {
    return DateTimeFormatter.ofPattern("zzzz", locale).format(instant.atZone(zone));
  }

  /**
   * The reply-by instant of an application under review: the one already stored for this
   * enrollment, else {@code now} plus {@code replyByHours}. {@code null} when no reply-by time is
   * configured (blank), or the configuration isn't a positive whole number of hours (logged).
   */
  public static Instant replyBy(String stored, String replyByHours, Instant now) {
    if (stored != null && !stored.isBlank()) {
      try {
        return Instant.parse(stored);
      } catch (DateTimeException e) {
        log.warnv("Stored reply-by time {0} is unreadable: computing it again", stored);
      }
    }
    if (replyByHours == null || replyByHours.isBlank()) {
      return null;
    }
    long hours;
    try {
      hours = Long.parseLong(replyByHours.trim());
    } catch (NumberFormatException e) {
      hours = 0;
    }
    if (hours <= 0) {
      log.warnv(
          "Reply-by hours {0} is not a positive whole number: the review page shows no reply-by"
              + " time",
          replyByHours);
      return null;
    }
    try {
      return now.plusSeconds(Math.multiplyExact(hours, 3600));
    } catch (ArithmeticException | DateTimeException e) {
      log.warnv(
          "Reply-by hours {0} exceed the supported instant range: the review page shows no"
              + " reply-by time",
          replyByHours);
      return null;
    }
  }

  /**
   * What the review page shows for the reply-by time: {@code dateTime} in the Post's zone, {@code
   * zone} and the zone's default long name {@code zoneName}. A Post without a window shows UTC
   * (logged), named as such.
   */
  public static Map<String, String> replyByPage(
      Instant replyBy, Window window, String post, Locale locale) {
    ZoneId zone = UTC;
    if (window != null && window.zone() != null) {
      zone = window.zone();
    } else {
      log.infov("No enrollment window for {0}: the reply-by time shows in UTC", post);
    }
    return Map.of(
        "dateTime", formatDateTime(replyBy, zone, locale),
        "zone", zone.getId(),
        "zoneName", zoneName(replyBy, zone, locale));
  }

  private static String text(JsonNode node, String field) {
    JsonNode value = node.get(field);
    return value == null || value.isNull() ? null : value.asText();
  }

  private static Instant instant(JsonNode node, String field) {
    String value = text(node, field);
    return value == null ? null : Instant.parse(value);
  }

  private static String required(JsonNode node, String field) {
    String value = text(node, field);
    if (value == null) {
      throw new IllegalArgumentException("no " + field);
    }
    return value;
  }
}
