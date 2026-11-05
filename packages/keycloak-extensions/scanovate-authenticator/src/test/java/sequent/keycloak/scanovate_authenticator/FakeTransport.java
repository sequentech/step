// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.scanovate_authenticator;

import java.io.IOException;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Deque;
import java.util.List;
import java.util.Map;

/** Records the requests sent to an on-premise service and answers them with queued replies. */
class FakeTransport implements HttpTransport {
  record Call(String url, Map<String, String> headers, String body) {}

  final List<Call> calls = new ArrayList<>();
  private final Deque<Object> replies = new ArrayDeque<>();

  FakeTransport reply(int status, String body) {
    replies.add(new HttpResult(status, body));
    return this;
  }

  FakeTransport fail(String message) {
    replies.add(new IOException(message));
    return this;
  }

  @Override
  public HttpResult postJson(String url, Map<String, String> headers, String body)
      throws IOException {
    calls.add(new Call(url, headers, body));
    Object reply = replies.poll();
    if (reply instanceof IOException exception) {
      throw exception;
    }
    return (HttpResult) reply;
  }
}
