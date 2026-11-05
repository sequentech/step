// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.IOException;
import lombok.extern.jbosslog.JBossLog;
import sequent.keycloak.authenticator.diagnostics.ScanovateDiagnostics.ServiceError;
import sequent.keycloak.scanovate_authenticator.HttpTransport.HttpResult;

/**
 * Sends requests that answer JSON, retrying with exponential backoff on network and server errors.
 * Any other status than 200 fails right away.
 */
@JBossLog
class RetryingRequests {
  static final long BASE_RETRY_DELAY_MS = 1_000;

  private static final ObjectMapper MAPPER = new ObjectMapper();

  @FunctionalInterface
  interface Request {
    HttpResult send() throws IOException;
  }

  private final int maxRetries;
  private final Sleeper sleeper;

  RetryingRequests(int maxRetries, Sleeper sleeper) {
    this.maxRetries = Math.max(1, maxRetries);
    this.sleeper = sleeper;
  }

  /** Sends the request and parses its JSON body. {@code path} names the request in errors. */
  JsonNode execute(String path, Request request) throws IOException {
    ServiceFailure lastError = new ServiceFailure(ServiceError.TRANSPORT);
    for (int attempt = 0; attempt < maxRetries; attempt++) {
      if (attempt > 0) {
        backoff(attempt);
      }
      HttpResult result;
      try {
        result = request.send();
      } catch (IOException e) {
        log.warnv("{0}: attempt {1} failed", path, attempt + 1);
        lastError = ServiceFailure.from(e);
        continue;
      }
      if (result.status() >= 500) {
        log.warnv("{0}: attempt {1} got status {2}", path, attempt + 1, result.status());
        lastError = ServiceFailure.http(result.status());
        continue;
      }
      if (result.status() != 200) {
        throw ServiceFailure.http(result.status());
      }
      try {
        JsonNode response = MAPPER.readTree(result.body());
        if (response == null || !response.isObject())
          throw new ServiceFailure(ServiceError.MALFORMED_RESPONSE);
        return response;
      } catch (IOException e) {
        throw new ServiceFailure(ServiceError.MALFORMED_RESPONSE);
      }
    }
    throw lastError;
  }

  private void backoff(int attempt) throws IOException {
    try {
      sleeper.sleep(BASE_RETRY_DELAY_MS * (1L << (attempt - 1)));
    } catch (InterruptedException e) {
      Thread.currentThread().interrupt();
      throw new ServiceFailure(ServiceError.INTERRUPTED);
    }
  }
}
