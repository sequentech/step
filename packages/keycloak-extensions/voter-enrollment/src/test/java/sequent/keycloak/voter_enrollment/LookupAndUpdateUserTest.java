// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.io.IOException;
import java.util.Map;
import org.junit.jupiter.api.Test;
import sequent.keycloak.harvest.HarvestEndpoint;
import sequent.keycloak.harvest.HarvestTlsPolicy;

class LookupAndUpdateUserTest {

  @Test
  void verifyApplicationUrlUsesHarvestUrl() throws IOException {
    Map<String, String> environment =
        Map.of(
            HarvestEndpoint.ENV_HARVEST_URL, "https://harvest.internal:8443",
            HarvestEndpoint.ENV_HARVEST_DOMAIN, "harvest:8400",
            HarvestEndpoint.ENV_HARVEST_TLS_POLICY, HarvestTlsPolicy.REQUIRE_TLS.name());

    assertEquals(
        "https://harvest.internal:8443/verify-application",
        LookupAndUpdateUser.verifyApplicationUrl(environment::get));
  }

  @Test
  void verifyApplicationUrlFallsBackToPlainHttpHarvestDomain() throws IOException {
    Map<String, String> environment = Map.of(HarvestEndpoint.ENV_HARVEST_DOMAIN, "harvest:8400");

    assertEquals(
        "http://harvest:8400/verify-application",
        LookupAndUpdateUser.verifyApplicationUrl(environment::get));
  }

  @Test
  void verifyApplicationUrlRejectsPlainHttpWhenTlsIsRequired() {
    Map<String, String> environment =
        Map.of(
            HarvestEndpoint.ENV_HARVEST_DOMAIN,
            "harvest:8400",
            HarvestEndpoint.ENV_HARVEST_TLS_POLICY,
            HarvestTlsPolicy.REQUIRE_TLS.name());

    assertThrows(
        IllegalStateException.class,
        () -> LookupAndUpdateUser.verifyApplicationUrl(environment::get));
  }

  @Test
  void verifyApplicationUrlRequiresConfiguredHarvest() {
    Map<String, String> environment = Map.of();

    assertThrows(
        IOException.class, () -> LookupAndUpdateUser.verifyApplicationUrl(environment::get));
  }
}
