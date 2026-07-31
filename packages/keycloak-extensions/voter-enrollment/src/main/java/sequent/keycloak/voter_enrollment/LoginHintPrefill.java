// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import jakarta.ws.rs.core.MultivaluedHashMap;
import jakarta.ws.rs.core.MultivaluedMap;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.function.Supplier;
import java.util.regex.Pattern;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.authentication.AuthenticationFlowException;
import org.keycloak.authentication.forms.RegistrationPage;
import org.keycloak.forms.login.LoginFormsProvider;
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
  private static final String HIDDEN_ANNOTATION = "hidden";
  private static final String HIDDEN_INPUT_TYPE = "hidden";
  private static final String HTML_ATTRIBUTE_PREFIX = "html-attribute:";
  private static final Set<String> PASSWORD_INPUT_TYPES = Set.of("password", "html5-password");
  private static final Set<String> NON_PREFILLABLE_HTML_ATTRIBUTES =
      Set.of("disabled", "hidden", "inert", "readonly");
  private static final Pattern HIDING_STYLE_PATTERN =
      Pattern.compile(
          "display\\s*:\\s*none"
              + "|visibility\\s*:\\s*(?:hidden|collapse)"
              + "|opacity\\s*:\\s*0(?!\\.[0-9]*[1-9])",
          Pattern.CASE_INSENSITIVE);
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
        return valueOf(value.trim().toUpperCase(Locale.ROOT));
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

  /** Distinguishes an absent hint set from a validated set and an invalid stored set. */
  sealed interface HintResolution {
    record None() implements HintResolution {}

    record Valid(Prefill prefill) implements HintResolution {}

    record Invalid() implements HintResolution {}
  }

  private LoginHintPrefill() {}

  /**
   * Resolves which registration fields the validated login hints may prefill, and which of those
   * the voter is not allowed to change.
   *
   * @param session session used to build the registration user profile
   * @param clientNotes authentication session client notes holding the hints
   * @param excludedAttributes attributes the calling flow never prefills
   * @return a typed result that never turns invalid stored hints into an ordinary empty form
   */
  static HintResolution resolve(
      KeycloakSession session, Map<String, String> clientNotes, Set<String> excludedAttributes) {
    Map<String, String> hints;
    try {
      hints = extractHints(clientNotes);
    } catch (IllegalArgumentException invalidHints) {
      return new HintResolution.Invalid();
    }

    if (hints.isEmpty()) {
      return new HintResolution.None();
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

    return new HintResolution.Valid(
        new Prefill(writableHints, filterLockedHints(writableHints, attributes)));
  }

  /**
   * Converts a typed resolution into prefill data, aborting the flow with a generic 400 page when
   * invalid client notes reach a registration action.
   */
  static Prefill requireValid(
      HintResolution resolution, Supplier<LoginFormsProvider> errorFormSupplier) {
    if (resolution instanceof HintResolution.Invalid) {
      LoginFormsProvider form = errorFormSupplier.get();
      throw new AuthenticationFlowException(
          AuthenticationFlowError.GENERIC_AUTHENTICATION_ERROR,
          form.setError(LoginHintAuthorizationRequestFilter.INVALID_REQUEST_DESCRIPTION)
              .createErrorPage(jakarta.ws.rs.core.Response.Status.BAD_REQUEST));
    }
    if (resolution instanceof HintResolution.Valid valid) {
      return valid.prefill();
    }
    return Prefill.EMPTY;
  }

  static Map<String, String> extractHints(Map<String, String> clientNotes) {
    Map<String, String> hints = new LinkedHashMap<>();

    for (Map.Entry<String, String> clientNote : clientNotes.entrySet()) {
      if (!clientNote.getKey().startsWith(CLIENT_NOTE_PREFIX)) {
        continue;
      }

      String attributeName = clientNote.getKey().substring(CLIENT_NOTE_PREFIX.length());
      String value = clientNote.getValue();
      if (hints.size() == MAX_HINT_COUNT || !isValidHint(attributeName, value)) {
        throw new IllegalArgumentException("Invalid login hint parameters");
      }

      hints.put(attributeName, value);
    }

    return Map.copyOf(hints);
  }

  static boolean isValidHint(String attributeName, String value) {
    return !attributeName.isEmpty()
        && attributeName.length() <= MAX_HINT_NAME_LENGTH
        && HINT_NAME_PATTERN.matcher(attributeName).matches()
        && value != null
        && !isBlankHintValue(value)
        && value.length() <= MAX_HINT_VALUE_LENGTH;
  }

  private static boolean isBlankHintValue(String value) {
    return value
        .codePoints()
        .allMatch(
            codePoint ->
                Character.isWhitespace(codePoint)
                    || Character.isSpaceChar(codePoint)
                    || codePoint == 0xFEFF);
  }

  static MultivaluedMap<String, String> filterWritableHints(
      Map<String, String> hints, Attributes attributes, Set<String> excludedAttributes) {
    MultivaluedMap<String, String> filteredHints = new MultivaluedHashMap<>();
    Map<String, ?> writableAttributes = attributes.getWritable();
    Map<String, ?> unmanagedAttributes = attributes.getUnmanagedAttributes();

    hints.forEach(
        (attributeName, value) -> {
          String rejection =
              rejectionReason(
                  attributeName,
                  attributes.getMetadata(attributeName),
                  writableAttributes,
                  unmanagedAttributes,
                  excludedAttributes);
          if (rejection != null) {
            // Attribute names and profile configuration are safe to log; hint values never are.
            // Without this line a misconfigured attribute silently stops prefilling.
            log.debugv("Not prefilling login hint attribute {0}: {1}", attributeName, rejection);
            return;
          }

          filteredHints.putSingle(attributeName, value);
        });

    return filteredHints;
  }

  /**
   * Explains why a validated login hint may not prefill its attribute.
   *
   * @param attributeName profile attribute the hint targets
   * @param metadata user profile metadata for the attribute, null when the attribute is unknown
   * @param writableAttributes attributes the registration profile allows writing
   * @param unmanagedAttributes attributes outside the declarative profile
   * @param excludedAttributes attributes the calling flow never prefills
   * @return the reason, or null when the attribute may be prefilled
   */
  private static String rejectionReason(
      String attributeName,
      AttributeMetadata metadata,
      Map<String, ?> writableAttributes,
      Map<String, ?> unmanagedAttributes,
      Set<String> excludedAttributes) {
    if (CREDENTIAL_FIELDS.contains(attributeName)) {
      return "it is a credential field";
    }
    if (excludedAttributes.contains(attributeName)) {
      return "the registration flow excludes it";
    }
    if (!writableAttributes.containsKey(attributeName)) {
      return "it is not writable in the registration user profile";
    }
    if (unmanagedAttributes.containsKey(attributeName)) {
      return "it is an unmanaged attribute";
    }
    if (metadata == null) {
      return "it has no user profile metadata";
    }

    String hiddenOrCredentialInput = hiddenOrCredentialInputReason(metadata);
    if (hiddenOrCredentialInput != null) {
      return hiddenOrCredentialInput;
    }
    if (policyFor(metadata) == AttributePrefillPolicy.IGNORE) {
      return PREFILL_POLICY_ANNOTATION + " is IGNORE";
    }

    return null;
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
      java.util.List<String> submittedValues =
          formData == null ? null : formData.get(attributeName);
      if (hintValue != null
          && (submittedValues == null
              || submittedValues.size() != 1
              || !hintValue.equals(submittedValues.get(0)))) {
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

  private static String hiddenOrCredentialInputReason(AttributeMetadata metadata) {
    Map<String, Object> annotations = metadata.getAnnotations();
    if (annotations == null) {
      return null;
    }

    if (Boolean.parseBoolean(String.valueOf(annotations.get(HIDDEN_ANNOTATION)))) {
      return "it is annotated as hidden";
    }

    // The Sequent registration theme emits html-attribute:* annotations verbatim. Boolean
    // disabled/hidden/readonly/inert attributes are active merely by being present, and an inline
    // style can hide the control outright — in both cases a voter would never see the prefilled
    // value, so fail closed. Purely cosmetic annotations such as html-attribute:class are left
    // alone: styling a field is routine and must not silently disable prefill.
    for (Map.Entry<String, Object> annotation : annotations.entrySet()) {
      if (!annotation.getKey().startsWith(HTML_ATTRIBUTE_PREFIX)) {
        continue;
      }
      String htmlAttribute =
          annotation
              .getKey()
              .substring(HTML_ATTRIBUTE_PREFIX.length())
              .trim()
              .toLowerCase(Locale.ROOT);
      String annotationValue = String.valueOf(annotation.getValue());

      if (NON_PREFILLABLE_HTML_ATTRIBUTES.contains(htmlAttribute)) {
        return "the profile sets " + HTML_ATTRIBUTE_PREFIX + htmlAttribute;
      }
      if ("aria-hidden".equals(htmlAttribute) && Boolean.parseBoolean(annotationValue)) {
        return "the profile sets " + HTML_ATTRIBUTE_PREFIX + "aria-hidden";
      }
      if ("style".equals(htmlAttribute) && HIDING_STYLE_PATTERN.matcher(annotationValue).find()) {
        return HTML_ATTRIBUTE_PREFIX + "style hides the field";
      }
      if ("type".equals(htmlAttribute)) {
        String inputTypeReason = inputTypeReason(annotationValue, HTML_ATTRIBUTE_PREFIX + "type");
        if (inputTypeReason != null) {
          return inputTypeReason;
        }
      }
    }

    Object inputType = annotations.get(INPUT_TYPE_ANNOTATION);
    return inputType == null
        ? null
        : inputTypeReason(String.valueOf(inputType), INPUT_TYPE_ANNOTATION);
  }

  private static String inputTypeReason(String rawInputType, String source) {
    String inputType = rawInputType.trim().toLowerCase(Locale.ROOT);
    if (HIDDEN_INPUT_TYPE.equals(inputType)) {
      return source + " renders it as a hidden input";
    }
    if (PASSWORD_INPUT_TYPES.contains(inputType)) {
      return source + " renders it as a password input";
    }
    return null;
  }
}
