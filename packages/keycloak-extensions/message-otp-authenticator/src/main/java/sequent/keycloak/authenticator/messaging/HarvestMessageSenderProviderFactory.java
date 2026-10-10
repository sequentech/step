// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import com.google.auto.service.AutoService;
import java.net.URI;
import java.net.http.HttpClient;
import java.time.Duration;
import java.util.EnumSet;
import java.util.Optional;
import java.util.Set;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.Config;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import sequent.keycloak.authenticator.harvest.ServiceAccountTokenClient;

/**
 * Configuration ({@code --spi-message-sender-harvest-<key>=...}):
 *
 * <ul>
 *   <li>{@code url}: harvest's base URL; defaults to {@code http://$HARVEST_DOMAIN}.
 *   <li>{@code channels}: comma-separated channels sent through harvest; defaults to
 *       WHATSAPP,VIBER,MESSENGER, so email and SMS keep Keycloak's providers.
 *   <li>{@code timeout-seconds}: per request; defaults to 10.
 * </ul>
 */
@JBossLog
@AutoService(MessageSenderProviderFactory.class)
public class HarvestMessageSenderProviderFactory implements MessageSenderProviderFactory {
  public static final String PROVIDER_ID = "harvest";
  private static final Set<MessageChannel> DEFAULT_CHANNELS =
      EnumSet.of(MessageChannel.WHATSAPP, MessageChannel.VIBER, MessageChannel.MESSENGER);
  private static final long DEFAULT_TIMEOUT_SECONDS = 10;

  private Optional<URI> harvest;
  private Set<MessageChannel> channels;
  private Duration timeout;
  private HttpClient httpClient;
  private ServiceAccountTokenClient tokens;

  @Override
  public MessageSenderProvider create(KeycloakSession session) {
    return harvest
        .<MessageSenderProvider>map(
            uri -> new HarvestMessageSenderProvider(uri, channels, tokens, httpClient, timeout))
        .orElseGet(DefaultMessageSenderProvider::new);
  }

  @Override
  public void init(Config.Scope config) {
    harvest =
        harvestUri(
            Optional.ofNullable(config.get("url"))
                .orElseGet(() -> System.getenv("HARVEST_DOMAIN")));
    channels = parseChannels(config.get("channels"));
    Long seconds = config.getLong("timeout-seconds");
    timeout =
        Duration.ofSeconds(seconds == null || seconds <= 0 ? DEFAULT_TIMEOUT_SECONDS : seconds);
    httpClient = HttpClient.newBuilder().connectTimeout(timeout).build();
    tokens = ServiceAccountTokenClient.fromEnvironment();
    if (harvest.isEmpty()) {
      log.warn("init(): no harvest URL configured; the harvest message sender sends nothing");
    } else {
      log.infov("init(): harvest message sender for channels {0}", channels);
    }
  }

  static Optional<URI> harvestUri(String value) {
    if (value == null || value.isBlank()) {
      return Optional.empty();
    }
    String url = value.trim();
    if (url.endsWith("/")) {
      url = url.substring(0, url.length() - 1);
    }
    return Optional.of(URI.create(url.contains("://") ? url : "http://" + url));
  }

  static Set<MessageChannel> parseChannels(String value) {
    if (value == null || value.isBlank()) {
      return DEFAULT_CHANNELS;
    }
    Set<MessageChannel> parsed = EnumSet.noneOf(MessageChannel.class);
    for (String name : value.split(",")) {
      Optional<MessageChannel> channel = MessageChannel.parse(name);
      if (channel.isPresent()) {
        parsed.add(channel.get());
      } else {
        log.warnv("parseChannels(): ignoring unknown channel {0}", name.trim());
      }
    }
    return parsed;
  }

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public void close() {}

  @Override
  public String getId() {
    return PROVIDER_ID;
  }
}
