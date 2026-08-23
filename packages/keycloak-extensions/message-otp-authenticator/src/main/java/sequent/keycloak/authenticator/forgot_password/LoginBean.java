// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.UserModel;
import org.keycloak.userprofile.AttributeMetadata;
import org.keycloak.userprofile.AttributeValidatorMetadata;
import org.keycloak.userprofile.Attributes;
import org.keycloak.userprofile.UserProfile;
import org.keycloak.userprofile.UserProfileContext;
import org.keycloak.userprofile.UserProfileProvider;

/**
 * User Profile rendering metadata for the fields explicitly selected by {@code matchAttributes}.
 *
 * <p>Keycloak's standard form bean starts from {@link Attributes#getReadable()}, which hides
 * attributes the anonymous voter cannot view. For this pre-authentication form, selecting an
 * attribute in {@code matchAttributes} is the administrator's explicit instruction to ask for it,
 * so this bean looks up only those declarations directly. It never reads stored values, submitted
 * values, defaults, or arbitrary executable annotations into the anonymous response.
 */
public final class LoginBean {

  private static final Set<String> SAFE_ANNOTATIONS =
      Set.of(
          "inputType",
          "inputHelperTextBefore",
          "inputHelperTextAfter",
          "inputTypePlaceholder",
          "inputTypePattern",
          "inputTypeSize",
          "inputTypeMaxlength",
          "inputTypeMinlength",
          "inputTypeMax",
          "inputTypeMin",
          "inputTypeStep",
          "inputTypeCols",
          "inputTypeRows",
          "inputOptionLabels",
          "inputOptionLabelsI18nPrefix",
          "inputOptionsFromValidation",
          "filterSelectAttribute");

  private final Map<String, Attribute> attributesByName;

  public LoginBean(KeycloakSession session, List<String> matchAttributes) {
    UserProfileProvider provider =
        session == null ? null : session.getProvider(UserProfileProvider.class);
    UserProfile profile =
        provider == null
            ? null
            : provider.create(UserProfileContext.REGISTRATION, null, (UserModel) null);
    Attributes profileAttributes = profile == null ? null : profile.getAttributes();
    if (profileAttributes == null || matchAttributes == null || matchAttributes.isEmpty()) {
      attributesByName = Map.of();
      return;
    }

    Set<String> selectedNames = new LinkedHashSet<>(matchAttributes);
    Map<String, Attribute> selected = new LinkedHashMap<>();
    for (String name : matchAttributes) {
      if (name == null || selected.containsKey(name)) {
        continue;
      }
      AttributeMetadata metadata = profileAttributes.getMetadata(name);
      if (metadata != null) {
        selected.put(name, new Attribute(metadata, profileAttributes, selectedNames));
      }
    }
    attributesByName = Collections.unmodifiableMap(selected);
  }

  public Map<String, Attribute> getAttributesByName() {
    return attributesByName;
  }

  /** FreeMarker-facing rendering metadata for one selected match attribute. */
  public static final class Attribute {
    private final String name;
    private final String displayName;
    private final Attributes profileAttributes;
    private final Map<String, Object> annotations;
    private final Map<String, Map<String, Object>> validators;

    private Attribute(
        AttributeMetadata metadata, Attributes profileAttributes, Set<String> selectedNames) {
      name = metadata.getName();
      String configuredDisplayName = metadata.getAttributeDisplayName();
      displayName =
          configuredDisplayName == null || configuredDisplayName.isBlank()
              ? name
              : configuredDisplayName;
      this.profileAttributes = profileAttributes;
      annotations = safeAnnotations(metadata.getAnnotations(), selectedNames);
      validators = safeValidators(metadata.getValidators(), annotations);
    }

    public String getName() {
      return name;
    }

    public String getDisplayName() {
      return displayName;
    }

    public Map<String, Object> getAnnotations() {
      return annotations;
    }

    public Map<String, Map<String, Object>> getValidators() {
      return validators;
    }

    public List<String> getValues() {
      return List.of();
    }

    public String getValue() {
      return "";
    }

    public boolean isRequired() {
      return profileAttributes.isRequired(name);
    }

    public boolean isMultivalued() {
      return false;
    }

    public boolean isReadOnly() {
      return false;
    }

    public Map<String, Object> getHtml5DataAnnotations() {
      return Map.of();
    }
  }

  private static Map<String, Object> safeAnnotations(
      Map<String, Object> configured, Set<String> selectedNames) {
    if (configured == null || configured.isEmpty()) {
      return Map.of();
    }

    Map<String, Object> safe = new LinkedHashMap<>();
    for (Map.Entry<String, Object> entry : configured.entrySet()) {
      if (!SAFE_ANNOTATIONS.contains(entry.getKey()) || entry.getValue() == null) {
        continue;
      }
      if ("filterSelectAttribute".equals(entry.getKey())
          && (!(entry.getValue() instanceof String target) || !selectedNames.contains(target))) {
        continue;
      }

      Object value = entry.getValue();
      if ("inputType".equals(entry.getKey())) {
        if ("multiselect".equals(value)) {
          value = "select";
        } else if ("multiselect-checkboxes".equals(value)) {
          value = "select-radiobuttons";
        }
      }
      safe.put(entry.getKey(), value);
    }
    return Collections.unmodifiableMap(safe);
  }

  private static Map<String, Map<String, Object>> safeValidators(
      List<AttributeValidatorMetadata> configured, Map<String, Object> annotations) {
    if (configured == null || configured.isEmpty()) {
      return Map.of();
    }

    Set<String> exposedNames = new LinkedHashSet<>();
    exposedNames.add("options");
    if (annotations.get("inputOptionsFromValidation") instanceof String configuredName) {
      exposedNames.add(configuredName);
    }

    Map<String, Map<String, Object>> safe = new LinkedHashMap<>();
    for (AttributeValidatorMetadata validator : configured) {
      if (validator == null || !exposedNames.contains(validator.getValidatorId())) {
        continue;
      }
      Map<String, Object> config = validator.getValidatorConfig();
      safe.put(
          validator.getValidatorId(),
          config == null ? Map.of() : Collections.unmodifiableMap(new LinkedHashMap<>(config)));
    }
    return Collections.unmodifiableMap(safe);
  }
}
