// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.idp_linking_authenticator;

import java.util.List;
import java.util.stream.Collectors;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.authentication.authenticators.broker.AbstractIdpAuthenticator;
import org.keycloak.authentication.authenticators.broker.IdpConfirmOverrideLinkAuthenticator;
import org.keycloak.authentication.authenticators.broker.util.SerializedBrokeredIdentityContext;
import org.keycloak.broker.provider.BrokeredIdentityContext;
import org.keycloak.models.AuthenticatorConfigModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;

/**
 * First Broker Login authenticator that links an incoming IdP identity to an existing Keycloak user
 * by matching a configurable IdP claim against a configurable multi-value user attribute.
 *
 * <p>Logic:
 *
 * <ul>
 *   <li>Exactly one match → link the identity to the found user and succeed.
 *   <li>Zero matches → attempt (pass control to the next step in the flow).
 *   <li>Multiple matches → fail with an {@link AuthenticationFlowError#IDENTITY_PROVIDER_ERROR} to
 *       prevent ambiguous account linking.
 * </ul>
 */
@JBossLog
public class CustomAttributeIdpLinkingAuthenticator extends AbstractIdpAuthenticator {

  @Override
  protected void authenticateImpl(
      AuthenticationFlowContext context,
      SerializedBrokeredIdentityContext serializedCtx,
      BrokeredIdentityContext brokerContext) {

    AuthenticatorConfigModel authConfig = context.getAuthenticatorConfig();
    if (authConfig == null || authConfig.getConfig() == null) {
      log.warn(
          "CustomAttributeIdpLinkingAuthenticator: no configuration found, proceeding with next"
              + " step");
      context.attempted();
      return;
    }

    String idpClaim =
        authConfig
            .getConfig()
            .getOrDefault(
                CustomAttributeIdpLinkingAuthenticatorFactory.CONF_IDP_CLAIM,
                CustomAttributeIdpLinkingAuthenticatorFactory.DEFAULT_IDP_CLAIM);
    String userAttributeName =
        authConfig
            .getConfig()
            .getOrDefault(
                CustomAttributeIdpLinkingAuthenticatorFactory.CONF_USER_ATTRIBUTE,
                CustomAttributeIdpLinkingAuthenticatorFactory.DEFAULT_USER_ATTRIBUTE);

    log.debugf(
        "CustomAttributeIdpLinkingAuthenticator: brokerContext.attributes=%s",
        brokerContext.getAttributes());

    String incomingIdentifier = extractClaimValue(brokerContext, idpClaim);
    if (incomingIdentifier == null || incomingIdentifier.isEmpty()) {
      log.warnf(
          "CustomAttributeIdpLinkingAuthenticator: no value found for IdP claim '%s', proceeding"
              + " with next step",
          idpClaim);
      context.attempted();
      return;
    }

    RealmModel realm = context.getRealm();
    KeycloakSession session = context.getSession();

    List<UserModel> matchingUsers =
        session
            .users()
            .searchForUserByUserAttributeStream(realm, userAttributeName, incomingIdentifier)
            .collect(Collectors.toList());

    if (matchingUsers.isEmpty()) {
      log.infof(
          "CustomAttributeIdpLinkingAuthenticator: no user found with attribute '%s'='%s',"
              + " proceeding with next step",
          userAttributeName, incomingIdentifier);
      context.attempted();
      return;
    }

    if (matchingUsers.size() > 1) {
      log.errorf(
          "CustomAttributeIdpLinkingAuthenticator: %d users found with attribute '%s'='%s',"
              + " failing to prevent ambiguous account linking",
          matchingUsers.size(), userAttributeName, incomingIdentifier);
      context.failure(AuthenticationFlowError.IDENTITY_PROVIDER_ERROR);
      return;
    }

    UserModel existingUser = matchingUsers.get(0);
    log.infof(
        "CustomAttributeIdpLinkingAuthenticator: linking IdP identity to user '%s' via"
            + " attribute '%s'='%s'",
        existingUser.getUsername(), userAttributeName, incomingIdentifier);

    // Force override Link as we are mapping different users to a single user.
    AuthenticationSessionModel authSession = context.getAuthenticationSession();
    authSession.setAuthNote(IdpConfirmOverrideLinkAuthenticator.OVERRIDE_LINK, "true");

    context.setUser(existingUser);
    context.success();
  }

  @Override
  protected void actionImpl(
      AuthenticationFlowContext context,
      SerializedBrokeredIdentityContext serializedCtx,
      BrokeredIdentityContext brokerContext) {
    // No interactive form is shown by this authenticator.
  }

  @Override
  public boolean requiresUser() {
    return false;
  }

  @Override
  public boolean configuredFor(KeycloakSession session, RealmModel realm, UserModel user) {
    return true;
  }

  @Override
  public void setRequiredActions(KeycloakSession session, RealmModel realm, UserModel user) {}

  @Override
  public void close() {}

  /**
   * Extracts the value of a claim from the brokered identity context.
   *
   * <p>Well-known fields (email, username, id/sub, firstname, lastname) are resolved through their
   * dedicated accessors. Any other name is looked up in the mapped user-attribute collection.
   */
  public String extractClaimValue(BrokeredIdentityContext brokerContext, String claim) {
    switch (claim.toLowerCase()) {
      case "email":
        return brokerContext.getEmail();
      case "username":
        return brokerContext.getUsername();
      case "id":
      case "sub":
        return brokerContext.getId();
      case "firstname":
      case "first_name":
        return brokerContext.getFirstName();
      case "lastname":
      case "last_name":
        return brokerContext.getLastName();
      default:
        return brokerContext.getUserAttribute(claim);
    }
  }
}
