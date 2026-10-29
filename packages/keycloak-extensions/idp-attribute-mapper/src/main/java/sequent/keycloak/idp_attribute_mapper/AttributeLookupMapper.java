// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.idp_attribute_mapper;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.Response;
import java.util.Arrays;
import java.util.Collection;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.concurrent.ConcurrentHashMap;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.broker.oidc.KeycloakOIDCIdentityProviderFactory;
import org.keycloak.broker.oidc.OIDCIdentityProviderFactory;
import org.keycloak.broker.oidc.mappers.AbstractClaimMapper;
import org.keycloak.broker.provider.AbstractIdentityProviderMapper;
import org.keycloak.broker.provider.BrokeredIdentityContext;
import org.keycloak.broker.provider.IdentityProviderMapper;
import org.keycloak.broker.saml.SAMLEndpoint;
import org.keycloak.broker.saml.SAMLIdentityProviderFactory;
import org.keycloak.dom.saml.v2.assertion.AssertionType;
import org.keycloak.dom.saml.v2.assertion.AttributeType;
import org.keycloak.dom.saml.v2.assertion.NameIDType;
import org.keycloak.dom.saml.v2.assertion.SubjectType;
import org.keycloak.models.IdentityProviderMapperModel;
import org.keycloak.models.IdentityProviderSyncMode;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.services.ErrorPageException;
import sequent.keycloak.idp_attribute_mapper.MapperSettings.MissPolicy;
import sequent.keycloak.idp_attribute_mapper.MapperSettings.Resolution;
import sequent.keycloak.idp_attribute_mapper.MapperSettings.Target;

/**
 * Identity provider mapper that reads one or more SAML attributes or OIDC claims, extracts,
 * normalizes and validates them, optionally translates them through a lookup table, and stores the
 * result in a user attribute or the username.
 */
@JBossLog
@AutoService(IdentityProviderMapper.class)
public class AttributeLookupMapper extends AbstractIdentityProviderMapper {

  public static final String PROVIDER_ID = "sequent-attribute-lookup-idp-mapper";

  private static final String[] COMPATIBLE_PROVIDERS = {
    SAMLIdentityProviderFactory.PROVIDER_ID,
    OIDCIdentityProviderFactory.PROVIDER_ID,
    KeycloakOIDCIdentityProviderFactory.PROVIDER_ID
  };

  private static final String RESOLUTION_KEY_PREFIX = PROVIDER_ID + ".";

  private static final String EMAIL = "email";
  private static final String FIRST_NAME = "firstName";
  private static final String LAST_NAME = "lastName";

  private static final List<ProviderConfigProperty> CONFIG_PROPERTIES = buildConfigProperties();

  private record CachedSettings(
      Map<String, String> config, MapperSettings settings, RuntimeException error) {}

  private static final Map<String, CachedSettings> SETTINGS_CACHE = new ConcurrentHashMap<>();

  private static List<ProviderConfigProperty> buildConfigProperties() {
    return List.of(
        property(
            MapperSettings.SOURCES,
            "Sources",
            "SAML attribute names (or friendly names) or OIDC claims to read, in order. Use '"
                + MapperSettings.SOURCE_NAME_ID
                + "' for the SAML NameID and '"
                + MapperSettings.SOURCE_BROKER_ID
                + "' for the brokered user id. The first value of each source is used.",
            ProviderConfigProperty.MULTIVALUED_STRING_TYPE,
            null,
            null),
        property(
            MapperSettings.EXTRACT_REGEX,
            "Extract regex",
            "Optional. Applied to each source value; the first capture group (or the whole match"
                + " if there is no group) is kept. A value that does not match is a miss.",
            ProviderConfigProperty.STRING_TYPE,
            null,
            null),
        property(
            MapperSettings.SEPARATOR,
            "Separator",
            "Joins the values when there are several sources.",
            ProviderConfigProperty.STRING_TYPE,
            MapperSettings.DEFAULT_SEPARATOR,
            null),
        property(
            MapperSettings.CASE,
            "Case",
            "Case applied to each value after trimming it. Lookup table keys get the same case.",
            ProviderConfigProperty.LIST_TYPE,
            MapperSettings.CaseMode.NONE.name(),
            names(MapperSettings.CaseMode.values())),
        property(
            MapperSettings.VALIDATION_REGEX,
            "Validation regex",
            "Optional. The joined value must match it entirely, before the lookup; otherwise it is"
                + " a miss.",
            ProviderConfigProperty.STRING_TYPE,
            null,
            null),
        property(
            MapperSettings.LOOKUP_TABLE,
            "Lookup table",
            "Optional. One row per line: '<value> = <key>, <key>, <from>..<to>' or '<key>,<value>'."
                + " Ranges apply to the last run of digits. A key with no row is a miss. Without a"
                + " table, the joined value is stored.",
            ProviderConfigProperty.TEXT_TYPE,
            null,
            null),
        property(
            MapperSettings.TARGET,
            "Target",
            "USER_ATTRIBUTE stores the value in the attribute below. USERNAME sets the username of"
                + " users created on first login.",
            ProviderConfigProperty.LIST_TYPE,
            Target.USER_ATTRIBUTE.name(),
            names(Target.values())),
        property(
            MapperSettings.TARGET_ATTRIBUTE,
            "Target attribute",
            "User attribute to store the value in. 'email', 'firstName' and 'lastName' set those"
                + " user properties.",
            ProviderConfigProperty.STRING_TYPE,
            null,
            null),
        property(
            MapperSettings.ON_MISS,
            "On miss",
            "What to do when a source is missing, extraction or validation fails, or the table has"
                + " no row. UNSET leaves the target without a value (and removes it on update)."
                + " DEFAULT stores the default value. REJECT stops the login with the error"
                + " message.",
            ProviderConfigProperty.LIST_TYPE,
            MissPolicy.UNSET.name(),
            names(MissPolicy.values())),
        property(
            MapperSettings.DEFAULT_VALUE,
            "Default value",
            "Value stored on a miss when 'On miss' is DEFAULT.",
            ProviderConfigProperty.STRING_TYPE,
            null,
            null),
        property(
            MapperSettings.ERROR_MESSAGE,
            "Error message",
            "Message key or text shown when 'On miss' is REJECT.",
            ProviderConfigProperty.STRING_TYPE,
            MapperSettings.DEFAULT_ERROR_MESSAGE,
            null));
  }

  private static ProviderConfigProperty property(
      String name,
      String label,
      String helpText,
      String type,
      String defaultValue,
      List<String> options) {
    ProviderConfigProperty property = new ProviderConfigProperty();
    property.setName(name);
    property.setLabel(label);
    property.setHelpText(helpText);
    property.setType(type);
    if (defaultValue != null) {
      property.setDefaultValue(defaultValue);
    }
    if (options != null) {
      property.setOptions(options);
    }
    return property;
  }

  private static List<String> names(Enum<?>[] values) {
    return Arrays.stream(values).map(Enum::name).toList();
  }

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  @Override
  public String[] getCompatibleProviders() {
    return COMPATIBLE_PROVIDERS;
  }

  @Override
  public String getDisplayCategory() {
    return "Attribute Importer";
  }

  @Override
  public String getDisplayType() {
    return "Attribute Transform and Lookup";
  }

  @Override
  public String getHelpText() {
    return "Reads SAML attributes or OIDC claims, extracts, normalizes and validates them,"
        + " optionally translates them through a lookup table, and stores the result in a user"
        + " attribute or the username.";
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    return CONFIG_PROPERTIES;
  }

  @Override
  public boolean supportsSyncMode(IdentityProviderSyncMode syncMode) {
    return true;
  }

  static MapperSettings settings(IdentityProviderMapperModel mapperModel) {
    Map<String, String> config =
        mapperModel.getConfig() == null ? Map.of() : mapperModel.getConfig();
    String key =
        mapperModel.getId() != null
            ? mapperModel.getId()
            : mapperModel.getIdentityProviderAlias() + "/" + mapperModel.getName();
    CachedSettings cached = SETTINGS_CACHE.get(key);
    if (cached == null || !cached.config().equals(config)) {
      Map<String, String> copy = new HashMap<>(config);
      long start = System.nanoTime();
      try {
        MapperSettings parsed = MapperSettings.parse(copy);
        cached = new CachedSettings(copy, parsed, null);
        log.infof(
            "Parsed mapper %s: %d table entries in %d ms",
            key,
            parsed.table() == null ? 0 : parsed.table().size(),
            (System.nanoTime() - start) / 1_000_000);
      } catch (RuntimeException e) {
        cached = new CachedSettings(copy, null, e);
        log.errorf("Invalid configuration in mapper %s: %s", key, e.getMessage());
      }
      SETTINGS_CACHE.put(key, cached);
    }
    if (cached.error() != null) {
      throw cached.error();
    }
    return cached.settings();
  }

  static String sourceValue(BrokeredIdentityContext context, String source) {
    if (MapperSettings.SOURCE_BROKER_ID.equals(source)) {
      return context.getId();
    }
    AssertionType assertion =
        (AssertionType) context.getContextData().get(SAMLEndpoint.SAML_ASSERTION);
    if (MapperSettings.SOURCE_NAME_ID.equals(source)) {
      return assertion == null ? null : nameId(assertion);
    }
    if (assertion != null) {
      return samlAttribute(assertion, source);
    }
    return firstValue(AbstractClaimMapper.getClaimValue(context, source));
  }

  private static String nameId(AssertionType assertion) {
    SubjectType subject = assertion.getSubject();
    if (subject == null
        || subject.getSubType() == null
        || !(subject.getSubType().getBaseID() instanceof NameIDType nameId)) {
      return null;
    }
    return nameId.getValue();
  }

  private static String samlAttribute(AssertionType assertion, String name) {
    return assertion.getAttributeStatements().stream()
        .flatMap(statement -> statement.getAttributes().stream())
        .map(choice -> choice.getAttribute())
        .filter(
            (AttributeType attribute) ->
                name.equals(attribute.getName()) || name.equals(attribute.getFriendlyName()))
        .flatMap(attribute -> attribute.getAttributeValue().stream())
        .filter(Objects::nonNull)
        .map(Object::toString)
        .findFirst()
        .orElse(null);
  }

  private static String firstValue(Object value) {
    if (value instanceof Collection<?> collection) {
      return collection.stream()
          .filter(Objects::nonNull)
          .map(Object::toString)
          .findFirst()
          .orElse(null);
    }
    return value == null ? null : value.toString();
  }

  /** Value to store, or {@code null} to leave the target unset. Rejects the login if configured. */
  private String resolve(
      KeycloakSession session,
      IdentityProviderMapperModel mapperModel,
      BrokeredIdentityContext context) {
    MapperSettings settings;
    try {
      settings = settings(mapperModel);
    } catch (RuntimeException e) {
      throw rejection(
          session,
          context,
          Response.Status.INTERNAL_SERVER_ERROR,
          MapperSettings.DEFAULT_ERROR_MESSAGE);
    }
    Resolution resolution = settings.resolve(source -> sourceValue(context, source));
    if (resolution.isHit()) {
      return resolution.value();
    }
    log.debugf(
        "Mapper %s missed (%s) for input '%s'",
        mapperModel.getName(), resolution.missReason(), resolution.input());
    return switch (settings.missPolicy()) {
      case UNSET -> null;
      case DEFAULT -> settings.defaultValue();
      case REJECT -> {
        log.infof(
            "Mapper %s rejected a login from %s: %s",
            mapperModel.getName(), mapperModel.getIdentityProviderAlias(), resolution.missReason());
        throw rejection(session, context, Response.Status.FORBIDDEN, settings.errorMessage());
      }
    };
  }

  /** Builds the exception that stops the login and shows the error page. */
  protected RuntimeException rejection(
      KeycloakSession session,
      BrokeredIdentityContext context,
      Response.Status status,
      String message) {
    return new ErrorPageException(session, context.getAuthenticationSession(), status, message);
  }

  private static String resolutionKey(IdentityProviderMapperModel mapperModel) {
    return RESOLUTION_KEY_PREFIX + mapperModel.getId() + "." + mapperModel.getName();
  }

  /** Runs on every login, before the user is looked up, so it is where a login is rejected. */
  @Override
  public void preprocessFederatedIdentity(
      KeycloakSession session,
      RealmModel realm,
      IdentityProviderMapperModel mapperModel,
      BrokeredIdentityContext context) {
    String value = resolve(session, mapperModel, context);
    // Store a marker for a null value so later phases don't resolve again.
    context
        .getContextData()
        .put(resolutionKey(mapperModel), value == null ? Unset.INSTANCE : value);
    if (value == null) {
      return;
    }
    MapperSettings settings = settings(mapperModel);
    if (settings.target() == Target.USERNAME) {
      context.setModelUsername(value);
      return;
    }
    String attribute = settings.targetAttribute();
    if (attribute.equalsIgnoreCase(EMAIL)) {
      context.setEmail(value);
    } else if (attribute.equalsIgnoreCase(FIRST_NAME)) {
      context.setFirstName(value);
    } else if (attribute.equalsIgnoreCase(LAST_NAME)) {
      context.setLastName(value);
    } else {
      context.setUserAttribute(attribute, value);
    }
  }

  private enum Unset {
    INSTANCE
  }

  private String resolved(
      KeycloakSession session,
      IdentityProviderMapperModel mapperModel,
      BrokeredIdentityContext context) {
    Object stored = context.getContextData().get(resolutionKey(mapperModel));
    if (stored == Unset.INSTANCE) {
      return null;
    }
    if (stored instanceof String value) {
      return value;
    }
    return resolve(session, mapperModel, context);
  }

  @Override
  public void importNewUser(
      KeycloakSession session,
      RealmModel realm,
      UserModel user,
      IdentityProviderMapperModel mapperModel,
      BrokeredIdentityContext context) {
    MapperSettings settings = settings(mapperModel);
    if (settings.target() != Target.USER_ATTRIBUTE) {
      return;
    }
    String value = resolved(session, mapperModel, context);
    if (value != null) {
      write(user, settings.targetAttribute(), value);
    }
  }

  @Override
  public void updateBrokeredUser(
      KeycloakSession session,
      RealmModel realm,
      UserModel user,
      IdentityProviderMapperModel mapperModel,
      BrokeredIdentityContext context) {
    MapperSettings settings = settings(mapperModel);
    if (settings.target() != Target.USER_ATTRIBUTE) {
      return;
    }
    String attribute = settings.targetAttribute();
    String value = resolved(session, mapperModel, context);
    if (value != null) {
      write(user, attribute, value);
    } else if (!isBuiltIn(attribute) && user.getFirstAttribute(attribute) != null) {
      user.removeAttribute(attribute);
    }
  }

  @Override
  public void updateBrokeredUserLegacy(
      KeycloakSession session,
      RealmModel realm,
      UserModel user,
      IdentityProviderMapperModel mapperModel,
      BrokeredIdentityContext context) {
    updateBrokeredUser(session, realm, user, mapperModel, context);
  }

  private static boolean isBuiltIn(String attribute) {
    return attribute.equalsIgnoreCase(EMAIL)
        || attribute.equalsIgnoreCase(FIRST_NAME)
        || attribute.equalsIgnoreCase(LAST_NAME);
  }

  private static void write(UserModel user, String attribute, String value) {
    if (attribute.equalsIgnoreCase(EMAIL)) {
      if (!value.equals(user.getEmail())) {
        user.setEmail(value);
      }
    } else if (attribute.equalsIgnoreCase(FIRST_NAME)) {
      if (!value.equals(user.getFirstName())) {
        user.setFirstName(value);
      }
    } else if (attribute.equalsIgnoreCase(LAST_NAME)) {
      if (!value.equals(user.getLastName())) {
        user.setLastName(value);
      }
    } else if (!List.of(value).equals(user.getAttributes().get(attribute))) {
      user.setSingleAttribute(attribute, value);
    }
  }
}
