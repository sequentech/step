// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.authenticator;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

class ResendTimerTest {
  private static final long NOW = 1_800_000_000_000L;
  private static final String CODE_TTL_SECONDS = "300";
  private static final String RESEND_TIMER_SECONDS = "60";

  private static String expiryForCodeSentSecondsAgo(long seconds) {
    return Long.toString(NOW + 300_000L - seconds * 1000L);
  }

  @Test
  void resendIsRefusedWithinTheResendTimer() {
    assertFalse(
        Utils.isResendAllowed(
            expiryForCodeSentSecondsAgo(1), CODE_TTL_SECONDS, RESEND_TIMER_SECONDS, NOW));
    assertFalse(
        Utils.isResendAllowed(
            expiryForCodeSentSecondsAgo(59), CODE_TTL_SECONDS, RESEND_TIMER_SECONDS, NOW));
  }

  @Test
  void resendIsAllowedOnceTheResendTimerHasElapsed() {
    assertTrue(
        Utils.isResendAllowed(
            expiryForCodeSentSecondsAgo(61), CODE_TTL_SECONDS, RESEND_TIMER_SECONDS, NOW));
  }

  @Test
  void resendIsRefusedWhenTheTimingIsUnknown() {
    assertFalse(Utils.isResendAllowed(null, CODE_TTL_SECONDS, RESEND_TIMER_SECONDS, NOW));
    assertFalse(
        Utils.isResendAllowed(expiryForCodeSentSecondsAgo(61), null, RESEND_TIMER_SECONDS, NOW));
    assertFalse(
        Utils.isResendAllowed(expiryForCodeSentSecondsAgo(61), CODE_TTL_SECONDS, null, NOW));
  }
}
