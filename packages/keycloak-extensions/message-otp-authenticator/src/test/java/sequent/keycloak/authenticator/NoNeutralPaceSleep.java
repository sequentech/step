// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import org.junit.jupiter.api.extension.BeforeEachCallback;
import org.junit.jupiter.api.extension.ExtensionContext;

/**
 * Registered for every test of the module (auto-detected): a neutral page's {@link NeutralPace}
 * hold doesn't sleep in tests, which would add seconds to each. Tests of the pace itself install a
 * {@link FakePace}.
 */
public final class NoNeutralPaceSleep implements BeforeEachCallback {
  @Override
  public void beforeEach(ExtensionContext context) {
    NeutralPace.clock = System::nanoTime;
    NeutralPace.sleeper = millis -> {};
  }
}
