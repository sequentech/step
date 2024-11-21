// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link;

import jakarta.ws.rs.*;
import jakarta.ws.rs.core.MediaType;
import java.util.OptionalInt;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.models.ClientModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.UserModel;

@JBossLog
public class SmartLinkResource extends AbstractAdminResource {

  public SmartLinkResource(KeycloakSession session) {
    super(session);
  }

  @POST
  @Consumes(MediaType.APPLICATION_JSON)
  @Produces(MediaType.APPLICATION_JSON)
  public SmartLinkResponse createSmartLink(final SmartLinkRequest request) {
    if (!permissions.users().canManage()) {
      throw new ForbiddenException("magic link requires manage-users");
    }

    ClientModel client = session.clients().getClientByClientId(realm, request.getClientId());
    if (client == null) {
      throw new NotFoundException(
          String.format("Client with ID %s not found.", request.getClientId()));
    }

    if (!SmartLink.validateRedirectUri(session, request.getRedirectUri(), client)) {
      throw new BadRequestException(
          String.format("redirectUri %s disallowed by client.", request.getRedirectUri()));
    }

    String emailOrUsername = request.getEmailOrUsername();
    boolean forceCreate = request.isForceCreate();
    boolean sendNotification = request.isSendNotification();
    final boolean updateProfile = request.isUpdateProfile();
    final boolean updatePassword = request.isUpdatePassword();

    if (request.getUsername() != null) {
      emailOrUsername = request.getUsername();
      forceCreate = false;
      sendNotification = false;
    }

    UserModel user =
        SmartLink.getOrCreate(
            session,
            realm,
            emailOrUsername,
            forceCreate,
            updateProfile,
            updatePassword,
            SmartLink.registerEvent(event));
    if (user == null) {
      throw new NotFoundException(
          String.format(
              "User with email/username %s not found, and forceCreate is off.", emailOrUsername));
    }

    SmartLinkActionToken token =
        SmartLink.createActionToken(
            user,
            request.getClientId(),
            request.getRedirectUri(),
            OptionalInt.of(request.getExpirationSeconds()),
            request.getScopes(),
            request.getNonce(),
            request.getState(),
            request.getRememberMe(),
            request.getActionTokenPersistent(),
            request.getMarkEmailVerified());
    String link = SmartLink.linkFromActionToken(session, realm, token);
    boolean sent = false;
    if (sendNotification) {
      sent = SmartLink.sendSmartLinkNotification(session, user, link);
      log.infof("sent notification to %s? %b. Link? %s", request.getEmailOrUsername(), sent, link);
    }

    SmartLinkResponse response = new SmartLinkResponse();
    response.setUserId(user.getId());
    response.setLink(link);
    response.setSent(sent);

    return response;
  }
}
