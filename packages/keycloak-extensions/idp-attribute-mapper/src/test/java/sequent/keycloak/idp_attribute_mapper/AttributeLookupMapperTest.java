// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.idp_attribute_mapper;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import jakarta.ws.rs.core.Response;
import java.util.GregorianCalendar;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import javax.xml.datatype.DatatypeConfigurationException;
import javax.xml.datatype.DatatypeFactory;
import javax.xml.datatype.XMLGregorianCalendar;
import org.junit.jupiter.api.Test;
import org.keycloak.broker.oidc.OIDCIdentityProvider;
import org.keycloak.broker.provider.BrokeredIdentityContext;
import org.keycloak.broker.saml.SAMLEndpoint;
import org.keycloak.dom.saml.v2.assertion.AssertionType;
import org.keycloak.dom.saml.v2.assertion.AttributeStatementType;
import org.keycloak.dom.saml.v2.assertion.AttributeType;
import org.keycloak.dom.saml.v2.assertion.NameIDType;
import org.keycloak.dom.saml.v2.assertion.SubjectType;
import org.keycloak.models.IdentityProviderMapperModel;
import org.keycloak.models.IdentityProviderModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.representations.IDToken;

class AttributeLookupMapperTest {

  private static final String TABLE = "area-1 = 09:0001..09:0100\narea-2 = 09:0101..09:0200";

  /** Records rejections instead of rendering the error page. */
  static final class Rejected extends RuntimeException {
    final Response.Status status;

    Rejected(Response.Status status, String message) {
      super(message);
      this.status = status;
    }
  }

  private final AttributeLookupMapper mapper =
      new AttributeLookupMapper() {
        @Override
        protected RuntimeException rejection(
            KeycloakSession session,
            BrokeredIdentityContext context,
            Response.Status status,
            String message) {
          return new Rejected(status, message);
        }
      };

  private final KeycloakSession session = mock(KeycloakSession.class);
  private final RealmModel realm = mock(RealmModel.class);

  private static IdentityProviderMapperModel mapperModel(String... pairs) {
    IdentityProviderMapperModel model = new IdentityProviderMapperModel();
    model.setId(UUID.randomUUID().toString());
    model.setName("test");
    model.setIdentityProviderAlias("idp");
    Map<String, String> config = new HashMap<>();
    for (int i = 0; i < pairs.length; i += 2) {
      config.put(pairs[i], pairs[i + 1]);
    }
    model.setConfig(config);
    return model;
  }

  private static IdentityProviderModel idpModel() {
    IdentityProviderModel model = new IdentityProviderModel();
    model.setAlias("idp");
    model.setEnabled(true);
    return model;
  }

  private static XMLGregorianCalendar issueInstant() {
    try {
      return DatatypeFactory.newInstance().newXMLGregorianCalendar(new GregorianCalendar());
    } catch (DatatypeConfigurationException e) {
      throw new IllegalStateException(e);
    }
  }

  private static BrokeredIdentityContext samlContext(String nameId, String... attributes) {
    BrokeredIdentityContext context = new BrokeredIdentityContext(nameId, idpModel());
    AssertionType assertion = new AssertionType("a1", issueInstant());
    SubjectType subject = new SubjectType();
    SubjectType.STSubType subType = new SubjectType.STSubType();
    NameIDType nameIdType = new NameIDType();
    nameIdType.setValue(nameId);
    subType.addBaseID(nameIdType);
    subject.setSubType(subType);
    assertion.setSubject(subject);
    AttributeStatementType statement = new AttributeStatementType();
    for (int i = 0; i < attributes.length; i += 2) {
      AttributeType attribute = new AttributeType("urn:oid:" + attributes[i]);
      attribute.setFriendlyName(attributes[i]);
      attribute.addAttributeValue(attributes[i + 1]);
      statement.addAttribute(new AttributeStatementType.ASTChoiceType(attribute));
    }
    assertion.addStatement(statement);
    context.getContextData().put(SAMLEndpoint.SAML_ASSERTION, assertion);
    return context;
  }

  private static BrokeredIdentityContext oidcContext(String sub, Map<String, Object> claims) {
    BrokeredIdentityContext context = new BrokeredIdentityContext(sub, idpModel());
    IDToken token = new IDToken();
    token.setSubject(sub);
    claims.forEach(token::setOtherClaims);
    context.getContextData().put(OIDCIdentityProvider.VALIDATED_ID_TOKEN, token);
    return context;
  }

  private IdentityProviderMapperModel areaMapper(String onMiss) {
    return mapperModel(
        MapperSettings.SOURCES, "region##precinct",
        MapperSettings.TARGET_ATTRIBUTE, "area-id",
        MapperSettings.LOOKUP_TABLE, TABLE,
        MapperSettings.ON_MISS, onMiss,
        MapperSettings.DEFAULT_VALUE, "area-default",
        MapperSettings.ERROR_MESSAGE, "unknownPrecinct");
  }

  @Test
  void samlNameIdBecomesUsername() {
    IdentityProviderMapperModel model =
        mapperModel(
            MapperSettings.SOURCES, MapperSettings.SOURCE_NAME_ID,
            MapperSettings.EXTRACT_REGEX, "^([^@]+)@",
            MapperSettings.TARGET, "USERNAME",
            MapperSettings.ON_MISS, "REJECT");
    BrokeredIdentityContext context = samlContext("abc-123@example.org");
    mapper.preprocessFederatedIdentity(session, realm, model, context);
    assertEquals("abc-123", context.getModelUsername());

    UserModel user = mock(UserModel.class);
    mapper.updateBrokeredUser(session, realm, user, model, context);
    verify(user, never()).setUsername(anyString());
  }

  @Test
  void samlAttributesMatchByNameOrFriendlyName() {
    BrokeredIdentityContext context = samlContext("id", "region", "09", "precinct", "0150");
    assertEquals("09", AttributeLookupMapper.sourceValue(context, "region"));
    assertEquals("09", AttributeLookupMapper.sourceValue(context, "urn:oid:region"));
    assertNull(AttributeLookupMapper.sourceValue(context, "missing"));
    assertEquals("id", AttributeLookupMapper.sourceValue(context, MapperSettings.SOURCE_NAME_ID));
    assertEquals("id", AttributeLookupMapper.sourceValue(context, MapperSettings.SOURCE_BROKER_ID));
  }

  @Test
  void oidcClaimsAreRead() {
    BrokeredIdentityContext context =
        oidcContext("sub-1", Map.of("region", "09", "precincts", List.of("0150", "0151")));
    assertEquals("09", AttributeLookupMapper.sourceValue(context, "region"));
    assertEquals("0150", AttributeLookupMapper.sourceValue(context, "precincts"));
    assertEquals(
        "sub-1", AttributeLookupMapper.sourceValue(context, MapperSettings.SOURCE_BROKER_ID));
    assertNull(AttributeLookupMapper.sourceValue(context, MapperSettings.SOURCE_NAME_ID));
  }

  @Test
  void lookupSetsAttributeOnImport() {
    IdentityProviderMapperModel model = areaMapper("REJECT");
    BrokeredIdentityContext context = samlContext("id", "region", "09", "precinct", "0150");
    mapper.preprocessFederatedIdentity(session, realm, model, context);
    assertEquals(List.of("area-2"), context.getAttributes().get("area-id"));

    UserModel user = mock(UserModel.class);
    when(user.getAttributes()).thenReturn(Map.of());
    mapper.importNewUser(session, realm, user, model, context);
    verify(user).setSingleAttribute("area-id", "area-2");
  }

  @Test
  void oidcLookupWorksTheSameWay() {
    IdentityProviderMapperModel model = areaMapper("REJECT");
    BrokeredIdentityContext context =
        oidcContext("sub-1", Map.of("region", "09", "precinct", "0001"));
    mapper.preprocessFederatedIdentity(session, realm, model, context);
    UserModel user = mock(UserModel.class);
    when(user.getAttributes()).thenReturn(Map.of());
    mapper.importNewUser(session, realm, user, model, context);
    verify(user).setSingleAttribute("area-id", "area-1");
  }

  @Test
  void rejectStopsLoginOnMiss() {
    IdentityProviderMapperModel model = areaMapper("REJECT");
    BrokeredIdentityContext context = samlContext("id", "region", "09", "precinct", "0999");
    Rejected rejected =
        assertThrows(
            Rejected.class,
            () -> mapper.preprocessFederatedIdentity(session, realm, model, context));
    assertEquals(Response.Status.FORBIDDEN, rejected.status);
    assertEquals("unknownPrecinct", rejected.getMessage());
  }

  @Test
  void defaultIsStoredOnMiss() {
    IdentityProviderMapperModel model = areaMapper("DEFAULT");
    BrokeredIdentityContext context = samlContext("id", "region", "09");
    mapper.preprocessFederatedIdentity(session, realm, model, context);
    UserModel user = mock(UserModel.class);
    when(user.getAttributes()).thenReturn(Map.of());
    mapper.importNewUser(session, realm, user, model, context);
    verify(user).setSingleAttribute("area-id", "area-default");
  }

  @Test
  void unsetRemovesAttributeOnUpdate() {
    IdentityProviderMapperModel model = areaMapper("UNSET");
    BrokeredIdentityContext context = samlContext("id", "region", "09", "precinct", "0999");
    mapper.preprocessFederatedIdentity(session, realm, model, context);
    UserModel user = mock(UserModel.class);
    when(user.getFirstAttribute("area-id")).thenReturn("area-1");
    mapper.updateBrokeredUser(session, realm, user, model, context);
    verify(user).removeAttribute("area-id");

    UserModel newUser = mock(UserModel.class);
    mapper.importNewUser(session, realm, newUser, model, context);
    verify(newUser, never()).setSingleAttribute(anyString(), anyString());
  }

  @Test
  void updateChangesAttributeOnlyWhenDifferent() {
    IdentityProviderMapperModel model = areaMapper("REJECT");
    BrokeredIdentityContext moved = samlContext("id", "region", "09", "precinct", "0001");
    mapper.preprocessFederatedIdentity(session, realm, model, moved);
    UserModel user = mock(UserModel.class);
    when(user.getAttributes()).thenReturn(Map.of("area-id", List.of("area-2")));
    mapper.updateBrokeredUser(session, realm, user, model, moved);
    verify(user).setSingleAttribute("area-id", "area-1");

    UserModel unchanged = mock(UserModel.class);
    when(unchanged.getAttributes()).thenReturn(Map.of("area-id", List.of("area-1")));
    mapper.updateBrokeredUser(session, realm, unchanged, model, moved);
    verify(unchanged, never()).setSingleAttribute(anyString(), anyString());
  }

  @Test
  void updateWithoutPreprocessResolvesAgain() {
    IdentityProviderMapperModel model = areaMapper("REJECT");
    BrokeredIdentityContext context = samlContext("id", "region", "09", "precinct", "0150");
    UserModel user = mock(UserModel.class);
    when(user.getAttributes()).thenReturn(Map.of());
    mapper.updateBrokeredUser(session, realm, user, model, context);
    verify(user).setSingleAttribute("area-id", "area-2");
  }

  @Test
  void emailTargetUsesUserProperty() {
    IdentityProviderMapperModel model =
        mapperModel(
            MapperSettings.SOURCES, "mail",
            MapperSettings.CASE, "LOWER",
            MapperSettings.TARGET_ATTRIBUTE, "email");
    BrokeredIdentityContext context = samlContext("id", "mail", "Voter@Example.org");
    mapper.preprocessFederatedIdentity(session, realm, model, context);
    assertEquals("voter@example.org", context.getEmail());
    UserModel user = mock(UserModel.class);
    mapper.updateBrokeredUser(session, realm, user, model, context);
    verify(user).setEmail("voter@example.org");
  }

  @Test
  void invalidConfigurationFailsLoginWithServerError() {
    IdentityProviderMapperModel model =
        mapperModel(
            MapperSettings.SOURCES, "a",
            MapperSettings.TARGET_ATTRIBUTE, "x",
            MapperSettings.LOOKUP_TABLE, "garbage");
    BrokeredIdentityContext context = samlContext("id", "a", "1");
    Rejected rejected =
        assertThrows(
            Rejected.class,
            () -> mapper.preprocessFederatedIdentity(session, realm, model, context));
    assertEquals(Response.Status.INTERNAL_SERVER_ERROR, rejected.status);
  }

  @Test
  void settingsAreCachedUntilConfigChanges() {
    IdentityProviderMapperModel model = areaMapper("REJECT");
    MapperSettings first = AttributeLookupMapper.settings(model);
    assertSame(first, AttributeLookupMapper.settings(model));
    model.getConfig().put(MapperSettings.LOOKUP_TABLE, "area-3 = 09:0001");
    MapperSettings second = AttributeLookupMapper.settings(model);
    assertEquals("area-3", second.table().lookup("09:0001"));
  }
}
