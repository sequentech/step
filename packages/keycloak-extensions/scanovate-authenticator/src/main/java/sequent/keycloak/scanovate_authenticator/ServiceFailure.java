// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.io.IOException;
import java.net.ConnectException;
import java.net.SocketTimeoutException;
import java.net.http.HttpTimeoutException;
import javax.net.ssl.SSLException;
import sequent.keycloak.authenticator.diagnostics.ScanovateDiagnostics.ServiceError;

/** Carries only a bounded category/status, never the vendor body, URL or original exception. */
final class ServiceFailure extends IOException {
  private final ServiceError category;
  private final Integer status;

  ServiceFailure(ServiceError category) {
    this(category, null);
  }

  private ServiceFailure(ServiceError category, Integer status) {
    super("Scanovate service failure: " + category.value());
    this.category = category;
    this.status = status;
  }

  static ServiceFailure http(int status) {
    return new ServiceFailure(
        ServiceError.HTTP_ERROR, status >= 100 && status <= 599 ? status : null);
  }

  static ServiceFailure from(IOException error) {
    if (error instanceof ServiceFailure failure) return failure;
    Throwable cause = error;
    for (int depth = 0; cause != null && depth < 4; depth++, cause = cause.getCause()) {
      if (cause instanceof HttpTimeoutException || cause instanceof SocketTimeoutException)
        return new ServiceFailure(ServiceError.TIMEOUT);
      if (cause instanceof SSLException) return new ServiceFailure(ServiceError.TLS);
      if (cause instanceof ConnectException) return new ServiceFailure(ServiceError.CONNECTION);
    }
    return new ServiceFailure(ServiceError.TRANSPORT);
  }

  ServiceError category() {
    return category;
  }

  Integer status() {
    return status;
  }
}
