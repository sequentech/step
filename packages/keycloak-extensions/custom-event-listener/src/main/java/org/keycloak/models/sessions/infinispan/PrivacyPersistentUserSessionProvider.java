// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

package org.keycloak.models.sessions.infinispan;

import java.util.Collection;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.function.Predicate;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.UserSessionModel;
import org.keycloak.models.sessions.infinispan.changes.SessionEntityWrapper;
import org.keycloak.models.sessions.infinispan.entities.UserSessionEntity;
import org.keycloak.models.utils.UserSessionModelDelegate;

/**
 * Pinned to Keycloak's persistent embedded provider. This package is required to override its
 * package-private adapter factory and reuse its protected transactions without reflection.
 */
public final class PrivacyPersistentUserSessionProvider extends PersistentUserSessionProvider {
  private final Predicate<RealmModel> privateRealm;
  private final Predicate<String> networkNote;

  public PrivacyPersistentUserSessionProvider(
      PersistentUserSessionProvider builtIn,
      Predicate<RealmModel> privateRealm,
      Predicate<String> networkNote) {
    // The built-in factory owns cache initialization, transaction registration and cluster
    // listeners.
    // Its original provider's empty event sender remains harmless; this instance records mutations.
    super(builtIn.session, builtIn.sessionTx, builtIn.clientSessionTx);
    this.privateRealm = privateRealm;
    this.networkNote = networkNote;
  }

  private Map<String, String> safeNotes(Map<String, String> input) {
    Map<String, String> result = new LinkedHashMap<>();
    input.forEach(
        (name, value) -> {
          if (!networkNote.test(name)) result.put(name, value);
        });
    return result;
  }

  private UserSessionModel safeImport(UserSessionModel input) {
    if (!privateRealm.test(input.getRealm())) return input;
    return new UserSessionModelDelegate(input) {
      @Override
      public String getIpAddress() {
        return null;
      }

      @Override
      public Map<String, String> getNotes() {
        return safeNotes(input.getNotes());
      }
    };
  }

  @Override
  public UserSessionModel createUserSession(
      String id,
      RealmModel realm,
      UserModel user,
      String loginUsername,
      String ipAddress,
      String authMethod,
      boolean rememberMe,
      String brokerSessionId,
      String brokerUserId,
      UserSessionModel.SessionPersistenceState persistence) {
    return super.createUserSession(
        id,
        realm,
        user,
        loginUsername,
        privateRealm.test(realm) ? null : ipAddress,
        authMethod,
        rememberMe,
        brokerSessionId,
        brokerUserId,
        persistence);
  }

  @Override
  public void importUserSessions(Collection<UserSessionModel> sessions, boolean offline) {
    super.importUserSessions(sessions.stream().map(this::safeImport).toList(), offline);
  }

  @Override
  public SessionEntityWrapper<UserSessionEntity> importUserSession(
      UserSessionModel input, boolean offline) {
    return super.importUserSession(safeImport(input), offline);
  }

  @Override
  public SessionEntityWrapper<UserSessionEntity> wrapPersistentEntity(
      RealmModel realm, boolean offline, UserSessionModel input) {
    return super.wrapPersistentEntity(realm, offline, safeImport(input));
  }

  @Override
  public UserSessionModel createOfflineUserSession(UserSessionModel input) {
    return super.createOfflineUserSession(safeImport(input));
  }

  @Override
  UserSessionAdapter<?> wrap(
      RealmModel realm, UserSessionEntity entity, boolean offline, UserModel user) {
    if (!privateRealm.test(realm)) return super.wrap(realm, entity, offline, user);
    return new PrivateSession(user, realm, entity, offline);
  }

  private final class PrivateSession
      extends UserSessionAdapter<PrivacyPersistentUserSessionProvider> {
    PrivateSession(UserModel user, RealmModel realm, UserSessionEntity entity, boolean offline) {
      super(
          session,
          user,
          PrivacyPersistentUserSessionProvider.this,
          sessionTx,
          clientSessionTx,
          realm,
          entity,
          offline);
    }

    @Override
    public String getIpAddress() {
      return null;
    }

    @Override
    public Map<String, String> getNotes() {
      return safeNotes(super.getNotes());
    }

    @Override
    public String getNote(String name) {
      return networkNote.test(name) ? null : super.getNote(name);
    }

    @Override
    public void setNote(String name, String value) {
      if (networkNote.test(name)) super.removeNote(name);
      else super.setNote(name, value);
    }

    @Override
    public void restartSession(
        RealmModel realm,
        UserModel user,
        String loginUsername,
        String ipAddress,
        String authMethod,
        boolean rememberMe,
        String brokerSessionId,
        String brokerUserId) {
      super.restartSession(
          realm,
          user,
          loginUsername,
          privateRealm.test(realm) ? null : ipAddress,
          authMethod,
          rememberMe,
          brokerSessionId,
          brokerUserId);
    }
  }
}
