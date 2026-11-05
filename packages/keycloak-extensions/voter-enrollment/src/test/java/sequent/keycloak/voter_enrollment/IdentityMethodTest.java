// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import java.util.HashMap;
import java.util.Map;
import java.util.Optional;
import org.junit.jupiter.api.Test;
import org.keycloak.sessions.AuthenticationSessionModel;

class IdentityMethodTest {
  private static Optional<IdentityMethod> forwarded(String note) {
    AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
    when(authSession.getAuthNote(LookupAndUpdateUser.IDENTITY_METHOD)).thenReturn(note);
    return LookupAndUpdateUser.identityMethod(authSession);
  }

  @Test
  void theLookupForwardsTheMethodAVerificationStepSet() {
    assertEquals(Optional.of(IdentityMethod.VERIFIED), forwarded("VERIFIED"));
    assertEquals(Optional.of(IdentityMethod.MANUAL_ENTRY), forwarded("MANUAL_ENTRY"));
  }

  /**
   * Without a verification step, or with a value no step sets, the application carries no method:
   * the matrix's identity conditions don't apply, and nothing becomes VERIFIED by default.
   */
  @Test
  void aMissingOrUnknownMethodIsNotForwarded() {
    assertEquals(Optional.empty(), forwarded(null));
    assertEquals(Optional.empty(), forwarded(""));
    assertEquals(Optional.empty(), forwarded("verified"));
    assertEquals(Optional.empty(), forwarded("VERIFIED;MANUAL_ENTRY"));
    assertEquals(Optional.empty(), forwarded("AUTOMATIC"));
  }

  @Test
  void theAnnotationsCarryOnlyAKnownMethod() {
    AuthenticationSessionModel authSession = mock(AuthenticationSessionModel.class);
    when(authSession.getAuthNote(LookupAndUpdateUser.IDENTITY_METHOD)).thenReturn("VERIFIED");
    Map<String, Object> annotations = new HashMap<>();

    LookupAndUpdateUser.putIdentityMethod(annotations, authSession);

    assertEquals(Map.of(LookupAndUpdateUser.IDENTITY_METHOD, "VERIFIED"), annotations);

    when(authSession.getAuthNote(LookupAndUpdateUser.IDENTITY_METHOD)).thenReturn("verified");
    annotations.clear();

    LookupAndUpdateUser.putIdentityMethod(annotations, authSession);

    assertEquals(Map.of(), annotations);
  }
}
