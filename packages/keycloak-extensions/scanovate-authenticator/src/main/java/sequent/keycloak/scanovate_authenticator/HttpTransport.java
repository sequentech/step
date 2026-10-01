// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.io.IOException;
import java.util.List;
import java.util.Map;

/** Minimal HTTP abstraction used to talk to B-Trust. */
public interface HttpTransport {
  record HttpResult(int status, String body) {}

  /** A file part of a {@code multipart/form-data} request. */
  record MultipartPart(String name, String filename, String contentType, byte[] content) {}

  HttpResult get(String url, Map<String, String> headers) throws IOException;

  HttpResult postJson(String url, Map<String, String> headers, String body) throws IOException;

  HttpResult postMultipart(String url, Map<String, String> headers, List<MultipartPart> parts)
      throws IOException;
}
