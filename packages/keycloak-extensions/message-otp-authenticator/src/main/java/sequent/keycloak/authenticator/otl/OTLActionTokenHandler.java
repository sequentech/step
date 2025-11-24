// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.otl;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.Response;
import java.net.URI;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.TokenVerifier.Predicate;
import org.keycloak.authentication.AuthenticationProcessor;
import org.keycloak.authentication.actiontoken.AbstractActionTokenHandler;
import org.keycloak.authentication.actiontoken.ActionTokenContext;
import org.keycloak.authentication.actiontoken.ActionTokenHandlerFactory;
import org.keycloak.authentication.actiontoken.TokenUtils;
import org.keycloak.events.Details;
import org.keycloak.events.Errors;
import org.keycloak.events.EventBuilder;
import org.keycloak.events.EventType;
import org.keycloak.models.ClientModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;
import org.keycloak.services.managers.AuthenticationSessionManager;
import org.keycloak.services.messages.Messages;
import org.keycloak.services.util.AuthenticationFlowURLHelper;
import org.keycloak.sessions.AuthenticationSessionCompoundId;
import org.keycloak.sessions.AuthenticationSessionModel;
import org.keycloak.sessions.CommonClientSessionModel.ExecutionStatus;
import sequent.keycloak.authenticator.Utils;

/**
 * Handles an OTLActionToken. It looks up the session that initiated the OTL flow, set the
 * "otl-verified" auth-note to true and then creates a new session in the same flow step and
 * continues from there.
 *
 * <p>This allows to continue even in another web browser, without sharing the cookies.
 */
@JBossLog
@AutoService(ActionTokenHandlerFactory.class)
public class OTLActionTokenHandler extends AbstractActionTokenHandler<OTLActionToken> {
  public OTLActionTokenHandler() {
    super(
        /* id= */ OTLActionToken.TOKEN_TYPE,
        /* tokenClass= */ OTLActionToken.class,
        /* defaultErrorMessage= */ Messages.STALE_CODE,
        /* defaultEventType= */ EventType.IDENTITY_PROVIDER_LINK_ACCOUNT,
        /* defaultEventError= */ Errors.INVALID_TOKEN);

    log.info("OTLActionTokenHandler");
  }

  protected AuthenticationSessionModel getOriginalSession(
      ActionTokenContext<OTLActionToken> tokenContext, OTLActionToken token) {
    final KeycloakSession session = tokenContext.getSession();
    final RealmModel realm = tokenContext.getRealm();
    final String originalCompoundSessionId = token.getOriginalCompoundSessionId();
    final AuthenticationSessionManager asm = new AuthenticationSessionManager(session);
    final AuthenticationSessionCompoundId compoundId =
        AuthenticationSessionCompoundId.encoded(originalCompoundSessionId);
    final ClientModel originalClient = realm.getClientById(compoundId.getClientUUID());
    final AuthenticationSessionModel originalSession =
        asm.getAuthenticationSessionByIdAndClient(
            realm, compoundId.getRootSessionId(), originalClient, compoundId.getTabId());

    return originalSession;
  }

  // getVerifiers() checks that the original session is found to be copied. If
  // it's not, then we show Messages.EXPIRED_ACTION_TOKEN_NO_SESSION message
  @SuppressWarnings("unchecked")
  @Override
  public Predicate<? super OTLActionToken>[] getVerifiers(
      ActionTokenContext<OTLActionToken> tokenContext) {
    log.info("getVerifiers()");
    return TokenUtils.predicates(
        TokenUtils.checkThat(
            (token) -> {
              final AuthenticationSessionModel originalSession =
                  getOriginalSession(tokenContext, token);

              return originalSession != null;
            },
            Errors.NOT_ALLOWED,
            Messages.EXPIRED_ACTION_TOKEN_NO_SESSION));
  }

  @Override
  public AuthenticationSessionModel startFreshAuthenticationSession(
      OTLActionToken token, ActionTokenContext<OTLActionToken> tokenContext) {
    return tokenContext.createAuthenticationSessionForClient(token.getIssuedFor());
  }

  @Override
  public Response handleToken(
      OTLActionToken token, ActionTokenContext<OTLActionToken> tokenContext) {
    log.info("handleToken(): start");
    AuthenticationSessionModel authSession = tokenContext.getAuthenticationSession();
    final String originalCompoundSessionId = token.getOriginalCompoundSessionId();
    final String originUserId = token.getUserId();
    final List<String> authNoteNames = new ArrayList<>(Arrays.asList(token.getAuthNoteNames()));
    final boolean isDeferredUser = token.getIsDeferredUser();
    final RealmModel realm = tokenContext.getRealm();
    final KeycloakSession session = tokenContext.getSession();

    // we need to restore the current flow path too
    if (!authNoteNames.contains(AuthenticationProcessor.CURRENT_FLOW_PATH)) {
      authNoteNames.add(AuthenticationProcessor.CURRENT_FLOW_PATH);
    }

    EventBuilder event = tokenContext.getEvent();
    event
        .event(EventType.IDENTITY_PROVIDER_LINK_ACCOUNT)
        .detail(Details.CONTEXT, "originUserId = " + originUserId);
    event.success();

    log.infov(
        "handleToken(): tokenContext.isAuthenticationSessionFresh() = {0}",
        tokenContext.isAuthenticationSessionFresh());

    AuthenticationSessionManager asm = new AuthenticationSessionManager(session);

    AuthenticationSessionCompoundId compoundId =
        AuthenticationSessionCompoundId.encoded(originalCompoundSessionId);
    ClientModel originalClient = realm.getClientById(compoundId.getClientUUID());

    // NOTE: originalSession cannot be null, we checked this in getVerifiers()
    AuthenticationSessionModel originalSession =
        asm.getAuthenticationSessionByIdAndClient(
            realm, compoundId.getRootSessionId(), originalClient, compoundId.getTabId());

    // Copy all relevant data from originalSession to targetSession
    originalSession
        .getClientNotes()
        .forEach(
            (String name, String note) -> {
              log.infov("setClientNote name={0}, value={1}", name, note);
              authSession.setClientNote(name, note);
            });
    originalSession
        .getUserSessionNotes()
        .forEach(
            (String name, String note) -> {
              log.infov("setting setUserSessionNote name={0}", name);
              authSession.setUserSessionNote(name, note);
            });
    authNoteNames.forEach(
        (String name) -> {
          log.debugv(
              "setting setAuthNote name={0}, value={1}", name, originalSession.getAuthNote(name));
          authSession.setAuthNote(name, originalSession.getAuthNote(name));
        });
    originalSession
        .getExecutionStatus()
        .forEach(
            (String authenticator, ExecutionStatus status) -> {
              log.infov(
                  "setting setExecutionStatus authenticator={0}, status={1}",
                  authenticator, status);
              authSession.setExecutionStatus(authenticator, status);
            });
    log.infov("setting redirectUri={0}", originalSession.getRedirectUri());
    authSession.setRedirectUri(originalSession.getRedirectUri());

    log.infov(
        "setting executionId={0}",
        originalSession.getAuthNote(AuthenticationProcessor.LAST_PROCESSED_EXECUTION));
    tokenContext.setExecutionId(
        originalSession.getAuthNote(AuthenticationProcessor.LAST_PROCESSED_EXECUTION));

    if (isDeferredUser) {
      authSession.setAuthenticatedUser(null);
    }

    log.info("setting OTL_VISITED=true");
    authSession.setAuthNote(Utils.OTL_VISITED, "true");

    // Once everything is copied, then we remove the original auth session
    asm.removeAuthenticationSession(realm, originalSession, true);

    AuthenticationFlowURLHelper helper =
        new AuthenticationFlowURLHelper(session, realm, tokenContext.getUriInfo());
    URI redirectUri = helper.getLastExecutionUrl(authSession);
    log.infov("redirectUri={0}", redirectUri.toString());
    return Response.status(Response.Status.FOUND).location(redirectUri).build();
  }

  // to execute again, you will need a new token. it's a one time token
  @Override
  public boolean canUseTokenRepeatedly(
      OTLActionToken token, ActionTokenContext<OTLActionToken> tokenContext) {
    return false;
  }
}
