// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequent.vote>
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.Response;
import java.util.Set;
import org.keycloak.authentication.AuthenticationFlowContext;
import org.keycloak.authentication.AuthenticationFlowError;
import org.keycloak.authentication.AuthenticatorFactory;
import org.keycloak.authentication.FlowStatus;
import org.keycloak.authentication.authenticators.directgrant.ValidatePassword;
import org.keycloak.models.ClientModel;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.sessions.AuthenticationSessionModel;

/** Password validation for the dedicated, confidential default-tenant trustee runner. */
@AutoService(AuthenticatorFactory.class)
public final class TrusteeRunnerPassword extends ValidatePassword {
  public static final String PROVIDER_ID = "sequent-trustee-runner-password";
  static final String CLIENT_ID = "sequent-trustee-runner";
  static final String REALM = "tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
  static final String FLOW = "Sequent Trustee Runner Password Grant";
  private static final String VERIFIED_NOTE = "sequent.trustee-runner.password-verified";
  private static final Set<String> USERNAMES = Set.of("trustee1", "trustee2", "trustee3", "trustee4", "trustee5", "trustee6");

  static boolean eligible(RealmModel realm, ClientModel client, UserModel user) {
    return realm != null
        && REALM.equals(realm.getName())
        && client != null
        && CLIENT_ID.equals(client.getClientId())
        && client.isEnabled()
        && "openid-connect".equals(client.getProtocol())
        && "client-secret".equals(client.getClientAuthenticatorType())
        && !client.isPublicClient()
        && !client.isBearerOnly()
        && client.isDirectAccessGrantsEnabled()
        && !client.isStandardFlowEnabled()
        && !client.isImplicitFlowEnabled()
        && !client.isServiceAccountsEnabled()
        && !client.isFullScopeAllowed()
        && user != null
        && user.isEnabled()
        && USERNAMES.contains(user.getUsername())
        && user.getServiceAccountClientLink() == null
        && user.getAttributeStream("trustee").toList().equals(java.util.List.of(user.getUsername()))
        && user.getAttributeStream("tenant-id").toList()
            .equals(java.util.List.of(REALM.substring("tenant-".length())))
        && user.getGroupsStream()
            .anyMatch(group -> group.getParentId() == null && "trustee".equals(group.getName()));
  }

  static boolean verifiedMachineSession(
      RealmModel realm, AuthenticationSessionModel session, UserModel user) {
    return session != null
        && eligible(realm, session.getClient(), user)
        && user.getId() != null
        && user.getId().equals(session.getAuthNote(VERIFIED_NOTE));
  }

  @Override
  public void authenticate(AuthenticationFlowContext context) {
    AuthenticationSessionModel session = context.getAuthenticationSession();
    UserModel user = context.getUser();
    if (session != null) {
      session.removeAuthNote(VERIFIED_NOTE);
    }
    if (session == null
        || !eligible(context.getRealm(), session.getClient(), user)
        || context.getTopLevelFlow() == null
        || !FLOW.equals(context.getTopLevelFlow().getAlias())
        || !"password".equals(
            context.getHttpRequest().getDecodedFormParameters().getFirst("grant_type"))) {
      context.failure(
          AuthenticationFlowError.INVALID_USER,
          errorResponse(Response.Status.BAD_REQUEST.getStatusCode(),
              "invalid_grant", "Invalid user credentials"));
      return;
    }
    // Call Keycloak's stock validator, not a hand-rolled password comparison.
    super.authenticate(context);
    if (context.getStatus() == FlowStatus.SUCCESS && user.getId() != null) {
      session.setAuthNote(VERIFIED_NOTE, user.getId());
    }
  }

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  @Override
  public String getDisplayType() {
    return "Sequent Trustee Runner Password";
  }
}
