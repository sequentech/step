// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.Map;
import java.util.Set;
import java.util.regex.Pattern;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.forms.RegistrationPage;
import org.keycloak.models.KeycloakSession;
import org.keycloak.protocol.oidc.endpoints.AuthorizationEndpoint;
import org.keycloak.userprofile.AttributeMetadata;
import org.keycloak.userprofile.Attributes;
import org.keycloak.userprofile.UserProfileContext;
import org.keycloak.userprofile.UserProfileProvider;

@JBossLog
final class LoginHintPrefill {

  static final String HINT_PREFIX = "login_hint__";
  static final int MAX_HINT_COUNT = 5;
  static final int MAX_HINT_NAME_LENGTH = 128;
  static final int MAX_HINT_VALUE_LENGTH = 255;
  static final String PREFILL_POLICY_ANNOTATION = "loginHintPrefillPolicy";
  static final String READ_ONLY_FIELD_MODIFIED_MESSAGE = "loginHintReadOnlyFieldModified";

  private static final String CLIENT_NOTE_PREFIX =
      AuthorizationEndpoint.LOGIN_SESSION_NOTE_ADDITIONAL_REQ_PARAMS_PREFIX + HINT_PREFIX;
  private static final Pattern HINT_NAME_PATTERN = Pattern.compile("[A-Za-z0-9._-]+");
  private static final String INPUT_TYPE_ANNOTATION = "inputType";
  private static final String HIDDEN_INPUT_TYPE = "hidden";
  private static final Set<String> CREDENTIAL_FIELDS =
      Set.of(RegistrationPage.FIELD_PASSWORD, RegistrationPage.FIELD_PASSWORD_CONFIRM);

  /**
   * Per-attribute policy declaring how a validated login hint may prefill the attribute. Declared
   * as the {@value #PREFILL_POLICY_ANNOTATION} user profile annotation.
   */
  enum AttributePrefillPolicy {
    /** Prefill the field and let the voter change the value. Applied when unannotated. */
    EDITABLE,
    /** Prefill the field, render it read-only and reject a value the voter changed. */
    READ_ONLY,
    /** Never prefill the field from a login hint. */
    IGNORE;

    static AttributePrefillPolicy parse(String value) {
      try {
        return valueOf(value.trim().toUpperCase());
      } catch (IllegalArgumentException unknownPolicy) {
        log.warnv(
            "Unknown {0} annotation value {1}, refusing to prefill the attribute",
            PREFILL_POLICY_ANNOTATION, value);
        return IGNORE;
      }
    }
  }

  /** Login hint prefill data resolved for a single registration form request. */
  record Prefill(MultivaluedMap<String, String> writableHints, Set<String> lockedAttributes) {

    static final Prefill EMPTY = new Prefill(new MultivaluedHashMap<>(), Set.of());

    boolean isEmpty() {
      return writableHints.isEmpty();
    }
  }

  private LoginHintPrefill() {}

  /**
   * Resolves which registration fields the validated login hints may prefill, and which of those
   * the voter is not allowed to change.
   *
   * @param session session used to build the registration user profile
   * @param clientNotes authentication session client notes holding the hints
   * @param excludedAttributes attributes the calling flow never prefills
   * @return the prefill data, empty when there is no usable hint
   */
  static Prefill resolve(
      KeycloakSession session, Map<String, String> clientNotes, Set<String> excludedAttributes) {
    Map<String, String> hints;
    try {
      hints = extractHints(clientNotes);
    } catch (IllegalArgumentException invalidHints) {
      return Prefill.EMPTY;
    }

    if (hints.isEmpty()) {
      return Prefill.EMPTY;
    }

    MultivaluedMap<String, String> candidateFormData = new MultivaluedHashMap<>();
    hints.forEach(candidateFormData::putSingle);
    Attributes attributes =
        session
            .getProvider(UserProfileProvider.class)
            .create(UserProfileContext.REGISTRATION, candidateFormData)
            .getAttributes();
    MultivaluedMap<String, String> writableHints =
        filterWritableHints(hints, attributes, excludedAttributes);

    return new Prefill(writableHints, filterLockedHints(writableHints, attributes));
  }

  static Map<String, String> extractHints(Map<String, String> clientNotes) {
    Map<String, String> hints = new LinkedHashMap<>();

    for (Map.Entry<String, String> clientNote : clientNotes.entrySet()) {
      if (!clientNote.getKey().startsWith(CLIENT_NOTE_PREFIX)) {
        continue;
      }

      String attributeName = clientNote.getKey().substring(CLIENT_NOTE_PREFIX.length());
      String value = clientNote.getValue();
      if (hints.size() == MAX_HINT_COUNT
          || attributeName.isEmpty()
          || attributeName.length() > MAX_HINT_NAME_LENGTH
          || !HINT_NAME_PATTERN.matcher(attributeName).matches()
          || value == null
          || value.isBlank()
          || value.length() > MAX_HINT_VALUE_LENGTH) {
        throw new IllegalArgumentException("Invalid login hint parameters");
      }

      hints.put(attributeName, value);
    }

    return Map.copyOf(hints);
  }

  static MultivaluedMap<String, String> filterWritableHints(
      Map<String, String> hints, Attributes attributes, Set<String> excludedAttributes) {
    MultivaluedMap<String, String> filteredHints = new MultivaluedHashMap<>();
    Map<String, ?> writableAttributes = attributes.getWritable();
    Map<String, ?> unmanagedAttributes = attributes.getUnmanagedAttributes();

    hints.forEach(
        (attributeName, value) -> {
          AttributeMetadata metadata = attributes.getMetadata(attributeName);
          if (!CREDENTIAL_FIELDS.contains(attributeName)
              && !excludedAttributes.contains(attributeName)
              && writableAttributes.containsKey(attributeName)
              && !unmanagedAttributes.containsKey(attributeName)
              && metadata != null
              && !isHidden(metadata)
              && policyFor(metadata) != AttributePrefillPolicy.IGNORE) {
            filteredHints.putSingle(attributeName, value);
          }
        });

    return filteredHints;
  }

  /**
   * Returns the prefilled attributes the voter is not allowed to change.
   *
   * @param writableHints hints already filtered by {@link #filterWritableHints}
   * @param attributes user profile attributes backing the form
   * @return names of the attributes annotated as {@link AttributePrefillPolicy#READ_ONLY}
   */
  static Set<String> filterLockedHints(
      MultivaluedMap<String, String> writableHints, Attributes attributes) {
    Set<String> lockedAttributes = new LinkedHashSet<>();

    writableHints.forEach(
        (attributeName, values) -> {
          AttributeMetadata metadata = attributes.getMetadata(attributeName);
          if (metadata != null && policyFor(metadata) == AttributePrefillPolicy.READ_ONLY) {
            lockedAttributes.add(attributeName);
          }
        });

    return Collections.unmodifiableSet(lockedAttributes);
  }

  /**
   * Returns the locked attributes whose submitted value does not match the login hint. Rendering
   * the field read-only is only a browser affordance, so the submitted value is checked as well.
   *
   * @param writableHints hints already filtered by {@link #filterWritableHints}
   * @param lockedAttributes attributes returned by {@link #filterLockedHints}
   * @param formData submitted form parameters
   * @return names of the locked attributes that were submitted with another value
   */
  static Set<String> findModifiedLockedHints(
      MultivaluedMap<String, String> writableHints,
      Set<String> lockedAttributes,
      MultivaluedMap<String, String> formData) {
    Set<String> modifiedAttributes = new LinkedHashSet<>();

    for (String attributeName : lockedAttributes) {
      String hintValue = writableHints.getFirst(attributeName);
      String submittedValue = formData == null ? null : formData.getFirst(attributeName);
      if (hintValue != null && !hintValue.equals(submittedValue)) {
        modifiedAttributes.add(attributeName);
      }
    }

    return Collections.unmodifiableSet(modifiedAttributes);
  }

  /**
   * Returns a copy of the submitted form data with the locked attributes restored to their login
   * hint value, so a redisplayed form does not keep a rejected value in a field the voter can no
   * longer edit.
   *
   * @param formData submitted form parameters
   * @param writableHints hints already filtered by {@link #filterWritableHints}
   * @param modifiedAttributes attributes returned by {@link #findModifiedLockedHints}
   * @return the corrected form data
   */
  static MultivaluedMap<String, String> restoreLockedHints(
      MultivaluedMap<String, String> formData,
      MultivaluedMap<String, String> writableHints,
      Set<String> modifiedAttributes) {
    MultivaluedMap<String, String> correctedFormData = new MultivaluedHashMap<>(formData);
    modifiedAttributes.forEach(
        attributeName ->
            correctedFormData.putSingle(attributeName, writableHints.getFirst(attributeName)));

    return correctedFormData;
  }

  private static AttributePrefillPolicy policyFor(AttributeMetadata metadata) {
    Map<String, Object> annotations = metadata.getAnnotations();
    Object policy = annotations == null ? null : annotations.get(PREFILL_POLICY_ANNOTATION);

    if (policy == null) {
      return AttributePrefillPolicy.EDITABLE;
    }

    return AttributePrefillPolicy.parse(String.valueOf(policy));
  }

  private static boolean isHidden(AttributeMetadata metadata) {
    Map<String, Object> annotations = metadata.getAnnotations();
    return annotations != null && HIDDEN_INPUT_TYPE.equals(annotations.get(INPUT_TYPE_ANNOTATION));
  }
}
