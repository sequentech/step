// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import java.io.IOException;
import java.net.ConnectException;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpConnectTimeoutException;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.net.http.HttpTimeoutException;
import java.time.Duration;
import java.util.Set;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.util.JsonSerialization;
import sequent.keycloak.authenticator.harvest.ServiceAccountTokenClient;

/**
 * Sends through harvest's internal messaging API with a service-account token of the tenant realm.
 * Message content, destinations, codes and tokens are never logged.
 */
@JBossLog
public class HarvestMessageSenderProvider implements MessageSenderProvider {
  static final String SEND_PATH = "/messages/send";
  static final String LINK_PATH = "/messages/link";
  static final String LINK_STATUS_PATH = "/messages/link/status";
  static final String LINK_CONFIRM_PATH = "/messages/link/confirm";

  private final URI harvest;
  private final Set<MessageChannel> channels;
  private final ServiceAccountTokenClient tokens;
  private final HttpClient httpClient;
  private final Duration timeout;

  public HarvestMessageSenderProvider(
      URI harvest,
      Set<MessageChannel> channels,
      ServiceAccountTokenClient tokens,
      HttpClient httpClient,
      Duration timeout) {
    this.harvest = harvest;
    this.channels = Set.copyOf(channels);
    this.tokens = tokens;
    this.httpClient = httpClient;
    this.timeout = timeout;
  }

  public HarvestMessageSenderProvider(
      URI harvest,
      Set<MessageChannel> channels,
      ServiceAccountTokenClient tokens,
      Duration timeout) {
    this(
        harvest,
        channels,
        tokens,
        HttpClient.newBuilder().connectTimeout(timeout).build(),
        timeout);
  }

  @Override
  public Set<MessageChannel> getChannels() {
    return channels;
  }

  @Override
  public SendMessageResponse send(SendMessageRequest request) {
    SendMessageResponse response = dispatch(request);
    log.infov(
        "send(): channel={0}, purpose={1}, state={2}",
        request.channel(), request.purpose(), response.attemptState());
    return response;
  }

  private SendMessageResponse dispatch(SendMessageRequest request) {
    String token;
    try {
      token = token(request.tenantId());
    } catch (IOException e) {
      return SendMessageResponse.of(MessageAttemptState.FAILED, "service-account-token");
    }
    HttpResponse<String> response;
    try {
      response = post(SEND_PATH, request, token);
    } catch (HttpConnectTimeoutException | ConnectException e) {
      return SendMessageResponse.of(MessageAttemptState.FAILED, "unreachable");
    } catch (HttpTimeoutException e) {
      return SendMessageResponse.of(MessageAttemptState.UNKNOWN, "timeout");
    } catch (IOException e) {
      return SendMessageResponse.of(MessageAttemptState.UNKNOWN, "transport");
    }
    int status = response.statusCode();
    if (status >= 200 && status < 300) {
      try {
        return JsonSerialization.readValue(response.body(), SendMessageResponse.class);
      } catch (IOException e) {
        return SendMessageResponse.of(MessageAttemptState.UNKNOWN, "unreadable-response");
      }
    }
    if (status >= 400 && status < 500) {
      return SendMessageResponse.of(MessageAttemptState.FAILED, "rejected-" + status);
    }
    return SendMessageResponse.of(MessageAttemptState.UNKNOWN, "status-" + status);
  }

  @Override
  public CreateMessengerLinkResponse createMessengerLink(CreateMessengerLinkRequest request)
      throws IOException {
    return call(LINK_PATH, request.tenantId(), request, CreateMessengerLinkResponse.class);
  }

  @Override
  public MessengerLinkStatus messengerLinkStatus(MessengerLinkRequest request) throws IOException {
    return call(LINK_STATUS_PATH, request.tenantId(), request, MessengerLinkStatus.class);
  }

  @Override
  public MessengerLinkStatus confirmMessengerLink(MessengerLinkRequest request) throws IOException {
    return call(LINK_CONFIRM_PATH, request.tenantId(), request, MessengerLinkStatus.class);
  }

  private <T> T call(String path, String tenantId, Object body, Class<T> type) throws IOException {
    HttpResponse<String> response = post(path, body, token(tenantId));
    int status = response.statusCode();
    if (status < 200 || status >= 300) {
      log.warnv("{0} failed with status {1}", path, status);
      throw new IOException(path + " failed with status " + status);
    }
    return JsonSerialization.readValue(response.body(), type);
  }

  private String token(String tenantId) throws IOException {
    return tokens.fetchAccessToken(ServiceAccountTokenClient.tenantRealmName(tenantId));
  }

  private HttpResponse<String> post(String path, Object body, String token) throws IOException {
    HttpRequest request =
        HttpRequest.newBuilder()
            .uri(harvest.resolve(path))
            .timeout(timeout)
            .header("Content-Type", "application/json")
            .header("Authorization", "Bearer " + token)
            .POST(HttpRequest.BodyPublishers.ofString(JsonSerialization.writeValueAsString(body)))
            .build();
    try {
      return httpClient.send(request, HttpResponse.BodyHandlers.ofString());
    } catch (InterruptedException e) {
      Thread.currentThread().interrupt();
      throw new IOException("Interrupted while calling harvest", e);
    }
  }
}
