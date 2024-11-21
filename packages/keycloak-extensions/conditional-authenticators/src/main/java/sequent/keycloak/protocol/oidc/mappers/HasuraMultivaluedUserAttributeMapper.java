// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.protocol.oidc.mappers;

import java.util.ArrayList;
import java.util.Collection;
import java.util.List;
import java.util.stream.Collectors;
import org.keycloak.models.ProtocolMapperModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.UserSessionModel;
import org.keycloak.models.utils.KeycloakModelUtils;
import org.keycloak.protocol.ProtocolMapperUtils;
import org.keycloak.protocol.oidc.mappers.AbstractOIDCProtocolMapper;
import org.keycloak.protocol.oidc.mappers.OIDCAccessTokenMapper;
import org.keycloak.protocol.oidc.mappers.OIDCAttributeMapperHelper;
import org.keycloak.protocol.oidc.mappers.OIDCIDTokenMapper;
import org.keycloak.protocol.oidc.mappers.TokenIntrospectionTokenMapper;
import org.keycloak.protocol.oidc.mappers.UserInfoTokenMapper;
import org.keycloak.provider.ProviderConfigProperty;
import org.keycloak.representations.IDToken;

/**
 * Mappings UserModel.attribute to an ID Token claim. Token claim name can be a full qualified
 * nested object name, i.e. "address.country". This will create a nested json object within the
 * token claim.
 *
 * @author <a href="mailto:bill@burkecentral.com">Bill Burke</a>
 * @version $Revision: 1 $
 */
public class HasuraMultivaluedUserAttributeMapper extends AbstractOIDCProtocolMapper
    implements OIDCAccessTokenMapper,
        OIDCIDTokenMapper,
        UserInfoTokenMapper,
        TokenIntrospectionTokenMapper {

  private static final List<ProviderConfigProperty> configProperties =
      new ArrayList<ProviderConfigProperty>();

  static {
    ProviderConfigProperty property;
    property = new ProviderConfigProperty();
    property.setName(ProtocolMapperUtils.USER_ATTRIBUTE);
    property.setLabel(ProtocolMapperUtils.USER_MODEL_ATTRIBUTE_LABEL);
    property.setHelpText(ProtocolMapperUtils.USER_MODEL_ATTRIBUTE_HELP_TEXT);
    property.setType(ProviderConfigProperty.USER_PROFILE_ATTRIBUTE_LIST_TYPE);
    configProperties.add(property);
    OIDCAttributeMapperHelper.addAttributeConfig(
        configProperties, HasuraMultivaluedUserAttributeMapper.class);
  }

  public static final String PROVIDER_ID = "hasura-multivalued-oidc-usermodel-attribute-mapper";

  public List<ProviderConfigProperty> getConfigProperties() {
    return configProperties;
  }

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  @Override
  public String getDisplayType() {
    return "Hasura Multivalue User Attribute";
  }

  @Override
  public String getDisplayCategory() {
    return TOKEN_MAPPER_CATEGORY;
  }

  @Override
  public String getHelpText() {
    return "Map a custom user multivalue attribute to a token claim, compatible with hasura.";
  }

  protected void setClaim(
      IDToken token, ProtocolMapperModel mappingModel, UserSessionModel userSession) {

    UserModel user = userSession.getUser();
    String attributeName = mappingModel.getConfig().get(ProtocolMapperUtils.USER_ATTRIBUTE);
    boolean aggregateAttrs =
        Boolean.valueOf(mappingModel.getConfig().get(ProtocolMapperUtils.AGGREGATE_ATTRS));
    Collection<String> attributeValue =
        KeycloakModelUtils.resolveAttribute(user, attributeName, aggregateAttrs);
    if (attributeValue == null) return;

    // Format the collection as a string
    String result =
        attributeValue.stream()
            .map(s -> "\"" + s + "\"") // Add double quotes around each string
            .collect(
                Collectors.joining(
                    ", ", "{", "}")); // Join with commas and wrap with curly brackets
    OIDCAttributeMapperHelper.mapClaim(token, mappingModel, result);
  }

  public static ProtocolMapperModel createClaimMapper(
      String name,
      String userAttribute,
      String tokenClaimName,
      String claimType,
      boolean accessToken,
      boolean idToken,
      boolean introspectionEndpoint,
      boolean multivalued) {
    return createClaimMapper(
        name,
        userAttribute,
        tokenClaimName,
        claimType,
        accessToken,
        idToken,
        introspectionEndpoint,
        multivalued,
        false);
  }

  public static ProtocolMapperModel createClaimMapper(
      String name,
      String userAttribute,
      String tokenClaimName,
      String claimType,
      boolean accessToken,
      boolean idToken,
      boolean introspectionEndpoint,
      boolean multivalued,
      boolean aggregateAttrs) {
    ProtocolMapperModel mapper =
        OIDCAttributeMapperHelper.createClaimMapper(
            name,
            userAttribute,
            tokenClaimName,
            claimType,
            accessToken,
            idToken,
            introspectionEndpoint,
            PROVIDER_ID);

    if (multivalued) {
      mapper.getConfig().put(ProtocolMapperUtils.MULTIVALUED, "true");
    }
    if (aggregateAttrs) {
      mapper.getConfig().put(ProtocolMapperUtils.AGGREGATE_ATTRS, "true");
    }

    return mapper;
  }

  public static ProtocolMapperModel createClaimMapper(
      String name,
      String userAttribute,
      String tokenClaimName,
      String claimType,
      boolean accessToken,
      boolean idToken,
      boolean introspectionEndpoint) {
    return createClaimMapper(
        name,
        userAttribute,
        tokenClaimName,
        claimType,
        accessToken,
        idToken,
        introspectionEndpoint,
        false,
        false);
  }
}
