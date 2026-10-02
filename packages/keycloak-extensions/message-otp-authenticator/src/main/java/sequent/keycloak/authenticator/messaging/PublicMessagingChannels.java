// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import com.fasterxml.jackson.databind.JsonNode;
import java.io.IOException;
import java.util.ArrayList;
import java.util.Collection;
import java.util.EnumMap;
import java.util.EnumSet;
import java.util.HashMap;
import java.util.Iterator;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.models.RealmModel;
import org.keycloak.util.JsonSerialization;

/**
 * The event's messaging channels as published to the realm attribute {@code sequent.messaging}:
 * labels and purposes only, never account identifiers or credentials. Unknown channels are ignored
 * so an older Keycloak keeps working with a newer projection.
 */
@JBossLog
public final class PublicMessagingChannels {
  public static final String REALM_ATTRIBUTE = "sequent.messaging";
  static final int VERSION = 1;

  public record MessengerPage(String pageId, String username, String name) {
    public String displayName() {
      if (name != null && !name.isBlank()) {
        return name;
      }
      return username != null && !username.isBlank() ? username : pageId;
    }
  }

  private record Channel(Set<MessagePurpose> purposes, String senderLabel) {}

  private final Map<MessageChannel, Channel> channels;
  private final Map<String, Set<MessageChannel>> electionChannels;
  private final MessengerPage messengerPage;

  private PublicMessagingChannels(
      Map<MessageChannel, Channel> channels,
      Map<String, Set<MessageChannel>> electionChannels,
      MessengerPage messengerPage) {
    this.channels = channels;
    this.electionChannels = electionChannels;
    this.messengerPage = messengerPage;
  }

  public static Optional<PublicMessagingChannels> fromRealm(RealmModel realm) {
    return realm == null ? Optional.empty() : parse(realm.getAttribute(REALM_ATTRIBUTE));
  }

  public static Optional<PublicMessagingChannels> parse(String value) {
    if (value == null || value.isBlank()) {
      return Optional.empty();
    }
    JsonNode root;
    try {
      root = JsonSerialization.readValue(value, JsonNode.class);
    } catch (IOException e) {
      log.warn("parse(): the realm messaging attribute is not valid JSON; no channels offered");
      return Optional.empty();
    }
    if (root.path("version").asInt(-1) != VERSION) {
      log.warnv("parse(): unsupported messaging version {0}", root.path("version").asText());
      return Optional.empty();
    }
    Map<MessageChannel, Channel> channels = new EnumMap<>(MessageChannel.class);
    MessengerPage page = null;
    for (JsonNode node : root.path("channels")) {
      Optional<MessageChannel> channel = MessageChannel.parse(node.path("channel").asText(null));
      if (channel.isEmpty() || channels.containsKey(channel.get())) {
        continue;
      }
      Set<MessagePurpose> purposes = EnumSet.noneOf(MessagePurpose.class);
      for (JsonNode purpose : node.path("purposes")) {
        for (MessagePurpose candidate : MessagePurpose.values()) {
          if (candidate.name().equals(purpose.asText())) {
            purposes.add(candidate);
          }
        }
      }
      channels.put(channel.get(), new Channel(purposes, text(node.path("sender_label"))));
      JsonNode pageNode = node.path("messenger_page");
      if (channel.get() == MessageChannel.MESSENGER && text(pageNode.path("page_id")) != null) {
        page =
            new MessengerPage(
                text(pageNode.path("page_id")),
                text(pageNode.path("username")),
                text(pageNode.path("name")));
      }
    }
    Map<String, Set<MessageChannel>> electionChannels = new HashMap<>();
    Iterator<Map.Entry<String, JsonNode>> elections = root.path("election_channels").fields();
    while (elections.hasNext()) {
      Map.Entry<String, JsonNode> election = elections.next();
      Set<MessageChannel> allowed = EnumSet.noneOf(MessageChannel.class);
      for (JsonNode channel : election.getValue()) {
        MessageChannel.parse(channel.asText()).ifPresent(allowed::add);
      }
      electionChannels.put(election.getKey(), allowed);
    }
    return Optional.of(new PublicMessagingChannels(channels, electionChannels, page));
  }

  private static String text(JsonNode node) {
    return node.isTextual() && !node.asText().isBlank() ? node.asText() : null;
  }

  /**
   * Channels enabled for the purpose that the voter's elections offer. When the voter's election is
   * not known yet, only channels that every restricted election offers.
   */
  public List<MessageChannel> channelsFor(MessagePurpose purpose, Collection<String> electionIds) {
    List<MessageChannel> result = new ArrayList<>();
    for (Map.Entry<MessageChannel, Channel> entry : channels.entrySet()) {
      if (entry.getValue().purposes().contains(purpose) && offeredTo(entry.getKey(), electionIds)) {
        result.add(entry.getKey());
      }
    }
    return result;
  }

  private boolean offeredTo(MessageChannel channel, Collection<String> electionIds) {
    if (electionIds == null || electionIds.isEmpty()) {
      return electionChannels.values().stream().allMatch(allowed -> allowed.contains(channel));
    }
    return electionIds.stream()
        .anyMatch(
            election -> {
              Set<MessageChannel> allowed = electionChannels.get(election);
              return allowed == null || allowed.contains(channel);
            });
  }

  public Optional<String> senderLabel(MessageChannel channel) {
    return Optional.ofNullable(channels.get(channel)).map(Channel::senderLabel);
  }

  public Optional<MessengerPage> messengerPage() {
    return Optional.ofNullable(messengerPage);
  }
}
