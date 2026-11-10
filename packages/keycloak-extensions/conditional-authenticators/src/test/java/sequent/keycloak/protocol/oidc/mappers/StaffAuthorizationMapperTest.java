// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
package sequent.keycloak.protocol.oidc.mappers;

import static org.junit.jupiter.api.Assertions.*;

import java.util.HashSet;
import java.util.List;
import java.util.Set;
import org.junit.jupiter.api.Test;

class StaffAuthorizationMapperTest {
  @Test
  void anAdministratorCannotCombineBoardAndDataRoles() {
    Set<String> actual =
        StaffAuthorizationMapper.separated(
            Set.of(
                "admin-user",
                "role-write",
                "election-event-read",
                "voter-read",
                "sign-open-voting",
                "signing-requests-cancel",
                "electoral-log-console-read",
                "self-test-read"));
    assertEquals(Set.of("admin-user", "role-write", "self-test-read"), actual);
  }

  @Test
  void administratorsKeepHeldSettingsNavigationAndIndependentRoleEditing() {
    Set<String> settings =
        Set.of(
            "admin-user",
            "users-menu",
            "settings-menu",
            "templates-menu",
            "settings-election-types-tab",
            "settings-voting-channels-tab",
            "settings-templates-tab",
            "settings-languages-tab",
            "settings-localization-tab",
            "settings-look-feel-tab",
            "settings-trustees-tab",
            "settings-countries-tab",
            "user-permission-create",
            "user-permission-write");
    Set<String> combined = new java.util.HashSet<>(settings);
    combined.addAll(
        Set.of(
            "election-event-read",
            "permission-label-write",
            "trustees-export",
            "sign-open-voting",
            "election-event-create"));
    assertEquals(settings, StaffAuthorizationMapper.separated(combined));
    assertEquals(Set.of("admin-user"), StaffAuthorizationMapper.separated(Set.of("admin-user")));
  }

  @Test
  void administratorsCannotImportOrCreateElectionContexts() {
    assertEquals(
        Set.of("admin-user"),
        StaffAuthorizationMapper.separated(Set.of("admin-user", "election-event-create")));
  }

  @Test
  void administratorsKeepNoMessagingPermissions() {
    assertEquals(
        Set.of("admin-user"),
        StaffAuthorizationMapper.separated(
            Set.of(
                "admin-user",
                "messaging-config-read",
                "messaging-config-write",
                "messaging-account-read",
                "messaging-account-write")));
  }

  @Test
  void boardAccountsKeepTheirScopedDataAndApprovalPermissions() {
    Set<String> board =
        Set.of(
            "voter-read",
            "sign-open-voting",
            "signing-requests-cancel",
            "election-event-read",
            "election-event-create");
    assertEquals(board, StaffAuthorizationMapper.separated(board));
  }

  @Test
  void customAdministrativeGroupsCannotEvadeSeparationByOmittingLegacyRole() {
    assertEquals(
        Set.of("user-write"),
        StaffAuthorizationMapper.separated(Set.of("user-write", "tally-results-read")));
  }

  @Test
  void administratorsAndBoardMembersAreRecognisedFromTheirPermissions() {
    assertTrue(
        StaffAuthorizationMapper.administratorOrBoard(
            Set.of("admin-user", "tenant-write")::contains));
    assertTrue(
        StaffAuthorizationMapper.administratorOrBoard(
            Set.of("election-state-write", "voter-read")::contains));
    assertFalse(
        StaffAuthorizationMapper.administratorOrBoard(
            Set.of("election-state-write", "admin-user")::contains));
    assertFalse(
        StaffAuthorizationMapper.administratorOrBoard(
            Set.of("trustee-ceremony", "sign-key-ceremony", "sign-tally-key")::contains));
    assertFalse(StaffAuthorizationMapper.administratorOrBoard(Set.<String>of()::contains));
  }

  @Test
  void administratorOrBoardMatchesTheSeparatedTokenPermissions() {
    List<String> names =
        List.of(
            "admin-user",
            "role-write",
            "user-write",
            "signing-rules-write",
            "signing-issuers-write",
            "self-test-settings-write",
            "signing-checks-write",
            "signing-certificates-register",
            "signing-certificates-revoke",
            "role-create",
            "user-create",
            "tenant-write",
            "election-state-write",
            "trustee-ceremony",
            "voter-read");
    for (int mask = 0; mask < 1 << names.size(); mask++) {
      Set<String> held = new HashSet<>();
      for (int i = 0; i < names.size(); i++) if ((mask & 1 << i) != 0) held.add(names.get(i));
      Set<String> kept = StaffAuthorizationMapper.separated(held);
      assertEquals(
          kept.contains("tenant-write") || kept.contains("election-state-write"),
          StaffAuthorizationMapper.administratorOrBoard(held::contains),
          held::toString);
    }
  }

  @Test
  void onlyTenantRealmsAreStaffRealms() {
    assertTrue(StaffAuthorizationMapper.staffRealm("tenant-acme"));
    assertFalse(StaffAuthorizationMapper.staffRealm("tenant-acme-event-election"));
    assertFalse(StaffAuthorizationMapper.staffRealm("master"));
    assertFalse(StaffAuthorizationMapper.staffRealm(null));
  }
}
