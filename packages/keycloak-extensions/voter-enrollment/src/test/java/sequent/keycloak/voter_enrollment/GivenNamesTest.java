// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;

import org.junit.jupiter.api.Test;

/** The first and middle names the voter is shown when they were compared together. */
class GivenNamesTest {
  @Test
  void theMiddleNameIsShownAfterTheFirstName() {
    assertEquals("Juan Carlos, Santos", LookupAndUpdateUser.givenNames("Juan Carlos", "Santos"));
  }

  /** A passport gives every given name in the first name, and no middle name. */
  @Test
  void aMissingNameIsLeftOut() {
    assertEquals("JUAN CARLOS SANTOS", LookupAndUpdateUser.givenNames("JUAN CARLOS SANTOS", null));
    assertEquals("JUAN CARLOS SANTOS", LookupAndUpdateUser.givenNames("JUAN CARLOS SANTOS", " "));
    assertEquals("Santos", LookupAndUpdateUser.givenNames(null, "Santos"));
    assertNull(LookupAndUpdateUser.givenNames(null, ""));
  }
}
