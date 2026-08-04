// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

package sequent.keycloak.authenticator.forgot_password;

import jakarta.ws.rs.core.MultivaluedMap;
import java.util.stream.Stream;
import org.keycloak.forms.login.freemarker.model.AbstractUserProfileBean;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.UserModel;
import org.keycloak.userprofile.UserProfile;
import org.keycloak.userprofile.UserProfileContext;
import org.keycloak.userprofile.UserProfileProvider;

/**
 * Exposes the realm's User Profile attribute declarations to {@code login.ftl}'s {@code
 * matchAttributes} rendering, the same {@code profile.attributesByName[...]} shape {@code
 * register.ftl} already consumes via {@code user-profile-commons.ftl} - so a matched attribute
 * (e.g. {@code dateOfBirth}) gets the same field type, helper text, and required marker at login as
 * it does at registration.
 *
 * <p>Mirrors Keycloak's own {@code RegisterBean}: built with {@link
 * UserProfileContext#REGISTRATION} and no {@link UserModel}, since no user is resolved yet at the
 * point this form is rendered - the submitted values are what {@link
 * MultiAttributeCredentialResolver} uses to find one. Unlike {@code RegisterBean}, this passes
 * {@code writeableOnly=false} to {@link #init}: this form never persists anything through {@link
 * UserProfileProvider} (matching happens via a direct user-store search, not {@code
 * UserProfile.update()}), so an attribute a voter can't self-edit (write-restricted) should still
 * be usable to identify themselves for login, not silently hidden the way an actual registration
 * form must hide it.
 */
public class LoginBean extends AbstractUserProfileBean {

  public LoginBean(MultivaluedMap<String, String> formData, KeycloakSession session) {
    super(formData);
    init(session, false);
  }

  @Override
  protected UserProfile createUserProfile(UserProfileProvider provider) {
    return provider.create(UserProfileContext.REGISTRATION, null, (UserModel) null);
  }

  @Override
  protected Stream<String> getAttributeDefaultValues(String name) {
    return null;
  }

  @Override
  public String getContext() {
    return "MULTI_ATTRIBUTE_LOGIN";
  }
}
