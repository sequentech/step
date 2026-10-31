// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.custom_event_listener;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.Set;
import org.keycloak.connections.jpa.JpaConnectionProvider;
import org.keycloak.events.Event;
import org.keycloak.events.EventQuery;
import org.keycloak.events.EventStoreProvider;
import org.keycloak.events.admin.AdminEvent;
import org.keycloak.events.admin.AdminEventQuery;
import org.keycloak.events.admin.AuthDetails;
import org.keycloak.events.jpa.JpaEventStoreProvider;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.RealmModel;

/** Removes voter network attribution before the ordinary JPA event store sees an event. */
final class PrivacyEventStoreProvider extends JpaEventStoreProvider {
  private static final ObjectMapper JSON = new ObjectMapper();

  // Extension details are open-ended. Keep known authentication context, not arbitrary values under
  // unfamiliar names that could contain a browser, country, address, or device identifier.
  private static final Set<String> AUTHENTICATION_DETAILS =
      Set.of(
          "auth_method",
          "auth_type",
          "credential_type",
          "identity_provider",
          "identity_provider_identity",
          "username",
          "email",
          "remember_me",
          "response_type",
          "response_mode",
          "grant_type",
          "consent",
          "registration_method");

  private final KeycloakSession session;
  private final EventStoreProvider jpa;

  PrivacyEventStoreProvider(KeycloakSession session, EventStoreProvider jpa) {
    // The pinned JPA factory's periodic admin-expiry callback casts the default provider to this
    // base type. Inherit that maintenance path with the same session-owned entity manager.
    super(session, session.getProvider(JpaConnectionProvider.class).getEntityManager());
    this.session = session;
    this.jpa = jpa;
  }

  private boolean requiresRedaction(String realmId) {
    RealmModel realm = realmId == null ? null : session.realms().getRealm(realmId);
    // A removed or unresolved realm cannot justify a raw persistence fallback. Preserve the safe
    // audit outcome even when the authoritative realm cannot be found.
    return realm == null
        || realm.getName() == null
        || LogRealm.fromName(realm.getName()).kind() == LogRealm.Kind.EVENT;
  }

  @Override
  public void onEvent(Event event) {
    if (!requiresRedaction(event.getRealmId())) {
      jpa.onEvent(event);
      return;
    }
    // EventBuilder saves before invoking listeners. The original remains available to the separate,
    // explicitly enabled forensic sender; only this defensive copy reaches the ordinary store.
    Event safe = event.clone();
    safe.setIpAddress(null);
    safe.setError(UserEventDetails.safeError(event.getError()));
    safe.setDetails(authenticationDetails(event.getDetails()));
    jpa.onEvent(safe);
  }

  @Override
  public void onEvent(AdminEvent event, boolean includeRepresentation) {
    if (!requiresRedaction(event.getRealmId())) {
      jpa.onEvent(event, includeRepresentation);
      return;
    }
    // Keycloak's copy constructor dereferences authDetails even when a provider omits it.
    AdminEvent safe = new AdminEvent();
    safe.setId(event.getId());
    safe.setTime(event.getTime());
    safe.setRealmId(event.getRealmId());
    safe.setRealmName(event.getRealmName());
    safe.setOperationType(event.getOperationType());
    safe.setResourceTypeAsString(event.getResourceTypeAsString());
    safe.setResourcePath(event.getResourcePath());
    if (event.getAuthDetails() != null) {
      AuthDetails auth = new AuthDetails(event.getAuthDetails());
      auth.setIpAddress(null);
      safe.setAuthDetails(auth);
    }
    safe.setError(UserEventDetails.safeError(event.getError()));
    safe.setDetails(authenticationDetails(event.getDetails()));
    safe.setRepresentation(
        includeRepresentation ? safeRepresentation(event.getRepresentation()) : null);
    jpa.onEvent(safe, includeRepresentation);
  }

  private static Map<String, String> authenticationDetails(Map<String, String> details) {
    if (details == null) return null;
    Map<String, String> safe = new LinkedHashMap<>();
    details.forEach(
        (name, value) -> {
          if (AUTHENTICATION_DETAILS.contains(name)) {
            safe.put(name, UserEventDetails.safeDetailValue(value));
          }
        });
    return safe;
  }

  private static String safeRepresentation(String representation) {
    if (representation == null) return null;
    try {
      return JSON.writeValueAsString(AdminRepresentation.forEventRealmLog(representation));
    } catch (JsonProcessingException invalidJson) {
      // An unstructured representation has no safe field boundaries; never fall back to raw text.
      return JSON.getNodeFactory().textNode(AdminRepresentation.REDACTED).toString();
    }
  }

  @Override
  public EventQuery createQuery() {
    return jpa.createQuery();
  }

  @Override
  public AdminEventQuery createAdminQuery() {
    return jpa.createAdminQuery();
  }

  @Override
  public void clear() {
    jpa.clear();
  }

  @Override
  public void clear(RealmModel realm) {
    jpa.clear(realm);
  }

  @Override
  public void clear(RealmModel realm, long olderThan) {
    jpa.clear(realm, olderThan);
  }

  @Override
  public void clearExpiredEvents() {
    jpa.clearExpiredEvents();
  }

  @Override
  public void clearAdmin() {
    jpa.clearAdmin();
  }

  @Override
  public void clearAdmin(RealmModel realm) {
    jpa.clearAdmin(realm);
  }

  @Override
  public void clearAdmin(RealmModel realm, long olderThan) {
    jpa.clearAdmin(realm, olderThan);
  }

  @Override
  public void close() {
    // KeycloakSession owns and closes the explicitly obtained JPA provider.
  }
}
