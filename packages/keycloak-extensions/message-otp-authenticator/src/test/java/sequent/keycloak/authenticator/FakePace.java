// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import java.util.ArrayList;
import java.util.List;

/**
 * A fake clock for {@link NeutralPace}: sends move it on with {@link #advance}, and every hold is
 * recorded instead of slept. Closing it restores the real clock, still without sleeping ({@link
 * NoNeutralPaceSleep}).
 */
public final class FakePace implements AutoCloseable {
  private long nowMillis;
  private final List<Long> held = new ArrayList<>();

  public FakePace() {
    NeutralPace.clock = () -> nowMillis * 1_000_000L;
    NeutralPace.sleeper =
        millis -> {
          held.add(millis);
          nowMillis += millis;
        };
  }

  public long now() {
    return nowMillis;
  }

  public void advance(long millis) {
    nowMillis += millis;
  }

  /** The holds so far, in milliseconds. */
  public List<Long> held() {
    return List.copyOf(held);
  }

  @Override
  public void close() {
    NeutralPace.clock = System::nanoTime;
    NeutralPace.sleeper = millis -> {};
  }
}
