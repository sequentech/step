// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.voter_enrollment;

import com.google.auto.service.AutoService;
import jakarta.ws.rs.core.MultivaluedMap;
import java.time.Clock;
import java.time.Instant;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import lombok.extern.jbosslog.JBossLog;
import org.keycloak.Config;
import org.keycloak.authentication.FormAction;
import org.keycloak.authentication.FormActionFactory;
import org.keycloak.authentication.FormContext;
import org.keycloak.authentication.ValidationContext;
import org.keycloak.events.Errors;
import org.keycloak.forms.login.LoginFormsProvider;
import org.keycloak.models.AuthenticationExecutionModel;
import org.keycloak.models.KeycloakSession;
import org.keycloak.models.KeycloakSessionFactory;
import org.keycloak.models.RealmModel;
import org.keycloak.models.UserModel;
import org.keycloak.models.utils.FormMessage;
import org.keycloak.provider.ProviderConfigProperty;

/**
 * Refuses a registration for a Post whose enrollment is not open (VOTE-LIFECYCLE §8), and gives
 * register.ftl each Post's window so it can show the times and hold Continue back. The windows come
 * from the realm attribute {@value EnrollmentWindows#REALM_ATTRIBUTE}; a Post without one has no
 * enrollment schedule and is let through (the realm's registration switch still applies). A window
 * that can't be checked refuses the registration.
 */
@JBossLog
@AutoService(FormActionFactory.class)
public class EnrollmentWindowCheck implements FormAction, FormActionFactory {

  public static final String PROVIDER_ID = "enrollment-window-check";

  /** The register.ftl model attribute: one map per Post (see {@link #pageModel}). */
  public static final String PAGE_ATTRIBUTE = "enrollmentWindows";

  private final Clock clock;

  public EnrollmentWindowCheck() {
    this(Clock.systemUTC());
  }

  EnrollmentWindowCheck(Clock clock) {
    this.clock = clock;
  }

  @Override
  public void buildPage(FormContext context, LoginFormsProvider form) {
    EnrollmentWindows.Windows windows = EnrollmentWindows.read(context.getRealm());
    if (windows.byPost().isEmpty()) {
      return;
    }
    Locale locale = context.getSession().getContext().resolveLocale(null);
    form.setAttribute(PAGE_ATTRIBUTE, pageModel(windows.byPost(), clock.instant(), locale));
  }

  /**
   * One map per Post: {@code embassy}, {@code state} (before, open, closed or not-configured), and
   * for each limited side its time and zone: the opening in the Post's zone ({@code opens}, {@code
   * zone}, {@code opensZoneName}), the common close in the primary zone ({@code closes}, {@code
   * closeZone}, {@code closesZoneName}). The zone names are the defaults at that time.
   */
  static List<Map<String, String>> pageModel(
      Map<String, EnrollmentWindows.Window> windows, Instant now, Locale locale) {
    List<Map<String, String>> posts = new ArrayList<>();
    windows.forEach(
        (post, window) -> {
          Map<String, String> entry = new HashMap<>();
          entry.put("embassy", post);
          entry.put("state", window.stateAt(now).wireValue());
          if (window.problem() == null) {
            entry.put("zone", window.zone().getId());
            entry.put("closeZone", window.closeZone().getId());
          }
          if (window.opensAt() != null) {
            entry.put(
                "opens", EnrollmentWindows.formatDateTime(window.opensAt(), window.zone(), locale));
            entry.put(
                "opensZoneName",
                EnrollmentWindows.zoneName(window.opensAt(), window.zone(), locale));
          }
          if (window.closesAt() != null) {
            entry.put(
                "closes",
                EnrollmentWindows.formatDateTime(window.closesAt(), window.closeZone(), locale));
            entry.put(
                "closesZoneName",
                EnrollmentWindows.zoneName(window.closesAt(), window.closeZone(), locale));
          }
          posts.add(entry);
        });
    return posts;
  }

  @Override
  public void validate(ValidationContext context) {
    EnrollmentWindows.Windows windows = EnrollmentWindows.read(context.getRealm());
    MultivaluedMap<String, String> formData = context.getHttpRequest().getDecodedFormParameters();
    String post = formData.getFirst(EnrollmentWindows.POST_FIELD);
    EnrollmentWindows.Window window = windows.lookup(post);
    if (window == null) {
      context.success();
      return;
    }
    EnrollmentWindows.State state = window.stateAt(clock.instant());
    if (state == EnrollmentWindows.State.OPEN) {
      context.success();
      return;
    }
    String message;
    if (state == EnrollmentWindows.State.NOT_CONFIGURED) {
      message = EnrollmentWindows.NOT_CONFIGURED_MESSAGE;
      log.errorv(
          "Registration refused: the enrollment window of {0} can''t be checked ({1})",
          post, window.problem());
    } else {
      message = EnrollmentWindows.NOT_OPEN_MESSAGE;
      log.infov(
          "Registration refused: enrollment for {0} is {1} (opens {2}, closes {3})",
          post, state.wireValue(), window.opensAt(), window.closesAt());
    }
    if (post != null) {
      context.getEvent().detail(EnrollmentWindows.POST_FIELD, post);
    }
    context.error(Errors.INVALID_REGISTRATION);
    context.validationError(
        formData,
        List.of(new FormMessage(EnrollmentWindows.POST_FIELD, message, post == null ? "" : post)));
  }

  @Override
  public void success(FormContext context) {}

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
  public FormAction create(KeycloakSession session) {
    return this;
  }

  @Override
  public String getDisplayType() {
    return "Sequent: Enrollment window per Post";
  }

  @Override
  public String getReferenceCategory() {
    return null;
  }

  @Override
  public boolean isConfigurable() {
    return false;
  }

  @Override
  public AuthenticationExecutionModel.Requirement[] getRequirementChoices() {
    return new AuthenticationExecutionModel.Requirement[] {
      AuthenticationExecutionModel.Requirement.REQUIRED,
      AuthenticationExecutionModel.Requirement.DISABLED
    };
  }

  @Override
  public boolean isUserSetupAllowed() {
    return false;
  }

  @Override
  public String getHelpText() {
    return "Refuses a registration when the chosen Post (the embassy attribute) is outside its "
        + "enrollment window, read from the realm attribute enrollment_windows, and gives the "
        + "registration page each Post's opening and closing time.";
  }

  @Override
  public List<ProviderConfigProperty> getConfigProperties() {
    return List.of();
  }

  @Override
  public String getId() {
    return PROVIDER_ID;
  }

  @Override
  public void init(Config.Scope config) {}

  @Override
  public void postInit(KeycloakSessionFactory factory) {}

  @Override
  public void close() {}
}
