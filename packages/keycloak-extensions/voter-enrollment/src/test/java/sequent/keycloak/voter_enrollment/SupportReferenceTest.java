// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.voter_enrollment;

import static org.junit.jupiter.api.Assertions.*;

import java.util.Optional;
import org.junit.jupiter.api.Test;

/**
 * Q3: the pending page's support reference comes from the application record: 8 capitals and digits
 * in two groups of four, the same for the same application, never from the session or a contact.
 */
class SupportReferenceTest {
  private static final String APPLICATION = "7f0c2a9e-1b3d-4c5e-8f60-123456789abc";

  @Test
  void theReferenceIsEightCapitalsAndDigitsInTwoGroups() {
    String reference = SupportReference.of(APPLICATION).orElseThrow();
    assertTrue(reference.matches("[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}"), reference);
  }

  @Test
  void theSameApplicationAlwaysHasTheSameReference() {
    assertEquals(SupportReference.of(APPLICATION), SupportReference.of(APPLICATION));
    assertEquals(SupportReference.of(APPLICATION), SupportReference.of(APPLICATION.toUpperCase()));
    assertNotEquals(
        SupportReference.of(APPLICATION),
        SupportReference.of("7f0c2a9e-1b3d-4c5e-8f60-123456789abd"));
  }

  /** A fixed vector, so the Admin Portal (or harvest) can compute the same reference. */
  @Test
  void theDerivationIsStable() {
    assertEquals(Optional.of("65SR-MSSD"), SupportReference.of(APPLICATION));
  }

  @Test
  void noApplicationNoReference() {
    assertEquals(Optional.empty(), SupportReference.of(null));
    assertEquals(Optional.empty(), SupportReference.of(" "));
  }
}
