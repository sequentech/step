// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.messaging;

import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import java.util.function.Function;
import org.keycloak.models.UserModel;
import sequent.keycloak.authenticator.Utils;

/**
 * The one channel a voter is reached on. Mirrors sequent-core's {@code voter_channel}: both must
 * give the same answer for the same record.
 *
 * @param destination E.164 number, email address or Page-scoped ID
 * @param messengerPage the Page the Page-scoped ID belongs to; Messenger only
 */
public record VoterChannel(
    MessageChannel channel, String destination, Optional<String> messengerPage, Source source) {

  /** Channel order everywhere: WhatsApp, Viber, Messenger, SMS, Email. */
  public static final List<MessageChannel> CHANNEL_ORDER =
      List.of(
          MessageChannel.WHATSAPP,
          MessageChannel.VIBER,
          MessageChannel.MESSENGER,
          MessageChannel.SMS,
          MessageChannel.EMAIL);

  /** Which shape of record the channel was read from. Logged with every outcome. */
  public enum Source {
    /** {@code message-channel} with its destination. */
    RECORDED,
    /** A first-release record: {@code verified-channels} with its per-app numbers. */
    VERIFIED_CHANNELS,
    /** Enrolled before messaging: the email, else the mobile number. */
    LEGACY
  }

  /** The voter's one channel, or empty when the record has no usable destination. */
  public static Optional<VoterChannel> of(UserModel user) {
    return resolve(user.getEmail(), user::getAttributeStream);
  }

  /**
   * The rule, on any attribute source. See sequent-core {@code voter_channel} for the three cases.
   */
  public static Optional<VoterChannel> resolve(
      String email, Function<String, java.util.stream.Stream<String>> attributes) {
    Function<String, Optional<String>> first =
        name ->
            Optional.ofNullable(attributes.apply(name))
                .flatMap(values -> values.findFirst())
                .filter(value -> !value.isEmpty());
    Optional<String> mail = Optional.ofNullable(email).filter(value -> !value.isEmpty());
    Optional<MessageChannel> recorded =
        first.apply(MessagingAttributes.MESSAGE_CHANNEL).flatMap(MessageChannel::parse);

    List<String> verifiedValues =
        Optional.ofNullable(attributes.apply(MessagingAttributes.VERIFIED_CHANNELS))
            .map(values -> values.toList())
            .orElse(List.of());
    if (!verifiedValues.isEmpty()) {
      List<MessageChannel> verified =
          verifiedValues.stream().flatMap(value -> MessageChannel.parse(value).stream()).toList();
      List<MessageChannel> candidates = new ArrayList<>();
      recorded.filter(verified::contains).ifPresent(candidates::add);
      CHANNEL_ORDER.stream().filter(verified::contains).forEach(candidates::add);
      for (MessageChannel channel : candidates) {
        Optional<VoterChannel> found =
            switch (channel) {
              case WHATSAPP ->
                  phone(
                      channel,
                      first.apply(MessagingAttributes.WHATSAPP_NUMBER),
                      Source.VERIFIED_CHANNELS);
              case VIBER ->
                  phone(
                      channel,
                      first.apply(MessagingAttributes.VIBER_NUMBER),
                      Source.VERIFIED_CHANNELS);
              case SMS ->
                  phone(
                      channel, first.apply(Utils.PHONE_NUMBER_ATTRIBUTE), Source.VERIFIED_CHANNELS);
              case MESSENGER -> messenger(first, Source.VERIFIED_CHANNELS);
              case EMAIL -> phone(channel, mail, Source.VERIFIED_CHANNELS);
            };
        if (found.isPresent()) {
          return found;
        }
      }
    } else if (recorded.isPresent()) {
      MessageChannel channel = recorded.get();
      Optional<VoterChannel> found =
          switch (channel) {
            case WHATSAPP, VIBER, SMS ->
                phone(channel, first.apply(MessagingAttributes.MESSAGE_NUMBER), Source.RECORDED);
            case MESSENGER -> messenger(first, Source.RECORDED);
            case EMAIL -> phone(channel, mail, Source.RECORDED);
          };
      if (found.isPresent()) {
        return found;
      }
    }

    if (mail.isPresent()) {
      return phone(MessageChannel.EMAIL, mail, Source.LEGACY);
    }
    return phone(MessageChannel.SMS, first.apply(Utils.PHONE_NUMBER_ATTRIBUTE), Source.LEGACY);
  }

  private static Optional<VoterChannel> phone(
      MessageChannel channel, Optional<String> destination, Source source) {
    return destination.map(value -> new VoterChannel(channel, value, Optional.empty(), source));
  }

  private static Optional<VoterChannel> messenger(
      Function<String, Optional<String>> first, Source source) {
    Optional<String> id = first.apply(MessagingAttributes.MESSENGER_ID);
    Optional<String> page = first.apply(MessagingAttributes.MESSENGER_PAGE);
    if (id.isEmpty() || page.isEmpty()) {
      return Optional.empty();
    }
    return Optional.of(new VoterChannel(MessageChannel.MESSENGER, id.get(), page, source));
  }
}
