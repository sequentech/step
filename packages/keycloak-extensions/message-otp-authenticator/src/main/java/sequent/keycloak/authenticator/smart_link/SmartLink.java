// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.smart_link;

import com.google.common.base.Strings;
import com.google.common.collect.ImmutableList;
import com.google.common.collect.Maps;
import jakarta.ws.rs.core.UriBuilder;
import jakarta.ws.rs.core.UriInfo;
import java.net.URI;
import java.util.List;
import java.util.Map;
import java.util.OptionalInt;
import java.util.function.Consumer;
import java.util.stream.Collectors;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.Config;
import org.keycloak.authentication.Authenticator;
import org.keycloak.authentication.authenticators.browser.CookieAuthenticatorFactory;
import org.keycloak.authentication.authenticators.browser.IdentityProviderAuthenticatorFactory;
import org.keycloak.common.util.Time;
import org.keycloak.email.EmailException;
import org.keycloak.email.EmailTemplateProvider;
import org.keycloak.events.Details;
import org.keycloak.events.EventBuilder;
import org.keycloak.events.EventType;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.AuthenticationFlowModel;
import org.keycloak.models.ClientModel;
import org.keycloak.models.Constants;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.KeycloakSessionTask;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.utils.KeycloakModelUtils;
import org.keycloak.protocol.oidc.OIDCLoginProtocol;
import org.keycloak.protocol.oidc.utils.RedirectUtils;
import org.keycloak.provider.ProviderFactory;
import org.keycloak.services.Urls;
import org.keycloak.services.resources.LoginActionsService;
import org.keycloak.services.resources.RealmsResource;
import org.keycloak.sessions.AuthenticationSessionModel;

/** Common utilities Smart Link authentication, used by the authenticator and resource */
@JBossLog
public class SmartLink {

  public static final String SMART_LINK_AUTH_FLOW_ALIAS = "smart-link";

  public static final String COOKIE_PROVIDER_ID = CookieAuthenticatorFactory.PROVIDER_ID;

  public static final String IDP_REDIRECTOR_PROVIDER_ID =
      IdentityProviderAuthenticatorFactory.PROVIDER_ID;

  public static final String SMART_LINK_PROVIDER_ID = SmartLinkAuthenticatorFactory.PROVIDER_ID;

  public static Consumer<UserModel> registerEvent(final EventBuilder event) {
    return new Consumer<UserModel>() {
      @Override
      public void accept(UserModel user) {
        event
            .event(EventType.REGISTER)
            .detail(Details.REGISTER_METHOD, SmartLinkActionToken.TOKEN_TYPE)
            .detail(Details.USERNAME, user.getUsername())
            .detail(Details.EMAIL, user.getEmail())
            .user(user)
            .success();
      }
    };
  }

  public static UserModel getOrCreate(
      KeycloakSession session,
      RealmModel realm,
      String emailOrUsername,
      boolean forceCreate,
      boolean updateProfile,
      boolean updatePassword,
      Consumer<UserModel> onNew) {
    UserModel user = KeycloakModelUtils.findUserByNameOrEmail(session, realm, emailOrUsername);
    if (user == null && forceCreate) {
      user = session.users().addUser(realm, emailOrUsername);
      user.setEnabled(true);
      user.setEmail(emailOrUsername);
      if (onNew != null) {
        onNew.accept(user);
      }
      if (updatePassword) {
        user.addRequiredAction(UserModel.RequiredAction.UPDATE_PASSWORD);
      }
      if (updateProfile) {
        user.addRequiredAction(UserModel.RequiredAction.UPDATE_PROFILE);
      }
    }

    return user;
  }

  public static SmartLinkActionToken createActionToken(
      UserModel user,
      String clientId,
      OptionalInt validity,
      Boolean rememberMe,
      AuthenticationSessionModel authSession,
      Boolean isActionTokenPersistent,
      Boolean markEmailVerified) {
    String redirectUri = authSession.getRedirectUri();
    String scopes = authSession.getClientNote(OIDCLoginProtocol.SCOPE_PARAM);
    String state = authSession.getClientNote(OIDCLoginProtocol.STATE_PARAM);
    String nonce = authSession.getClientNote(OIDCLoginProtocol.NONCE_PARAM);
    log.infof(
        "Attempting SmartLinkAuthenticator for %s, %s, %s", user.getEmail(), clientId, redirectUri);
    log.infof("SmartLinkAuthenticator extra vars %s %s %s %b", scopes, state, nonce, rememberMe);
    return createActionToken(
        user,
        clientId,
        redirectUri,
        validity,
        scopes,
        nonce,
        state,
        rememberMe,
        isActionTokenPersistent,
        markEmailVerified);
  }

  public static SmartLinkActionToken createActionToken(
      UserModel user,
      String clientId,
      String redirectUri,
      OptionalInt validity,
      String scopes,
      String nonce,
      String state,
      Boolean rememberMe,
      Boolean persistent,
      Boolean markEmailVerified) {
    // build the action token
    int validityInSecs = validity.orElse(60 * 60 * 24); // 1 day
    int absoluteExpirationInSecs = Time.currentTime() + validityInSecs;
    SmartLinkActionToken token =
        new SmartLinkActionToken(
            user.getId(),
            absoluteExpirationInSecs,
            nonce,
            clientId,
            markEmailVerified,
            redirectUri,
            scopes,
            state,
            rememberMe,
            persistent);
    return token;
  }

  public static String linkFromActionToken(
      KeycloakSession session, RealmModel realm, SmartLinkActionToken token) {
    UriInfo uriInfo = session.getContext().getUri();

    // This is a workaround for situations where the realm you are using to
    // call this (e.g. master) is different than the one you are generating
    // the action token for. Because the SignatureProvider assumes the value
    // that is set in session.getContext().getRealm() has the keys it should
    // use, we need to temporarily reset it
    RealmModel r = session.getContext().getRealm();
    log.debugf("realm %s session.context.realm %s", realm.getName(), r.getName());

    // Because of the risk, throw an exception for master realm
    if (Config.getAdminRealm().equals(realm.getName())) {
      throw new IllegalStateException(
          String.format("Smart links not allowed for %s realm", Config.getAdminRealm()));
    }
    session.getContext().setRealm(realm);

    UriBuilder builder =
        actionTokenBuilder(
            uriInfo.getBaseUri(), token.serialize(session, realm, uriInfo), token.getIssuedFor());

    // and then set it back
    session.getContext().setRealm(r);
    return builder.build(realm.getName()).toString();
  }

  public static boolean validateRedirectUri(
      KeycloakSession session, String redirectUri, ClientModel client) {
    String redirect = RedirectUtils.verifyRedirectUri(session, redirectUri, client);
    log.debugf("Redirect after verify %s -> %s", redirectUri, redirect);
    return redirectUri.equals(redirect);
  }

  private static UriBuilder actionTokenBuilder(URI baseUri, String tokenString, String clientId) {
    log.debugf("baseUri: %s, tokenString: %s, clientId: %s", baseUri, tokenString, clientId);
    return Urls.realmBase(baseUri)
        .path(RealmsResource.class, "getLoginActionsService")
        .path(LoginActionsService.class, "executeActionToken")
        .queryParam(Constants.KEY, tokenString)
        .queryParam(Constants.CLIENT_ID, clientId);
  }

  // TODO: send sms too
  public static boolean sendSmartLinkNotification(
      KeycloakSession session, UserModel user, String link) {
    RealmModel realm = session.getContext().getRealm();
    try {
      EmailTemplateProvider emailTemplateProvider =
          session.getProvider(EmailTemplateProvider.class);
      String realmName = getRealmName(realm);
      List<Object> subjAttr = ImmutableList.of(realmName);
      Map<String, Object> bodyAttr = Maps.newHashMap();
      bodyAttr.put("realmName", realmName);
      bodyAttr.put("SmartLink", link);
      emailTemplateProvider
          .setRealm(realm)
          .setUser(user)
          .setAttribute("realmName", realmName)
          .send("SmartLinkSubject", subjAttr, "smart-link-email.ftl", bodyAttr);
      return true;
    } catch (EmailException error) {
      log.error("Failed to send smart link email", error);
    }
    return false;
  }

  public static String getRealmName(RealmModel realm) {
    return Strings.isNullOrEmpty(realm.getDisplayName()) ? realm.getName() : realm.getDisplayName();
  }

  public static void realmPostCreate(
      KeycloakSessionFactory factory, RealmModel.RealmPostCreateEvent event) {
    setupDefaultFlow(event.getKeycloakSession(), event.getCreatedRealm());
  }

  public static void realmPostCreateInTransaction(
      KeycloakSessionFactory factory, RealmModel.RealmPostCreateEvent event) {
    final String name = event.getCreatedRealm().getName();
    KeycloakModelUtils.runJobInTransaction(
        factory,
        new KeycloakSessionTask() {
          @Override
          public void run(KeycloakSession session) {
            try {
              setupDefaultFlow(session, session.realms().getRealmByName(name));
            } catch (Exception error) {
              log.warn("Error setting up default smart link flow", error);
            }
          }
        });
  }

  public static void setupDefaultFlow(KeycloakSession session, RealmModel realm) {
    AuthenticationFlowModel flow = realm.getFlowByAlias(SMART_LINK_AUTH_FLOW_ALIAS);
    if (flow != null) {
      log.infof("%s flow exists. Skipping.", SMART_LINK_AUTH_FLOW_ALIAS);
      return;
    }

    log.infof("creating built-in auth flow for %s", SMART_LINK_AUTH_FLOW_ALIAS);
    flow = new AuthenticationFlowModel();
    flow.setAlias(SMART_LINK_AUTH_FLOW_ALIAS);
    flow.setBuiltIn(true);
    flow.setProviderId("basic-flow");
    flow.setDescription("Simple smart link authentication flow.");
    flow.setTopLevel(true);
    flow = realm.addAuthenticationFlow(flow);

    // cookie
    addExecutionToFlow(
        session,
        realm,
        flow,
        COOKIE_PROVIDER_ID,
        AuthenticationExecutionModel.Requirement.ALTERNATIVE);
    // identity provider redirector
    addExecutionToFlow(
        session,
        realm,
        flow,
        IDP_REDIRECTOR_PROVIDER_ID,
        AuthenticationExecutionModel.Requirement.ALTERNATIVE);

    // forms
    AuthenticationFlowModel forms = new AuthenticationFlowModel();
    forms.setAlias(String.format("%s %s", SMART_LINK_AUTH_FLOW_ALIAS, "forms"));
    forms.setProviderId("basic-flow");
    forms.setDescription("Forms for simple smart link authentication flow.");
    forms.setTopLevel(false);
    forms = realm.addAuthenticationFlow(forms);

    AuthenticationExecutionModel execution = new AuthenticationExecutionModel();
    execution.setParentFlow(flow.getId());
    execution.setFlowId(forms.getId());
    execution.setRequirement(AuthenticationExecutionModel.Requirement.ALTERNATIVE);
    execution.setAuthenticatorFlow(true);
    execution.setPriority(getNextPriority(realm, flow));
    execution = realm.addAuthenticatorExecution(execution);

    addExecutionToFlow(
        session,
        realm,
        forms,
        SMART_LINK_PROVIDER_ID,
        AuthenticationExecutionModel.Requirement.REQUIRED);
  }

  private static int getNextPriority(RealmModel realm, AuthenticationFlowModel parentFlow) {
    List<AuthenticationExecutionModel> executions =
        realm.getAuthenticationExecutionsStream(parentFlow.getId()).collect(Collectors.toList());
    return (executions.isEmpty()) ? 0 : executions.get(executions.size() - 1).getPriority() + 1;
  }

  private static void addExecutionToFlow(
      KeycloakSession session,
      RealmModel realm,
      AuthenticationFlowModel flow,
      String providerId,
      AuthenticationExecutionModel.Requirement requirement) {
    boolean hasExecution =
        (realm
                .getAuthenticationExecutionsStream(flow.getId())
                .filter(e -> providerId.equals(e.getAuthenticator()))
                .count()
            > 0);

    if (!hasExecution) {
      log.infof("adding execution %s for auth flow for %s", providerId, flow.getAlias());
      ProviderFactory f =
          session.getKeycloakSessionFactory().getProviderFactory(Authenticator.class, providerId);
      AuthenticationExecutionModel execution = new AuthenticationExecutionModel();
      execution.setParentFlow(flow.getId());
      execution.setRequirement(requirement);
      execution.setAuthenticatorFlow(false);
      execution.setAuthenticator(providerId);
      execution = realm.addAuthenticatorExecution(execution);
    }
  }
}
