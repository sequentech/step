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
import java.util.HashSet;
import java.util.Iterator;
import java.util.List;
import java.util.Locale;
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

  /** What a Post that matches no election of the projection is offered. */
  public enum UnmatchedPostPolicy {
    /**
     * Only channels that every restricted election offers: for a Post a voter enters, which may not
     * be a known election.
     */
    COMMON_CHANNELS,
    /**
     * Every enabled channel: for the elections of a saved voter, where unlisted is unrestricted.
     */
    EVERY_CHANNEL
  }

  private record Channel(Set<MessagePurpose> purposes, String senderLabel) {}

  private final Map<MessageChannel, Channel> channels;
  private final Map<String, Set<MessageChannel>> electionChannels;
  private final Map<String, Set<String>> electionNames;
  private final MessengerPage messengerPage;

  private PublicMessagingChannels(
      Map<MessageChannel, Channel> channels,
      Map<String, Set<MessageChannel>> electionChannels,
      Map<String, Set<String>> electionNames,
      MessengerPage messengerPage) {
    this.channels = channels;
    this.electionChannels = electionChannels;
    this.electionNames = electionNames;
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
    return Optional.of(
        new PublicMessagingChannels(
            channels, electionChannels, electionNames(root, electionChannels.keySet()), page));
  }

  /** Every ID and label an election is known by. A label that two elections share names both. */
  private static Map<String, Set<String>> electionNames(JsonNode root, Set<String> restricted) {
    Map<String, Set<String>> names = new HashMap<>();
    for (String election : restricted) {
      names.computeIfAbsent(normalize(election), key -> new HashSet<>()).add(election);
    }
    Iterator<Map.Entry<String, JsonNode>> elections = root.path("election_labels").fields();
    while (elections.hasNext()) {
      Map.Entry<String, JsonNode> election = elections.next();
      String id = election.getKey();
      names.computeIfAbsent(normalize(id), key -> new HashSet<>()).add(id);
      for (JsonNode label : election.getValue()) {
        if (text(label) != null) {
          names.computeIfAbsent(normalize(label.asText()), key -> new HashSet<>()).add(id);
        }
      }
    }
    return names;
  }

  private static String normalize(String value) {
    return value.trim().toLowerCase(Locale.ROOT);
  }

  private static String text(JsonNode node) {
    return node.isTextual() && !node.asText().isBlank() ? node.asText() : null;
  }

  /**
   * Channels enabled for the purpose that the elections of a saved voter offer. An election the
   * projection does not list has no restriction.
   */
  public List<MessageChannel> channelsFor(MessagePurpose purpose, Collection<String> posts) {
    return channelsFor(purpose, posts, UnmatchedPostPolicy.EVERY_CHANNEL);
  }

  /**
   * Channels enabled for the purpose that the voter's Posts offer. A Post is the ID of its election
   * or any of its labels, compared without case or surrounding spaces. When no Post is known yet,
   * only channels that every restricted election offers.
   */
  public List<MessageChannel> channelsFor(
      MessagePurpose purpose, Collection<String> posts, UnmatchedPostPolicy unmatched) {
    List<MessageChannel> result = new ArrayList<>();
    for (Map.Entry<MessageChannel, Channel> entry : channels.entrySet()) {
      if (entry.getValue().purposes().contains(purpose)
          && offeredTo(entry.getKey(), posts, unmatched)) {
        result.add(entry.getKey());
      }
    }
    return result;
  }

  private boolean offeredTo(
      MessageChannel channel, Collection<String> posts, UnmatchedPostPolicy unmatched) {
    if (posts == null || posts.isEmpty()) {
      return everyRestrictedElectionOffers(channel);
    }
    return posts.stream().anyMatch(post -> offeredToPost(channel, post, unmatched));
  }

  private boolean everyRestrictedElectionOffers(MessageChannel channel) {
    return electionChannels.values().stream().allMatch(allowed -> allowed.contains(channel));
  }

  private boolean electionOffers(MessageChannel channel, String election) {
    Set<MessageChannel> allowed = electionChannels.get(election);
    return allowed == null || allowed.contains(channel);
  }

  private boolean offeredToPost(
      MessageChannel channel, String post, UnmatchedPostPolicy unmatched) {
    Set<String> elections = electionNames.get(normalize(post));
    if (elections == null) {
      return unmatched == UnmatchedPostPolicy.EVERY_CHANNEL
          || everyRestrictedElectionOffers(channel);
    }
    return elections.stream().allMatch(election -> electionOffers(channel, election));
  }

  public Optional<String> senderLabel(MessageChannel channel) {
    return Optional.ofNullable(channels.get(channel)).map(Channel::senderLabel);
  }

  public Optional<MessengerPage> messengerPage() {
    return Optional.ofNullable(messengerPage);
  }
}
