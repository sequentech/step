// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import jakarta.ws.rs.core.MultivaluedMap;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Set;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.UserModel;
import org.keycloak.representations.userprofile.config.UPAttribute;
import org.keycloak.representations.userprofile.config.UPConfig;
import org.keycloak.userprofile.Attributes;
import org.keycloak.userprofile.UserProfile;
import org.keycloak.userprofile.UserProfileContext;
import org.keycloak.userprofile.UserProfileProvider;

/**
 * A deliberately narrow model of the User Profile fields selected by {@code matchAttributes}.
 *
 * <p>The standard Keycloak form bean starts from {@link Attributes#getReadable()}, which removes
 * attributes whose profile permission does not include {@code view}. That is correct for showing an
 * authenticated user's stored profile, but not for this pre-authentication form: selecting an
 * attribute in {@code matchAttributes} is the administrator's explicit instruction to ask for it.
 * This bean therefore reads declarations from {@link UPConfig}, while never reading a user model or
 * stored attribute values.
 *
 * <p>Only the configured match fields and the rendering metadata used by the login theme are
 * exposed. Values come exclusively from the current form submission, so defaults, unrelated profile
 * attributes and persisted voter data cannot be reflected into an anonymous response.
 */
public class LoginBean {

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
          "filterSelectAttribute",
          "disableAttribute",
          "disableElement",
          "html-attribute:autocomplete");

  private final List<Attribute> attributes;
  private final Map<String, Attribute> attributesByName;

  public LoginBean(
      MultivaluedMap<String, String> formData,
      KeycloakSession session,
      List<String> matchAttributes) {
    UserProfileProvider provider = session.getProvider(UserProfileProvider.class);
    UPConfig configuration = provider == null ? null : provider.getConfiguration();
    if (configuration == null
        || configuration.getAttributes() == null
        || matchAttributes == null
        || matchAttributes.isEmpty()) {
      attributes = List.of();
      attributesByName = Map.of();
      return;
    }

    Map<String, UPAttribute> configuredByName = new LinkedHashMap<>();
    for (UPAttribute configured : configuration.getAttributes()) {
      if (configured != null && configured.getName() != null) {
        configuredByName.putIfAbsent(configured.getName(), configured);
      }
    }

    UserProfile profile = provider.create(UserProfileContext.REGISTRATION, null, (UserModel) null);
    Attributes profileAttributes = profile == null ? null : profile.getAttributes();
    Set<String> selectedNames = Collections.unmodifiableSet(new LinkedHashSet<>(matchAttributes));
    Map<String, Attribute> selected = new LinkedHashMap<>();
    for (String name : matchAttributes) {
      UPAttribute configured = configuredByName.get(name);
      if (configured == null || selected.containsKey(name)) {
        continue;
      }
      boolean required = profileAttributes != null && profileAttributes.isRequired(name);
      selected.put(name, new Attribute(configured, formData, selectedNames, required));
    }

    attributesByName = Collections.unmodifiableMap(selected);
    attributes = List.copyOf(selected.values());
  }

  public List<Attribute> getAttributes() {
    return attributes;
  }

  public Map<String, Attribute> getAttributesByName() {
    return attributesByName;
  }

  /** The sanitized login fields intentionally expose no arbitrary data-* annotations or scripts. */
  public Map<String, Object> getHtml5DataAnnotations() {
    return Map.of();
  }

  public String getContext() {
    return "MULTI_ATTRIBUTE_LOGIN";
  }

  /** FreeMarker-facing field metadata shaped like Keycloak's User Profile attribute bean. */
  public static final class Attribute {
    private final String name;
    private final String displayName;
    private final Map<String, Object> annotations;
    private final Map<String, Map<String, Object>> validators;
    private final List<String> values;
    private final boolean required;
    private final boolean multivalued;

    private Attribute(
        UPAttribute configured,
        MultivaluedMap<String, String> formData,
        Set<String> selectedNames,
        boolean required) {
      name = configured.getName();
      displayName = loginDisplayName(configured);
      annotations = safeAnnotations(configured.getAnnotations(), selectedNames);
      validators = safeValidators(configured.getValidations(), annotations);
      List<String> submitted = formData == null ? null : formData.get(name);
      values = submitted == null ? List.of() : submitted.stream().filter(Objects::nonNull).toList();
      this.required = required;
      multivalued = configured.isMultivalued();
    }

    private static String loginDisplayName(UPAttribute configured) {
      String configuredDisplayName = configured.getDisplayName();
      if (configuredDisplayName == null || configuredDisplayName.isBlank()) {
        return configured.getName();
      }

      // Keycloak's Admin Console generates this namespaced key for custom profile attributes. The
      // extension's theme-resource bundles intentionally use the attribute name itself, matching
      // the rest of STEP's profile rendering. Keep every other literal or custom message key as-is.
      String keycloakGeneratedKey = "${profile.attributes." + configured.getName() + "}";
      if (keycloakGeneratedKey.equals(configuredDisplayName)) {
        return "${" + configured.getName() + "}";
      }
      return configuredDisplayName;
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
        safe.put(entry.getKey(), entry.getValue());
      }
      return Collections.unmodifiableMap(safe);
    }

    private static Map<String, Map<String, Object>> safeValidators(
        Map<String, Map<String, Object>> configured, Map<String, Object> annotations) {
      if (configured == null || configured.isEmpty()) {
        return Map.of();
      }
      Set<String> exposedValidatorNames = new LinkedHashSet<>();
      exposedValidatorNames.add("options");
      if (annotations.get("inputOptionsFromValidation") instanceof String configuredName) {
        exposedValidatorNames.add(configuredName);
      }
      Map<String, Map<String, Object>> safe = new LinkedHashMap<>();
      for (String name : exposedValidatorNames) {
        Map<String, Object> validator = configured.get(name);
        if (validator != null) {
          safe.put(name, Collections.unmodifiableMap(new LinkedHashMap<>(validator)));
        }
      }
      return Collections.unmodifiableMap(safe);
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
      return values;
    }

    public String getValue() {
      return values.isEmpty() ? "" : values.get(0);
    }

    public boolean isRequired() {
      return required;
    }

    public boolean isMultivalued() {
      return multivalued;
    }

    /** Login collects a matching value; profile edit permissions must not disable its input. */
    public boolean isReadOnly() {
      return false;
    }

    /** Arbitrary data-* annotations are intentionally not exposed on an anonymous login page. */
    public Map<String, Object> getHtml5DataAnnotations() {
      return Map.of();
    }
  }
}
