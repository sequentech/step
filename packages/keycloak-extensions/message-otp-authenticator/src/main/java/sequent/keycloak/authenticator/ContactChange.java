// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import sequent.keycloak.authenticator.messaging.MessageChannel;
import sequent.keycloak.authenticator.messaging.MessagePurpose;
import sequent.keycloak.authenticator.messaging.MessageReason;
import sequent.keycloak.authenticator.messaging.MessageSenderProvider;
import sequent.keycloak.authenticator.messaging.PublicMessagingChannels;
import sequent.keycloak.authenticator.messaging.VoterChannels;

/**
 * Whether a voter may change how they are contacted ({@code message-channel-ra}), and on which
 * channels. One decision, shared by the action ({@code canRun}) and by the voting portal's entry
 * ({@code GET /realms/{realm}/messaging-channels}), so the entry is offered exactly when the action
 * would run (R7-14). Each refusal has one reason, the same in the log, the event and the portal's
 * answer.
 *
 * <ol>
 *   <li>The event must publish its channels ({@code sequent.messaging}); otherwise no channel is
 *       offered for codes.
 *   <li>Every enabled message OTP step of the realm must send to the voter's channel ({@code
 *       CHOSEN}). Otherwise a new channel would not get the sign-in codes of a step with a
 *       configured courier (R7-01): "A code step doesn't send to the voter's channel".
 *   <li>The voter's Post must offer at least one channel for codes that something delivers; email
 *       only when the account has an address.
 * </ol>
 */
@JBossLog
public final class ContactChange {
  private ContactChange() {}

  /**
   * The channels the voter may move to, in the ticket's order, or why none.
   *
   * @param refusal empty when the change is offered
   */
  public record Decision(List<MessageChannel> channels, Optional<MessageReason> refusal) {
    static Decision offer(List<MessageChannel> channels) {
      return new Decision(List.copyOf(channels), Optional.empty());
    }

    static Decision refuse(MessageReason reason) {
      return new Decision(List.of(), Optional.of(reason));
    }

    public boolean offered() {
      return refusal.isEmpty() && !channels.isEmpty();
    }
  }

  /** The decision for a signed-in voter: what the action does, and what the portal shows them. */
  public static Decision forVoter(KeycloakSession session, RealmModel realm, UserModel user) {
    Optional<PublicMessagingChannels> projection = PublicMessagingChannels.fromRealm(realm);
    if (projection.isEmpty() || user == null) {
      return Decision.refuse(MessageReason.CHANNEL_NOT_OFFERED_FOR_CODES);
    }
    Optional<MessageReason> steps = codeStepsRefusal(realm);
    if (steps.isPresent()) {
      return Decision.refuse(steps.get());
    }
    List<MessageChannel> offered = offered(session, projection, user);
    return offered.isEmpty()
        ? Decision.refuse(MessageReason.CHANNEL_NOT_OFFERED_FOR_CODES)
        : Decision.offer(offered);
  }

  /**
   * Without a voter: the event's code channels, by name. An event without messaging offers none and
   * gives no reason (nothing to explain); a realm whose code steps don't all send to the voter's
   * channel offers none, with the reason.
   */
  public static Decision forEvent(RealmModel realm) {
    Optional<PublicMessagingChannels> projection = PublicMessagingChannels.fromRealm(realm);
    if (projection.isEmpty()) {
      return Decision.offer(List.of());
    }
    Optional<MessageReason> steps = codeStepsRefusal(realm);
    if (steps.isPresent()) {
      return Decision.refuse(steps.get());
    }
    return Decision.offer(projection.get().channelsWithPurpose(MessagePurpose.OTP));
  }

  /**
   * {@link MessageReason#CODE_STEP_NOT_ON_VOTER_CHANNEL} while any code step that can run is
   * neither CHOSEN nor NONE.
   */
  static Optional<MessageReason> codeStepsRefusal(RealmModel realm) {
    List<String> steps = Utils.codeStepsNotOnVoterChannel(realm);
    if (steps.isEmpty()) {
      return Optional.empty();
    }
    log.infov(
        "contact change not offered: {0} (message OTP executions {1} use a configured courier)",
        MessageReason.CODE_STEP_NOT_ON_VOTER_CHANNEL.reason(), steps);
    return Optional.of(MessageReason.CODE_STEP_NOT_ON_VOTER_CHANNEL);
  }

  /**
   * The channels the voter's Post offers codes on, in the ticket's order, that something delivers.
   * Email is the account's own address, so it is offered only when the account has one.
   */
  static List<MessageChannel> offered(
      KeycloakSession session, Optional<PublicMessagingChannels> projection, UserModel user) {
    MessageSenderProvider sender = VoterChannels.sender(session);
    List<MessageChannel> offered = new ArrayList<>();
    for (MessageChannel channel :
        VoterChannels.offeredForOtp(projection, VoterChannels.elections(user))) {
      if (!VoterChannels.deliverable(sender, channel)) {
        continue;
      }
      if (channel == MessageChannel.EMAIL
          && (user.getEmail() == null || user.getEmail().isBlank())) {
        continue;
      }
      offered.add(channel);
    }
    return offered;
  }
}
