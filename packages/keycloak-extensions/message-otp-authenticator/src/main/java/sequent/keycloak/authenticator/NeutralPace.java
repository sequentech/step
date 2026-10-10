// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import java.util.Map;
import java.util.function.LongSupplier;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.provider.ProviderConfigProperty;

/**
 * The pace of the pages that must not tell whether an account exists (R7-06): the neutral reset
 * code page, the temporary password and the reset link. Each answers no sooner than a fixed time
 * (its floor) after the request began, with or without an account: an account's send (a call to
 * harvest, SMTP or the SMS provider) happens within that time, and no account waits the same. Only
 * a send that outlasts the floor can still be told apart by timing; such answers are logged so the
 * floor can be raised.
 *
 * <p>The floor is {@value #CONFIG} in the step's configuration, {@value #DEFAULT_MILLIS} ms by
 * default, at most {@value #MAX_MILLIS} ms; 0 turns pacing off.
 */
@JBossLog
public final class NeutralPace {
  /** Configuration key of the floor, in milliseconds. */
  public static final String CONFIG = "neutral-response-ms";

  public static final long DEFAULT_MILLIS = 3000;
  public static final long MAX_MILLIS = 15000;

  /** Waits; a seam for tests. */
  @FunctionalInterface
  interface Sleeper {
    void sleep(long millis) throws InterruptedException;
  }

  /** Nanoseconds; a seam for tests. */
  static volatile LongSupplier clock = System::nanoTime;

  static volatile Sleeper sleeper = Thread::sleep;

  private NeutralPace() {}

  /** The floor, for a step's configuration. */
  public static ProviderConfigProperty configProperty() {
    return new ProviderConfigProperty(
        CONFIG,
        "Neutral answer time (ms)",
        "A page that must not tell whether an account exists answers no sooner than this many"
            + " milliseconds after the request began, with or without an account, so its timing"
            + " doesn't tell either. Set it above the usual send time; an answer that takes longer"
            + " is logged. Default "
            + DEFAULT_MILLIS
            + ", at most "
            + MAX_MILLIS
            + "; 0 turns it off.",
        ProviderConfigProperty.STRING_TYPE,
        String.valueOf(DEFAULT_MILLIS));
  }

  /** When the request began. */
  public static long start() {
    return clock.getAsLong();
  }

  /** The floor of a step's configuration; the default when absent or not a number. */
  public static long floor(Map<String, String> config) {
    String value = config == null ? null : config.get(CONFIG);
    if (value == null || value.isBlank()) {
      return DEFAULT_MILLIS;
    }
    try {
      return Math.max(0, Math.min(MAX_MILLIS, Long.parseLong(value.trim())));
    } catch (NumberFormatException e) {
      log.warnv("{0} is not a number of milliseconds; using {1}", CONFIG, DEFAULT_MILLIS);
      return DEFAULT_MILLIS;
    }
  }

  /** Holds the answer until {@code floorMillis} after {@code start}. */
  public static void hold(long start, long floorMillis) {
    if (floorMillis <= 0) {
      return;
    }
    long elapsed = (clock.getAsLong() - start) / 1_000_000L;
    long left = floorMillis - elapsed;
    if (left <= 0) {
      log.infov(
          "a neutral page answered {0} ms after its {1} ms floor; raise {2} if this is frequent",
          -left, floorMillis, CONFIG);
      return;
    }
    try {
      sleeper.sleep(left);
    } catch (InterruptedException e) {
      Thread.currentThread().interrupt();
    }
  }
}
