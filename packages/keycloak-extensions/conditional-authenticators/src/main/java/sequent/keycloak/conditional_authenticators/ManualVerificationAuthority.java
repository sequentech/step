// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.conditional_authenticators;

import jakarta.ws.rs.ForbiddenException;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import org.keycloak.util.JsonSerialization;
import sequent.keycloak.authenticator.harvest.ServiceAccountTokenClient;

/**
 * The signing authority is Harvest, never a browser, bearer-role label or caller-provided realm.
 */
final class ManualVerificationAuthority {
  static final String INITIATION = "X-Sequent-Verification-Initiation";
  static final String PROOF = "X-Sequent-Verification-Proof";
  private static final String UUID = "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}";
  private static final Pattern SCOPE =
      Pattern.compile("tenant-(" + UUID + ")-event-(" + UUID + ")");
  private static final Pattern ID = Pattern.compile(UUID);
  private static final HttpClient HTTP =
      HttpClient.newBuilder().connectTimeout(Duration.ofSeconds(3)).build();

  private ManualVerificationAuthority() {}

  interface Check {
    boolean allowed(String tenant, Map<String, String> input) throws Exception;
  }

  static void require(String realm, String voter, String phase, String initiation, String proof) {
    require(realm, voter, phase, initiation, proof, ManualVerificationAuthority::call);
  }

  static void require(
      String realm, String voter, String phase, String initiation, String proof, Check check) {
    Matcher scope = SCOPE.matcher(realm == null ? "" : realm);
    if (!scope.matches()
        || voter == null
        || voter.isBlank()
        || !("issue".equals(phase) || "redeem".equals(phase))
        || ((initiation == null) != (proof == null))
        || (initiation != null
            && (!ID.matcher(initiation).matches() || !ID.matcher(proof).matches()))) {
      throw denied();
    }
    Map<String, String> body = new LinkedHashMap<>();
    body.put("tenant_id", scope.group(1));
    body.put("election_event_id", scope.group(2));
    body.put("voter_id", voter);
    body.put("phase", phase);
    body.put("initiation_id", initiation);
    body.put("proof", proof);
    try {
      if (check.allowed(scope.group(1), body)) return;
    } catch (InterruptedException interrupted) {
      Thread.currentThread().interrupt();
    } catch (Exception unavailable) {
      // Do not surface/log a token, proof, request body, voter or URL.
    }
    throw denied();
  }

  private static ForbiddenException denied() {
    return new ForbiddenException("Manual verification is not authorized");
  }

  static boolean call(String tenant, Map<String, String> body) throws Exception {
    String domain = System.getenv("HARVEST_DOMAIN");
    if (domain == null || domain.isBlank()) return false;
    String token =
        ServiceAccountTokenClient.shared()
            .fetchAccessToken(ServiceAccountTokenClient.tenantRealmName(tenant));
    // The deployment supplies HARVEST_DOMAIN, not the request. Never follow redirects.
    HttpRequest request =
        HttpRequest.newBuilder(URI.create("http://" + domain + "/manual-verification-authority"))
            .timeout(Duration.ofSeconds(8))
            .header("Authorization", "Bearer " + token)
            .header("Content-Type", "application/json")
            .POST(HttpRequest.BodyPublishers.ofByteArray(JsonSerialization.writeValueAsBytes(body)))
            .build();
    return HTTP.send(request, HttpResponse.BodyHandlers.discarding()).statusCode() == 204;
  }
}
