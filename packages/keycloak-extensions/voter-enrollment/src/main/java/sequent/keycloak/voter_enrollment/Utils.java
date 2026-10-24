// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
// SPDX-FileCopyrightText: 2021,2022 Inventage AG
//
// SPDX-License-Identifier: AGPL-3.0-only

// Partially based in:
// https://github.dev/inventage/keycloak-custom/tree/tutorial-passkey/extensions/extension-passkey/src/main/java/com/inventage/keycloak/registration/Utils.java

package sequent.keycloak.voter_enrollment;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import java.time.Instant;
import java.util.Collection;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import lombok.experimental.UtilityClass;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.FormContext;
import org.keycloak.credential.hash.PasswordHashProvider;
import org.keycloak.credential.hash.Pbkdf2PasswordHashProvider;
import org.keycloak.events.Details;
import org.keycloak.events.EventBuilder;
import org.keycloak.events.EventType;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.credential.PasswordCredentialModel;
import org.keycloak.protocol.oidc.OIDCLoginProtocol;
import org.keycloak.representations.userprofile.config.UPAttribute;
import org.keycloak.representations.userprofile.config.UPConfig;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.userprofile.UserProfile;
import org.keycloak.userprofile.UserProfileContext;
import org.keycloak.userprofile.UserProfileProvider;
import sequent.keycloak.authenticator.diagnostics.ScanovateDiagnostics;
import sequent.keycloak.authenticator.messaging.ContactDetails;
import sequent.keycloak.authenticator.messaging.MessageChannel;
import sequent.keycloak.authenticator.messaging.MessagingAttributes;
import sequent.keycloak.authenticator.messaging.VerifiedContacts;
import sequent.keycloak.authenticator.messaging.VoterChannel;

@UtilityClass
@JBossLog
public class Utils {
  public final String USER_STATUS_ATTRIBUTE = "user-status";
  public final String MULTIVALUE_SEPARATOR = "##";
  public static final String ERROR_MESSAGE_NOT_SENT = "messageNotSent";
  public static final String ERROR_USER_NOT_FOUND = "userNotFound";
  public static final String ERROR_MESSAGE_USER_NOT_FOUND = "User not found";
  public static final String ERROR_USER_ATTRIBUTES_NOT_UNSET = "User Attributes Not Unset";
  public static final String ERROR_USER_ATTRIBUTES_NOT_UNSET_ERROR =
      "userShouldHaveUnsetAttributes";
  public static final String ERROR_USER_ATTRIBUTES_NOT_UNIQUE = "User Attributes Not Unique";
  public static final String PHONE_NUMBER_ATTRIBUTE = "sequent.read-only.mobile-number";
  public static final String USER_PROFILE_ATTRIBUTES = "user_profile_attributes";
  public static final String AUTHENTICATOR_CLASS_NAME = "authenticator_class_name";
  public static final String SESSION_ID = "session_id";
  public static final String ERROR_GENERATING_APPROVAL = "approvalGenerationError";

  private static final String KEYS_USERDATA = "keyUserdata";
  private static final String KEYS_USERDATA_SEPARATOR = ";";
  private static final List<String> DEFAULT_KEYS_USERDATA =
      List.of(UserModel.FIRST_NAME, UserModel.LAST_NAME, UserModel.EMAIL, UserModel.USERNAME);
  /** Server-resolved roster match, protected from registration form fields. */
  public static final String USER_ID = "userId";

  /**
   * Notes that only Keycloak sets: the code and its state, what a code verified, the verification
   * outcome, the identity method, the stored field list and the matched user. A submitted form
   * field with one of these names must not become a note, or it could plant a code, a verified
   * contact, a verified identity or another voter.
   */
  private static final Set<String> RESERVED_NOTES =
      Stream.concat(
              MessagingAttributes.KEYCLOAK_NOTES.stream(),
              Stream.of(
                  sequent.keycloak.authenticator.Utils.CODE,
                  sequent.keycloak.authenticator.Utils.CODE_TTL,
                  sequent.keycloak.authenticator.Utils.CODE_ATTEMPTS,
                  sequent.keycloak.authenticator.Utils.OTL_VISITED,
                  sequent.keycloak.authenticator.Utils.EMAIL_VERIFIED,
                  LookupAndUpdateUser.VERIFICATION_COMPLETED,
                  LookupAndUpdateUser.VERIFICATION_STATUS,
                  LookupAndUpdateUser.VERIFICATION_REJECTION_REASON,
                  LookupAndUpdateUser.VERIFICATION_MISMATCHED_FIELDS,
                  LookupAndUpdateUser.FIELDS_MATCH,
                  LookupAndUpdateUser.IDENTITY_METHOD,
                  KEYS_USERDATA,
                  USER_ID))
          .collect(Collectors.toUnmodifiableSet());

  /**
   * Form fields that become notes besides the user profile's attributes: the account fields, the
   * password, the terms box and the messaging channel choice.
   */
  static final Set<String> FORM_FIELDS =
      Set.of(
          UserModel.USERNAME,
          UserModel.EMAIL,
          UserModel.FIRST_NAME,
          UserModel.LAST_NAME,
          "password",
          "password-confirm",
          "termsAccepted",
          MessagingAttributes.FORM_OTP_CHANNEL,
          MessagingAttributes.FORM_WHATSAPP_NUMBER,
          MessagingAttributes.FORM_VIBER_NUMBER,
          MessagingAttributes.FORM_MESSAGE_CONSENT);

  /**
   * Whether a submitted field may become an authentication note: an allowlist of the profile's
   * attributes and {@link #FORM_FIELDS}. Notes only Keycloak sets, and the messaging channel
   * attributes (written only after a code verified them), never come from a form.
   */
  static boolean isFormNote(String key, Set<String> profileAttributes) {
    if (key == null
        || key.contains(KEYS_USERDATA_SEPARATOR)
        || key.startsWith("scanovate-diagnostic-")
        || RESERVED_NOTES.contains(key)
        || MessagingAttributes.CONTACT_ATTRIBUTES.contains(key)) {
      return false;
    }
    return FORM_FIELDS.contains(key) || profileAttributes.contains(key);
  }

  /** The form's messaging channel fields: the channel, its numbers and the consent. */
  static final Set<String> CHANNEL_FIELDS =
      Set.of(
          MessagingAttributes.FORM_OTP_CHANNEL,
          MessagingAttributes.FORM_WHATSAPP_NUMBER,
          MessagingAttributes.FORM_VIBER_NUMBER,
          MessagingAttributes.FORM_MESSAGE_CONSENT);

  /**
   * {@link #isFormNote(String, Set)} for a form whose channel choice is {@code choice}: the channel
   * fields become notes only when the form asks for a channel (R7-09).
   */
  static boolean isFormNote(
      String key, Set<String> profileAttributes, EnrollmentChannels.ChannelChoicePolicy choice) {
    if (CHANNEL_FIELDS.contains(key)
        && choice != EnrollmentChannels.ChannelChoicePolicy.OTP_CHANNELS) {
      return false;
    }
    return isFormNote(key, profileAttributes);
  }

  static boolean isFormNote(String key) {
    return isFormNote(key, Set.of());
  }

  String escapeJson(String value) {
    return value != null
        ? value.replace("\"", "\\\"").replace("\n", "\\n").replace("\r", "\\r")
        : null;
  }

  public static Locale getLocale(AuthenticationFlowContext context) {
    RealmModel realm = context.getRealm();
    KeycloakSession session = context.getSession();
    UserModel user = context.getUser();

    Locale locale;
    if (user != null) {
      locale = session.getContext().resolveLocale(user);
    } else {
      locale = session.getContext().resolveLocale(null);
      if (locale == null) {
        String defaultLocale = realm.getDefaultLocale();
        if (defaultLocale != null) {
          locale = Locale.forLanguageTag(defaultLocale);
        } else {
          locale = Locale.getDefault();
        }
      }
    }
    return locale;
  }

  /**
   * We store the user data entered in the registration form in the session notes. This information
   * will later be retrieved to create a user account.
   */
  static void storeUserDataInAuthSessionNotes(
      FormContext context,
      List<String> searchAttributesList,
      EnrollmentChannels.ChannelChoicePolicy channelChoice) {
    log.info("storeUserDataInAuthSessionNotes: start");
    MultivaluedMap<String, String> formData = context.getHttpRequest().getDecodedFormParameters();
    AuthenticationSessionModel sessionModel = context.getAuthenticationSession();

    // Lookup user by attributes using form data
    UserModel user = Utils.lookupUserByFormData(context, searchAttributesList, formData);

    Set<String> profileAttributes =
        getRealmUserProfileAttributes(context.getSession()).stream()
            .map(UPAttribute::getName)
            .collect(Collectors.toSet());
    if (channelChoice != EnrollmentChannels.ChannelChoicePolicy.OTP_CHANNELS) {
      // A form that doesn't ask for a channel can't set one, nor keep one set earlier (R7-09).
      CHANNEL_FIELDS.forEach(sessionModel::removeAuthNote);
      sessionModel.removeAuthNote(MessagingAttributes.NOTE_OFFERED_CHANNELS);
    }

    Map<Boolean, List<String>> fieldsByStored =
        formData.keySet().stream()
            .collect(
                Collectors.partitioningBy(
                    key -> isFormNote(key, profileAttributes, channelChoice)));
    List<String> storedFields = fieldsByStored.get(true);
    List<String> ignoredFields = fieldsByStored.get(false);

    // We store each key
    String keys = Utils.serializeUserdataKeys(storedFields);

    log.debug(
        "storeUserDataInAuthSessionNotes: setAuthNote(" + Utils.KEYS_USERDATA + ", " + keys + ")");
    sessionModel.setAuthNote(Utils.KEYS_USERDATA, keys);

    storedFields.forEach(
        key -> {
          String values = Utils.serializeUserdataKeys(formData.get(key));
          log.debug("storeUserDataInAuthSessionNotes: setAuthNote(" + key + ")");
          sessionModel.setAuthNote(key, values);
        });
    if (!ignoredFields.isEmpty()) {
      log.warnv("storeUserDataInAuthSessionNotes: ignoring fields {0}", ignoredFields);
    }

    if (user != null) {
      sessionModel.setAuthNote(USER_ID, user.getId());
    } else {
      sessionModel.removeAuthNote(USER_ID);
    }
    sessionModel.removeAuthNote(ScanovateDiagnostics.ENTERED_NOTE);
    sessionModel.removeAuthNote(ScanovateDiagnostics.ATTEMPT_NOTE);
    for (String field : ScanovateDiagnostics.ENTERED_FIELDS) {
      sessionModel.removeAuthNote(ScanovateDiagnostics.ENTERED_NOTE + "." + field);
    }
    // The data was typed again: what an earlier verification established no longer holds.
    sessionModel.removeAuthNote(LookupAndUpdateUser.IDENTITY_METHOD);
  }

  /** We retrieve the user data stored in the session notes and create a new user in this realm. */
  static void createUserFromAuthSessionNotes(AuthenticationFlowContext context) {
    MultivaluedMap<String, String> formData = context.getHttpRequest().getDecodedFormParameters();
    MultivaluedMap<String, String> userAttributes = new MultivaluedHashMap<>();

    AuthenticationSessionModel authenticationSession = context.getAuthenticationSession();
    List<String> keysUserdata =
        Utils.deserializeUserdataKeys(authenticationSession.getAuthNote(Utils.KEYS_USERDATA));

    // keys userdata is transmitted from the UserCreationPasskeyAction class.
    if (keysUserdata != null) {
      for (String key : keysUserdata) {
        String value = authenticationSession.getAuthNote(key);
        if (value != null) {
          userAttributes.add(key, value);
        }
      }
    } // In case that another custom FormAction than UserCreationPasskey is used.
    else {
      for (String key : DEFAULT_KEYS_USERDATA) {
        String value = authenticationSession.getAuthNote(key);
        if (value != null) {
          userAttributes.add(key, value);
        }
      }
    }

    String email = formData.getFirst(UserModel.EMAIL);
    String username = formData.getFirst(UserModel.USERNAME);

    if (context.getRealm().isRegistrationEmailAsUsername()) {
      username = email;
    }

    context.getEvent().detail(Details.USERNAME, username).detail(Details.REGISTER_METHOD, "form");

    KeycloakSession session = context.getSession();

    UserProfileProvider profileProvider = session.getProvider(UserProfileProvider.class);
    UserProfile profile = profileProvider.create(UserProfileContext.REGISTRATION, userAttributes);
    UserModel user = profile.create();

    user.setEnabled(true);
    context.setUser(user);

    context.getAuthenticationSession().setClientNote(OIDCLoginProtocol.LOGIN_HINT_PARAM, username);

    context.getEvent().user(user);
    context.getEvent().success();
    context.newEvent().event(EventType.LOGIN);
    context
        .getEvent()
        .client(context.getAuthenticationSession().getClient().getClientId())
        .detail(Details.REDIRECT_URI, context.getAuthenticationSession().getRedirectUri())
        .detail(Details.AUTH_METHOD, context.getAuthenticationSession().getProtocol());
    String authType = context.getAuthenticationSession().getAuthNote(Details.AUTH_TYPE);
    if (authType != null) {
      context.getEvent().detail(Details.AUTH_TYPE, authType);
    }
  }

  static UserModel lookupUserByFormData(
      FormContext context, List<String> attributes, MultivaluedMap<String, String> formData) {
    log.info("lookupUserByFormData(): start");
    KeycloakSession session = context.getSession();
    RealmModel realm = context.getRealm();

    if (attributes.isEmpty()) {
      return null;
    }

    Map<String, String> searchValues = new HashMap<>();
    for (String attribute : attributes) {
      String value = formData.getFirst(attribute);
      if (value == null || value.isBlank()) {
        log.infov("lookupUserByFormData(): search attribute {0} was not provided", attribute);
        return null;
      }
      searchValues.put(attribute, value.trim());
    }
    searchValues.put(UserModel.EXACT, Boolean.TRUE.toString());

    List<UserModel> users =
        session
            .users()
            .searchForUserStream(realm, searchValues)
            .limit(2)
            .collect(Collectors.toList());

    if (users.size() != 1) {
      log.infov("lookupUserByFormData(): {0} users matched the search attributes", users.size());
      return null;
    }
    return users.get(0);
  }

  private static String serializeUserdataKeys(Collection<String> keys, String separator) {
    return String.join(separator, keys);
  }

  private static String serializeUserdataKeys(Collection<String> keys) {
    return serializeUserdataKeys(keys, KEYS_USERDATA_SEPARATOR);
  }

  private static List<String> deserializeUserdataKeys(String key, String separator) {
    if (key == null || key.isEmpty()) {
      return Collections.emptyList();
    }
    return List.of(key.split(separator));
  }

  private static List<String> deserializeUserdataKeys(String key) {
    return deserializeUserdataKeys(key, KEYS_USERDATA_SEPARATOR);
  }

  /**
   * Recovers the values of an attribute stored in the auth notes.
   *
   * @param context
   * @param attribute attribute to recover the values from
   * @return a collection of values in string format
   */
  public static List<String> getAttributeValuesFromAuthNote(
      AuthenticationFlowContext context, String attribute) {
    return Utils.deserializeUserdataKeys(context.getAuthenticationSession().getAuthNote(attribute));
  }

  /** Recovers the user stored in the auth notes. */
  public static UserModel lookupUserByAuthNotes(AuthenticationFlowContext context) {
    String userId = context.getAuthenticationSession().getAuthNote(USER_ID);

    return context.getSession().users().getUserById(context.getRealm(), userId);
  }

  public String getUserAttributesString(UserModel user) {
    if (user != null) {
      Map<String, List<String>> attributes = user.getAttributes();
      ObjectMapper mapper = new ObjectMapper();
      ObjectNode attributesJson = mapper.createObjectNode();

      for (String attributeName : attributes.keySet()) {
        List<String> values = attributes.get(attributeName);
        String value = values == null || values.isEmpty() ? null : values.get(0);
        // Events reach the electoral log: never a voter's number or email address.
        if (value != null && !ContactDetails.isContact(attributeName, value)) {
          attributesJson.put(attributeName, value);
        }
      }

      return attributesJson.toString();
    }
    return null;
  }

  public Map<String, String> buildApplicantData(
      KeycloakSession session, AuthenticationSessionModel authSession)
      throws JsonProcessingException {
    List<UPAttribute> realmsAttributes = getRealmUserProfileAttributes(session);
    Map<String, String> applicantData = new HashMap<>();

    for (UPAttribute attribute : realmsAttributes) {
      if (MessagingAttributes.CONTACT_ATTRIBUTES.contains(attribute.getName())) {
        continue;
      }
      String authNoteValue = authSession.getAuthNote(attribute.getName());

      if (authNoteValue != null && !authNoteValue.isBlank())
        applicantData.put(attribute.getName(), authNoteValue);
    }
    RealmModel realm = session.getContext().getRealm();
    applicantData.putAll(
        verifiedChannelData(
            authSession,
            realm == null ? "" : sequent.keycloak.authenticator.Utils.getRealmName(realm),
            realm == null
                ? MessagingAttributes.CONSENT_VERSION_DEFAULT
                : VerifiedContacts.consentVersion(realm),
            Instant.now()));

    return applicantData;
  }

  /**
   * The verified channel for a reviewer's decision, read from notes only Keycloak sets (never from
   * form fields), under the user attribute names windmill writes on approval: {@code
   * sequent.read-only.message-channel}, {@code sequent.read-only.message-number} (WhatsApp, Viber,
   * SMS), {@code sequent.read-only.messenger-id} and {@code sequent.read-only.messenger-page}
   * (Messenger), and {@code sequent.read-only.message-consent} (JSON {@code {at, version,
   * organization, channel}}) when the voter consented to that channel. Empty when no code verified
   * a channel.
   */
  static Map<String, String> verifiedChannelData(
      AuthenticationSessionModel authSession,
      String organization,
      String consentVersion,
      Instant now) {
    Map<String, String> data = new HashMap<>();
    Optional<VoterChannel> verified = VerifiedContacts.verified(authSession, null);
    if (verified.isEmpty()) {
      return data;
    }
    VoterChannel channel = verified.get();
    data.put(MessagingAttributes.MESSAGE_CHANNEL, channel.channel().name());
    if (VerifiedContacts.NUMBER_CHANNELS.contains(channel.channel())) {
      data.put(MessagingAttributes.MESSAGE_NUMBER, channel.destination());
    }
    if (channel.channel() == MessageChannel.MESSENGER) {
      data.put(MessagingAttributes.MESSENGER_ID, channel.destination());
      channel.messengerPage().ifPresent(page -> data.put(MessagingAttributes.MESSENGER_PAGE, page));
    }
    VerifiedContacts.verifiedConsent(
            authSession, consentVersion, organization, channel.channel(), now)
        .ifPresent(consent -> data.put(MessagingAttributes.MESSAGE_CONSENT, consent));
    return data;
  }

  public PasswordCredentialModel buildPassword(KeycloakSession session, String rawPassword) {
    RealmModel realm = session.getContext().getRealm();

    // Use the Pbkdf2PasswordHashProvider
    Pbkdf2PasswordHashProvider hashProvider =
        (Pbkdf2PasswordHashProvider)
            session.getProvider(PasswordHashProvider.class, "pbkdf2-sha256");

    int hashIterations = realm.getPasswordPolicy().getHashIterations();

    // Create a PasswordCredentialModel
    return hashProvider.encodedCredential(rawPassword, hashIterations);
  }

  public void buildEventDetails(
      EventBuilder builder,
      AuthenticationSessionModel authSession,
      UserModel user,
      KeycloakSession session,
      String className) {
    List<UPAttribute> realmsAttributes = getRealmUserProfileAttributes(session);
    for (UPAttribute attribute : realmsAttributes) {
      String authNoteValue = authSession.getAuthNote(attribute.getName());
      if (ContactDetails.isContact(attribute.getName(), authNoteValue)) {
        continue;
      }
      builder.detail(attribute.getName(), authNoteValue);
    }
    if (user != null) {
      builder.user(user.getId());
      builder.detail(USER_PROFILE_ATTRIBUTES, getUserAttributesString(user));
    } else {
      String userId = authSession.getAuthNote(USER_ID);
      builder.user(userId);
    }
    builder.detail(AUTHENTICATOR_CLASS_NAME, className);
    builder.detail(SESSION_ID, authSession.getParentSession().getId());
  }

  public static List<UPAttribute> getRealmUserProfileAttributes(KeycloakSession session) {
    UserProfileProvider userProfileProvider = session.getProvider(UserProfileProvider.class);
    UPConfig userProfileMetadata = userProfileProvider.getConfiguration();
    return userProfileMetadata.getAttributes();
  }

  public static int parseInt(String s, int defaultValue) {
    if (s == null) return defaultValue;
    try {
      return Integer.parseInt(s);
    } catch (NumberFormatException x) {
      return defaultValue;
    }
  }
}
