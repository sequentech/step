// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.idp_linking_authenticator;

import com.google.auto.service.AutoService;
import java.util.List;
import org.keycloak.Config;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.provider.ProviderConfigProperty;

/**
 * Factory for {@link CustomAttributeIdpLinkingAuthenticator}.
 *
 * <p>Exposes two configuration properties:
 *
 * <ul>
 *   <li>{@value #CONF_IDP_CLAIM} – the claim/attribute name to read from the incoming IdP identity
 *       (e.g. {@code email}, {@code username}, {@code SAFE_ID}).
 *   <li>{@value #CONF_USER_ATTRIBUTE} – the Keycloak user attribute whose values are searched for
 *       the incoming claim value (e.g. {@code linked_idp_identities}).
 * </ul>
 */
@AutoService(AuthenticatorFactory.class)
public class CustomAttributeIdpLinkingAuthenticatorFactory implements AuthenticatorFactory {

  public static final String PROVIDER_ID = "idp-linking-authenticator";

  public static final String CONF_IDP_CLAIM = "idp-claim";
  public static final String DEFAULT_IDP_CLAIM = "email";

  public static final String CONF_USER_ATTRIBUTE = "user-attribute";
  public static final String DEFAULT_USER_ATTRIBUTE = "linked_idp_identities";

  private static final CustomAttributeIdpLinkingAuthenticator SINGLETON =
      new CustomAttributeIdpLinkingAuthenticator();

  private static final AuthenticationExecutionModel.Requirement[] REQUIREMENT_CHOICES = {
    AuthenticationExecutionModel.Requirement.ALTERNATIVE,
    AuthenticationExecutionModel.Requirement.REQUIRED,
    AuthenticationExecutionModel.Requirement.DISABLED,
  };

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  @Override
  public String getDisplayType() {
    return "Custom Attribute IdP Identity Linking";
  }

  @Override
  public String getHelpText() {
    return "During the First Broker Login flow, searches for an existing Keycloak user whose"
        + " multi-value attribute contains the value of a configurable claim from the incoming IdP"
        + " identity. If exactly one match is found the IdP identity is linked to that user."
        + " Zero matches pass control to the next step; multiple matches fail the flow to prevent"
        + " ambiguous account linking.";
  }

  @Override
  public String getReferenceCategory() {
    return "Identity Provider Linking";
  }

  @Override
  public boolean isConfigurable() {
    return true;
  }

  @Override
  public boolean isUserSetupAllowed() {
    return false;
  }

  @Override
  public AuthenticationExecutionModel.Requirement[] getRequirementChoices() {
    return REQUIREMENT_CHOICES;
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    return List.of(
        new ProviderConfigProperty(
            CONF_IDP_CLAIM,
            "IdP Claim",
            "The claim/attribute to read from the incoming IdP identity (e.g. 'email',"
                + " 'username', 'id', or a custom mapped attribute such as 'SAFE_ID').",
            ProviderConfigProperty.STRING_TYPE,
            DEFAULT_IDP_CLAIM),
        new ProviderConfigProperty(
            CONF_USER_ATTRIBUTE,
            "User Attribute",
            "The Keycloak user attribute (multi-value) to search for the claim value"
                + " (e.g. 'linked_idp_identities').",
            ProviderConfigProperty.STRING_TYPE,
            DEFAULT_USER_ATTRIBUTE));
  }

  @Override
  public Authenticator create(KeycloakSession session) {
    return SINGLETON;
  }

  @Override
  public void init(Config.Scope config) {}

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public void close() {}
}
