// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import jakarta.ws.rs.core.MultivaluedMap;
import java.time.Instant;
import java.util.ArrayList;
import java.util.Collection;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import java.util.stream.Collectors;
import lombok.experimental.UtilityClass;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.utils.FormMessage;
import org.keycloak.representations.userprofile.config.UPAttribute;
import org.keycloak.services.messages.Messages;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.userprofile.UserProfileProvider;
import sequent.keycloak.authenticator.messaging.MessageChannel;
import sequent.keycloak.authenticator.messaging.MessageSenderProvider;
import sequent.keycloak.authenticator.messaging.MessagingAttributes;
import sequent.keycloak.authenticator.messaging.PublicMessagingChannels;
import sequent.keycloak.authenticator.messaging.VerifiedContacts;
import sequent.keycloak.authenticator.messaging.VoterChannels;

/** The enrollment's choice of channel for codes, and saving the contact it verified. */
@UtilityClass
public class EnrollmentChannels {

  /** Whether the registration form asks the voter how to get codes. */
  public enum ChannelChoicePolicy {
    NONE,
    OTP_CHANNELS
  }

  /**
   * The channels the voter's Post offers codes on that this Keycloak can deliver. The Post is the
   * ID or a label of its election; one that matches no election only gets the channels every
   * restricted election offers.
   */
  public List<MessageChannel> offered(
      KeycloakSession session, RealmModel realm, Collection<String> posts) {
    MessageSenderProvider sender = VoterChannels.sender(session);
    return VoterChannels.offeredForOtp(
            PublicMessagingChannels.fromRealm(realm),
            posts,
            PublicMessagingChannels.UnmatchedPostPolicy.COMMON_CHANNELS)
        .stream()
        .filter(channel -> VoterChannels.deliverable(sender, channel))
        .collect(Collectors.toList());
  }

  public String consentVersion(RealmModel realm) {
    return VerifiedContacts.consentVersion(realm);
  }

  /** What the consent box submits: the wording it showed and the channel it named. */
  public String consentValue(String consentVersion, MessageChannel channel) {
    return VerifiedContacts.consentValue(consentVersion, channel);
  }

  /** Errors of the submitted channel choice, its contact and the consent it needs. */
  public List<FormMessage> validate(
      List<MessageChannel> offered,
      MultivaluedMap<String, String> formData,
      String mobileAttribute,
      String consentVersion) {
    List<FormMessage> errors = new ArrayList<>();
    String submitted = formData.getFirst(MessagingAttributes.FORM_OTP_CHANNEL);
    Optional<MessageChannel> channel = MessageChannel.parse(submitted);
    if (channel.isEmpty()) {
      errors.add(
          new FormMessage(
              MessagingAttributes.FORM_OTP_CHANNEL, "messaging.channelChoice.required"));
      return errors;
    }
    if (!offered.contains(channel.get())) {
      errors.add(
          new FormMessage(
              MessagingAttributes.FORM_OTP_CHANNEL, "messaging.channelChoice.notAvailable"));
      return errors;
    }
    String numberField =
        switch (channel.get()) {
          case SMS -> mobileAttribute;
          case WHATSAPP -> MessagingAttributes.FORM_WHATSAPP_NUMBER;
          case VIBER -> MessagingAttributes.FORM_VIBER_NUMBER;
          case EMAIL, MESSENGER -> null;
        };
    if (numberField != null && !VoterChannels.isE164(trimmed(formData.getFirst(numberField)))) {
      errors.add(new FormMessage(numberField, "messaging.number.invalid"));
    }
    if (channel.get() == MessageChannel.EMAIL && trimmed(formData.getFirst("email")) == null) {
      errors.add(new FormMessage("email", Messages.MISSING_EMAIL));
    }
    // Consent names the organization and the channel, and is needed for every channel.
    if (!consentValue(consentVersion, channel.get())
        .equals(formData.getFirst(MessagingAttributes.FORM_MESSAGE_CONSENT))) {
      errors.add(
          new FormMessage(MessagingAttributes.FORM_MESSAGE_CONSENT, "messaging.consent.required"));
    }
    return errors;
  }

  private String trimmed(String value) {
    return value == null || value.isBlank() ? null : value.trim();
  }

  /**
   * Keeps the consent the validated form gave, in a note only Keycloak sets, for when the voter's
   * contact is saved.
   */
  public void recordConsent(
      AuthenticationSessionModel authSession,
      MultivaluedMap<String, String> formData,
      String consentVersion) {
    Optional<MessageChannel> channel =
        MessageChannel.parse(formData.getFirst(MessagingAttributes.FORM_OTP_CHANNEL));
    if (channel.isPresent()
        && consentValue(consentVersion, channel.get())
            .equals(formData.getFirst(MessagingAttributes.FORM_MESSAGE_CONSENT))) {
      authSession.setAuthNote(
          MessagingAttributes.NOTE_VERIFIED_CONSENT, consentValue(consentVersion, channel.get()));
    } else {
      authSession.removeAuthNote(MessagingAttributes.NOTE_VERIFIED_CONSENT);
    }
  }

  /**
   * The values the Post attribute can take: the options of its profile attribute, in their order.
   */
  public List<String> postOptions(KeycloakSession session, String postAttribute) {
    if (postAttribute == null || postAttribute.isBlank()) {
      return List.of();
    }
    UserProfileProvider profiles = session.getProvider(UserProfileProvider.class);
    UPAttribute attribute =
        profiles == null ? null : profiles.getConfiguration().getAttribute(postAttribute);
    if (attribute == null || attribute.getValidations() == null) {
      return List.of();
    }
    Map<String, Object> options = attribute.getValidations().get("options");
    Object values = options == null ? null : options.get("options");
    List<String> result = new ArrayList<>();
    if (values instanceof Collection<?> list) {
      for (Object value : list) {
        if (value != null && !value.toString().isBlank()) {
          result.add(value.toString());
        }
      }
    }
    return result;
  }

  /**
   * Each Post's channels, keyed by the Post's option value, for the enrollment page to show the
   * selected Post's channels and refresh them when the Post changes.
   */
  public Map<String, List<String>> channelsByPost(
      KeycloakSession session, RealmModel realm, String postAttribute) {
    Map<String, List<String>> byPost = new LinkedHashMap<>();
    for (String post : postOptions(session, postAttribute)) {
      byPost.put(post, VoterChannels.names(offered(session, realm, Set.of(post))));
    }
    return byPost;
  }

  /**
   * Saves the contact the enrollment code verified, replacing the voter's channel; consent is
   * recorded with its wording and the organization it named.
   */
  public void persist(
      UserModel user,
      AuthenticationSessionModel authSession,
      String organization,
      String consentVersion,
      Instant now) {
    VerifiedContacts.persist(user, authSession, organization, consentVersion, now);
  }
}
