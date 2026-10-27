// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.protocol.oidc.mappers;
import static org.junit.jupiter.api.Assertions.*;
import java.util.Set;
import org.junit.jupiter.api.Test;
class StaffAuthorizationMapperTest {
  @Test void anAdministratorCannotCombineBoardAndDataRoles() {
    Set<String> actual=StaffAuthorizationMapper.separated(Set.of("admin-user","role-write","election-event-read",
        "voter-read","sign-open-voting","signing-requests-cancel","electoral-log-console-read","self-test-read"));
    assertEquals(Set.of("admin-user","role-write","self-test-read"),actual);
  }
  @Test void administratorsKeepHeldSettingsNavigationAndIndependentRoleEditing() {
    Set<String> settings=Set.of("admin-user","users-menu","settings-menu","templates-menu",
        "settings-election-types-tab","settings-voting-channels-tab","settings-templates-tab",
        "settings-languages-tab","settings-localization-tab","settings-look-feel-tab",
        "settings-trustees-tab","settings-countries-tab","user-permission-create","user-permission-write");
    Set<String> combined=new java.util.HashSet<>(settings);
    combined.addAll(Set.of("election-event-read","permission-label-write","trustees-export","sign-open-voting","election-event-create"));
    assertEquals(settings,StaffAuthorizationMapper.separated(combined));
    assertEquals(Set.of("admin-user"),StaffAuthorizationMapper.separated(Set.of("admin-user")));
  }
  @Test void administratorsCannotImportOrCreateElectionContexts() {
    assertEquals(Set.of("admin-user"),StaffAuthorizationMapper.separated(Set.of("admin-user","election-event-create")));
  }
  @Test void boardAccountsKeepTheirScopedDataAndApprovalPermissions() {
    Set<String> board=Set.of("voter-read","sign-open-voting","signing-requests-cancel","election-event-read","election-event-create");
    assertEquals(board,StaffAuthorizationMapper.separated(board));
  }
  @Test void customAdministrativeGroupsCannotEvadeSeparationByOmittingLegacyRole() {
    assertEquals(Set.of("user-write"),StaffAuthorizationMapper.separated(Set.of("user-write","tally-results-read")));
  }
}
