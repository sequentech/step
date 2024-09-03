// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.otl;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.Response;

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
import org.keycloak.sessions.AuthenticationSessionCompoundId;
import org.keycloak.sessions.AuthenticationSessionModel;
import sequent.keycloak.authenticator.Utils;

/**
 * Handles an OTLActionToken. It looks up the session that initiated the OTL
 * flow, set the "otl-verified" auth-note to true and then creates a new session
 * in the same flow step and continues from there.
 * 
 * This allows to continue even in another web browser, without sharing the 
 * cookies.
 */
@JBossLog
@AutoService(ActionTokenHandlerFactory.class)
public class OTLActionTokenHandler
    extends AbstractActionTokenHandler<OTLActionToken> {
  public OTLActionTokenHandler() {
    super(
        /* id= */ OTLActionToken.TOKEN_TYPE,
        /* tokenClass= */ OTLActionToken.class,
        /* defaultErrorMessage= */ Messages.STALE_CODE,
        /* defaultEventType= */ EventType.IDENTITY_PROVIDER_LINK_ACCOUNT,
        /* defaultEventError= */ Errors.INVALID_TOKEN
    );

    log.info("OTLActionTokenHandler");
  }

  // getVerifiers() needs not to be empty, so we verify email (which should
  // checkout always, even if it's null)
  @Override
  public Predicate<? super OTLActionToken>[] getVerifiers(
      ActionTokenContext<OTLActionToken> tokenContext) {
    log.info("getVerifiers()");
    return TokenUtils.predicates(verifyEmail(tokenContext));
  }

  @Override
  public Response handleToken(
      OTLActionToken token, ActionTokenContext<OTLActionToken> tokenContext
  ) {
    log.info("handleToken(): start");
    AuthenticationSessionModel authSession = tokenContext.getAuthenticationSession();
    final String originalCompoundSessionId = token.getOriginalCompoundSessionId();
    final String originUserId = token.getUserId();
    final RealmModel realm = tokenContext.getRealm();
    final KeycloakSession session = tokenContext.getSession();
    
    EventBuilder event = tokenContext.getEvent();
    event.event(EventType.IDENTITY_PROVIDER_LINK_ACCOUNT)
        .detail(Details.CONTEXT, "originUserId = " + originUserId);
    event.success();

    tokenContext.getEvent().success();
    tokenContext.setEvent(tokenContext.getEvent().clone().event(EventType.VERIFY_EMAIL));

    AuthenticationSessionManager asm = new AuthenticationSessionManager(session);

    AuthenticationSessionCompoundId compoundId =
      AuthenticationSessionCompoundId.encoded(originalCompoundSessionId);
    ClientModel originalClient = realm.getClientById(compoundId.getClientUUID());
    AuthenticationSessionModel originalSession = asm
      .getAuthenticationSessionByIdAndClient(
        realm,
        compoundId.getRootSessionId(),
        originalClient,
        compoundId.getTabId()
      );

    // Check if the session is fresh or needs to be recreated
    if (tokenContext.isAuthenticationSessionFresh()) {
      log.debug("tokenContext.isAuthenticationSessionFresh()");
    }

    // Copy all relevant data from originalSession to targetSession
    originalSession.getClientNotes().forEach(authSession::setClientNote);
    originalSession.getUserSessionNotes().forEach(authSession::setUserSessionNote);
    // TODO:
    // originalSession.getAuthNotes().forEach(authSession::setAuthNote);
    originalSession.getExecutionStatus().forEach(authSession::setExecutionStatus);

    authSession.setAuthNote(Utils.OTL_VISITED, "true");

    // Once everything is copied, then we remove the original auth session
    asm.removeAuthenticationSession(realm, authSession, true);

    // redirect to the current flow path
    return tokenContext.brokerFlow(
      null,
      null,
      authSession.getAuthNote(AuthenticationProcessor.CURRENT_FLOW_PATH)
    );
  }

  // to execute again, you will need a new token. it's a one time token
  @Override
  public boolean canUseTokenRepeatedly(
    OTLActionToken token,
    ActionTokenContext<OTLActionToken> tokenContext
  ) {
    return false;
  }
}
