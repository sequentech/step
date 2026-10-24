// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import org.junit.jupiter.api.Test;

class ScanovateDiagnosticsTest {
  @Test
  void diagnosticFieldsStayWithinThePrivacyContract() {
    DiagnosticContract.verify();
  }
}
