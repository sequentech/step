// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import static org.junit.jupiter.api.Assertions.*;

import java.io.IOException;
import java.net.ConnectException;
import java.net.SocketTimeoutException;
import java.net.http.HttpTimeoutException;
import java.util.List;
import javax.net.ssl.SSLException;
import org.junit.jupiter.api.Test;
import sequent.keycloak.authenticator.diagnostics.ScanovateDiagnostics.ServiceError;

class ServiceFailureTest {
  @Test
  void transportTypesAreClassifiedWithoutKeepingTheirMessagesOrCauses() {
    List<IOException> failures =
        List.of(
            new HttpTimeoutException("synthetic secret"),
            new SocketTimeoutException("synthetic secret"),
            new ConnectException("synthetic secret"),
            new SSLException("synthetic secret"),
            new IOException("synthetic secret"));
    List<ServiceError> expected =
        List.of(
            ServiceError.TIMEOUT,
            ServiceError.TIMEOUT,
            ServiceError.CONNECTION,
            ServiceError.TLS,
            ServiceError.TRANSPORT);
    for (int index = 0; index < failures.size(); index++) {
      ServiceFailure failure =
          ServiceFailure.from(new IOException("synthetic outer", failures.get(index)));
      assertEquals(expected.get(index), failure.category());
      assertNull(failure.getCause());
      assertFalse(failure.toString().contains("synthetic"));
    }
    ServiceFailure status = ServiceFailure.http(401);
    assertEquals(401, status.status());
    assertEquals(ServiceError.HTTP_ERROR, status.category());
    assertSame(status, ServiceFailure.from(status));
    assertNull(ServiceFailure.http(999).status());
  }

  @Test
  void requestBoundaryPreservesStatusAndRejectsMalformedBodies() {
    RetryingRequests requests = new RetryingRequests(1, millis -> {});
    ServiceFailure rejected =
        assertThrows(
            ServiceFailure.class,
            () ->
                requests.execute(
                    "/fixed", () -> new HttpTransport.HttpResult(401, "synthetic secret")));
    assertEquals(401, rejected.status());
    assertEquals(ServiceError.HTTP_ERROR, rejected.category());
    for (String body : List.of("not json", "null", "[]", "\"synthetic secret\"")) {
      ServiceFailure malformed =
          assertThrows(
              ServiceFailure.class,
              () -> requests.execute("/fixed", () -> new HttpTransport.HttpResult(200, body)));
      assertEquals(ServiceError.MALFORMED_RESPONSE, malformed.category());
      assertNull(malformed.getCause());
      assertFalse(malformed.toString().contains("synthetic"));
    }
    ServiceFailure unavailable =
        assertThrows(
            ServiceFailure.class,
            () ->
                requests.execute(
                    "/fixed", () -> new HttpTransport.HttpResult(503, "synthetic secret")));
    assertEquals(503, unavailable.status());
  }
}
