// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import static org.junit.jupiter.api.Assertions.*;

import jakarta.ws.rs.ForbiddenException;
import java.io.IOException;
import java.util.concurrent.atomic.AtomicInteger;
import org.junit.jupiter.api.Test;

class ManualVerificationAuthorityTest {
  private static final String T = "00000000-0000-4000-8000-000000000011";
  private static final String E = "00000000-0000-4000-8000-000000000012";
  private static final String I = "00000000-0000-4000-8000-000000000013";
  private static final String P = "00000000-0000-4000-8000-000000000014";
  private static final String REALM = "tenant-" + T + "-event-" + E;

  @Test
  void bindsBothPhasesToRealmTargetAndExactInitiation() {
    for (String phase : new String[] {"issue", "redeem"}) {
      AtomicInteger calls = new AtomicInteger();
      ManualVerificationAuthority.require(
          REALM,
          "synthetic-target",
          phase,
          I,
          P,
          (tenant, input) -> {
            calls.incrementAndGet();
            assertEquals(T, tenant);
            assertEquals(T, input.get("tenant_id"));
            assertEquals(E, input.get("election_event_id"));
            assertEquals("synthetic-target", input.get("voter_id"));
            assertEquals(phase, input.get("phase"));
            assertEquals(I, input.get("initiation_id"));
            assertEquals(P, input.get("proof"));
            return true;
          });
      assertEquals(1, calls.get());
    }
  }

  @Test
  void legacyCallsStillNeedAnAffirmativeDecisionAndOutagesDeny() {
    for (String phase : new String[] {"issue", "redeem"}) {
      assertThrows(
          ForbiddenException.class,
          () ->
              ManualVerificationAuthority.require(
                  REALM, "voter", phase, null, null, (t, b) -> false));
      assertThrows(
          ForbiddenException.class,
          () ->
              ManualVerificationAuthority.require(
                  REALM,
                  "voter",
                  phase,
                  I,
                  P,
                  (t, b) -> {
                    throw new IOException("offline");
                  }));
      assertDoesNotThrow(
          () ->
              ManualVerificationAuthority.require(
                  REALM, "voter", phase, null, null, (t, b) -> b.get("proof") == null));
    }
  }

  @Test
  void incompleteProofAndNonEventRealmNeverConsultAuthority() {
    ManualVerificationAuthority.Check unreachable =
        (t, b) -> {
          throw new AssertionError("should not be called");
        };
    assertThrows(
        ForbiddenException.class,
        () -> ManualVerificationAuthority.require("master", "voter", "issue", I, P, unreachable));
    assertThrows(
        ForbiddenException.class,
        () -> ManualVerificationAuthority.require(REALM, "voter", "issue", I, null, unreachable));
    assertThrows(
        ForbiddenException.class,
        () -> ManualVerificationAuthority.require(REALM, "voter", "redeem", null, P, unreachable));
    assertThrows(
        ForbiddenException.class,
        () ->
            ManualVerificationAuthority.require(
                REALM, "voter", "redeem", "invalid", P, unreachable));
  }
}
