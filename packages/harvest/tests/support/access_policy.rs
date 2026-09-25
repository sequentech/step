// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Permission tables for the user, document and ceremony routes. Vectors are
//! compared whole: their order is the order a denial lists them in.

use super::*;
use crate::services::authorization::authorize;
use crate::test_claims::Claims;
use sequent_core::types::permissions::Permissions::*;

const TENANT_ID: &str = "tenant-a";
const USER_ID: &str = "test-user";

fn secrets(values: Option<Vec<String>>) -> SecretAttributes {
    HashMap::from([("test-secret".to_string(), values)])
}

fn labels() -> Attributes {
    HashMap::from([(PERMISSION_LABELS.to_string(), vec!["north".to_string()])])
}

fn roles(permissions: &[Permissions]) -> Vec<String> {
    permissions.iter().map(ToString::to_string).collect()
}

/// An edit that sets nothing, to be completed with struct update syntax.
fn no_changes(scope: UserScope) -> UserEdit<'static> {
    UserEdit {
        scope,
        enabled: None,
        attributes: None,
        secret_attributes: None,
        email: None,
        first_name: None,
        last_name: None,
        username: None,
        password: None,
    }
}

#[test]
fn any_election_event_id_addresses_its_voters() {
    assert_eq!(UserScope::of(Some("test-event")), UserScope::Voters);
    // An empty id is still an id.
    assert_eq!(UserScope::of(Some("")), UserScope::Voters);
    assert_eq!(UserScope::of(None), UserScope::Tenant);
}

#[test]
fn voters_and_tenant_users_need_their_own_read_create_and_delete_permissions() {
    assert_eq!(
        [UserScope::Voters, UserScope::Tenant].map(|scope| (
            read_permission(scope),
            create_permission(scope),
            delete_permission(scope),
        )),
        [
            (VOTER_READ, VOTER_CREATE, VOTER_DELETE),
            (USER_READ, USER_CREATE, USER_WRITE),
        ]
    );
}

#[test]
fn creating_a_user_adds_the_secret_and_label_permissions_its_body_needs() {
    let empty = SecretAttributes::new();
    let set = secrets(Some(vec!["value".into()]));
    let cleared = secrets(None);
    let other = HashMap::from([("ward".to_string(), vec!["7".to_string()])]);
    let labels = labels();
    for (scope, secret_attributes, attributes, expected) in [
        (UserScope::Voters, None, None, vec![VOTER_CREATE]),
        (UserScope::Voters, Some(&empty), None, vec![VOTER_CREATE]),
        (
            UserScope::Voters,
            Some(&set),
            None,
            vec![VOTER_CREATE, VOTER_SECRET_ATTRIBUTE_WRITE],
        ),
        (
            UserScope::Voters,
            Some(&cleared),
            None,
            vec![VOTER_CREATE, VOTER_SECRET_ATTRIBUTE_WRITE],
        ),
        // Permission labels are only checked for tenant users.
        (UserScope::Voters, None, Some(&labels), vec![VOTER_CREATE]),
        (UserScope::Tenant, None, None, vec![USER_CREATE]),
        (UserScope::Tenant, Some(&empty), None, vec![USER_CREATE]),
        (UserScope::Tenant, None, Some(&other), vec![USER_CREATE]),
        (
            UserScope::Tenant,
            None,
            Some(&labels),
            vec![USER_CREATE, PERMISSION_LABEL_WRITE],
        ),
    ] {
        assert_eq!(
            create_user_permissions(scope, secret_attributes, attributes),
            Ok(expected),
            "{scope:?} {secret_attributes:?} {attributes:?}"
        );
    }
}

#[test]
fn tenant_users_cannot_be_created_or_edited_with_encrypted_attributes() {
    for values in [Some(vec!["value".to_string()]), None] {
        let secret_attributes = secrets(values);
        assert_eq!(
            create_user_permissions(
                UserScope::Tenant,
                Some(&secret_attributes),
                Some(&labels()),
            ),
            Err(SecretAttributesOutsideElectionEvent)
        );
        let edit = UserEdit {
            secret_attributes: Some(&secret_attributes),
            ..no_changes(UserScope::Tenant)
        };
        assert_eq!(
            edit_user_access(&edit, &roles(&[USER_WRITE])),
            Err(SecretAttributesOutsideElectionEvent)
        );
    }
    assert_eq!(
        SecretAttributesOutsideElectionEvent.to_string(),
        "Encrypted attributes are only supported for election-event voters"
    );
}

#[test]
fn a_voter_edit_needs_voter_write_or_the_email_and_phone_permission() {
    let set = secrets(Some(vec!["value".into()]));
    let profile = UserEdit {
        first_name: Some("Ada"),
        ..no_changes(UserScope::Voters)
    };
    let with_password = UserEdit {
        password: Some("secret"),
        ..profile
    };
    let with_secrets = UserEdit {
        secret_attributes: Some(&set),
        ..no_changes(UserScope::Voters)
    };
    let with_password_and_secrets = UserEdit {
        password: Some("secret"),
        secret_attributes: Some(&set),
        ..no_changes(UserScope::Voters)
    };
    for (edit, held, expected) in [
        (&profile, vec![VOTER_WRITE], vec![VOTER_WRITE]),
        (&profile, vec![], vec![VOTER_EMAIL_TLF_EDIT]),
        (
            &profile,
            vec![VOTER_EMAIL_TLF_EDIT],
            vec![VOTER_EMAIL_TLF_EDIT],
        ),
        (
            &with_password,
            vec![VOTER_WRITE],
            vec![VOTER_WRITE, VOTER_CHANGE_PASSWORD],
        ),
        (
            &with_password,
            vec![],
            vec![VOTER_EMAIL_TLF_EDIT, VOTER_CHANGE_PASSWORD],
        ),
        (
            &with_secrets,
            vec![VOTER_WRITE],
            vec![VOTER_WRITE, VOTER_SECRET_ATTRIBUTE_WRITE],
        ),
        // Encrypted attributes still need VOTER_WRITE, after the others.
        (
            &with_secrets,
            vec![VOTER_EMAIL_TLF_EDIT],
            vec![
                VOTER_EMAIL_TLF_EDIT,
                VOTER_SECRET_ATTRIBUTE_WRITE,
                VOTER_WRITE,
            ],
        ),
        (
            &with_password_and_secrets,
            vec![VOTER_WRITE],
            vec![
                VOTER_WRITE,
                VOTER_CHANGE_PASSWORD,
                VOTER_SECRET_ATTRIBUTE_WRITE,
            ],
        ),
    ] {
        let access = edit_user_access(edit, &roles(&held)).unwrap();
        assert_eq!(access.permissions, expected, "holding {held:?}");
        assert!(!access.password_only);
    }
}

#[test]
fn a_password_alone_needs_only_the_password_permission() {
    for held in [vec![], vec![VOTER_WRITE]] {
        let access = edit_user_access(
            &UserEdit {
                password: Some("secret"),
                ..no_changes(UserScope::Voters)
            },
            &roles(&held),
        )
        .unwrap();
        assert_eq!(access.permissions, vec![VOTER_CHANGE_PASSWORD]);
        assert!(access.password_only);
    }
}

#[test]
fn any_other_field_set_with_the_password_makes_it_a_profile_edit() {
    let empty_attributes = Attributes::new();
    let empty_secrets = SecretAttributes::new();
    let password = UserEdit {
        password: Some("secret"),
        ..no_changes(UserScope::Voters)
    };
    // Even an empty map counts as set.
    for other_field in [
        UserEdit {
            enabled: Some(true),
            ..password
        },
        UserEdit {
            attributes: Some(&empty_attributes),
            ..password
        },
        UserEdit {
            secret_attributes: Some(&empty_secrets),
            ..password
        },
        UserEdit {
            email: Some("ada@example.invalid"),
            ..password
        },
        UserEdit {
            first_name: Some("Ada"),
            ..password
        },
        UserEdit {
            last_name: Some("Lovelace"),
            ..password
        },
        UserEdit {
            username: Some("ada"),
            ..password
        },
    ] {
        let access = edit_user_access(&other_field, &[]).unwrap();
        assert!(!access.password_only);
        assert_eq!(
            access.permissions,
            vec![VOTER_EMAIL_TLF_EDIT, VOTER_CHANGE_PASSWORD]
        );
    }
}

#[test]
fn a_voter_edit_without_a_password_is_never_password_only() {
    let access = edit_user_access(&no_changes(UserScope::Voters), &[]).unwrap();
    assert!(!access.password_only);
    assert_eq!(access.permissions, vec![VOTER_EMAIL_TLF_EDIT]);
}

#[test]
fn a_tenant_user_edit_needs_user_write_and_label_permissions() {
    let labels = labels();
    let empty_secrets = SecretAttributes::new();
    for (edit, expected) in [
        (no_changes(UserScope::Tenant), vec![USER_WRITE]),
        // Passwords of tenant users need no extra permission.
        (
            UserEdit {
                password: Some("secret"),
                ..no_changes(UserScope::Tenant)
            },
            vec![USER_WRITE],
        ),
        (
            UserEdit {
                secret_attributes: Some(&empty_secrets),
                ..no_changes(UserScope::Tenant)
            },
            vec![USER_WRITE],
        ),
        (
            UserEdit {
                attributes: Some(&labels),
                ..no_changes(UserScope::Tenant)
            },
            vec![USER_WRITE, PERMISSION_LABEL_WRITE],
        ),
    ] {
        let access = edit_user_access(&edit, &[]).unwrap();
        assert_eq!(access.permissions, expected);
        assert!(!access.password_only);
    }
    // Voters get no label check.
    let access = edit_user_access(
        &UserEdit {
            attributes: Some(&labels),
            ..no_changes(UserScope::Voters)
        },
        &roles(&[VOTER_WRITE]),
    )
    .unwrap();
    assert_eq!(access.permissions, vec![VOTER_WRITE]);
}

#[test]
fn voter_edit_capabilities_come_from_the_caller_roles() {
    let held = roles(&[VOTER_VOTED_EDIT, VOTER_EMAIL_TLF_EDIT]);
    let access =
        edit_user_access(&no_changes(UserScope::Voters), &held).unwrap();
    assert!(access.voter_voted_edit);
    assert!(access.voter_email_tlf_edit);

    let access = edit_user_access(&no_changes(UserScope::Voters), &[]).unwrap();
    assert!(!access.voter_voted_edit);
    assert!(!access.voter_email_tlf_edit);

    // They only apply to voters.
    let access =
        edit_user_access(&no_changes(UserScope::Tenant), &held).unwrap();
    assert!(!access.voter_voted_edit);
    assert!(!access.voter_email_tlf_edit);
}

#[test]
fn denials_list_the_permissions_in_the_order_the_routes_request_them() {
    // Holding no role keeps the set printed after "not in" empty.
    let claims = Claims::new(TENANT_ID, USER_ID).build();
    let denial = |permissions: Vec<Permissions>| {
        authorize(&claims, true, Some(TENANT_ID.into()), permissions)
            .unwrap_err()
            .1
    };
    let set = secrets(Some(vec!["value".into()]));
    assert_eq!(
        denial(
            create_user_permissions(UserScope::Voters, Some(&set), None)
                .unwrap()
        ),
        r#"Unathorized: ["voter-create", "voter-secret-attribute-write"] not in {}"#
    );
    let edit = UserEdit {
        password: Some("secret"),
        secret_attributes: Some(&set),
        ..no_changes(UserScope::Voters)
    };
    assert_eq!(
        denial(edit_user_access(&edit, &[]).unwrap().permissions),
        r#"Unathorized: ["voter-email-tlf-edit", "voter-change-password", "voter-secret-attribute-write", "voter-write"] not in {}"#
    );
}

#[test]
fn only_documents_holding_voter_secrets_need_the_secret_read_permission() {
    for (annotations, expected) in [
        (None, vec![]),
        (Some(DocumentAnnotations::default()), vec![]),
        (
            Some(DocumentAnnotations::password_protected("secret-id")),
            vec![],
        ),
        (
            Some(DocumentAnnotations::voter_secret_export()),
            vec![VOTER_SECRET_ATTRIBUTE_READ],
        ),
    ] {
        assert_eq!(
            document_extra_permissions(annotations.as_ref()),
            expected,
            "{annotations:?}"
        );
    }
}

#[test]
fn either_passing_check_grants_access_and_otherwise_the_second_denial_wins() {
    let denied = |message: &str| Err((Status::Unauthorized, message.into()));
    assert_eq!(authorize_any(Ok(()), Ok(())), Ok(()));
    assert_eq!(authorize_any(Ok(()), denied("second")), Ok(()));
    assert_eq!(authorize_any(denied("first"), Ok(())), Ok(()));
    assert_eq!(
        authorize_any(denied("first"), denied("second")),
        denied("second")
    );
}

#[test]
fn keys_ceremonies_are_listed_for_admins_or_trustees() {
    let ceremony = |held: &[Permissions]| {
        let claims = Claims::new(TENANT_ID, USER_ID).roles(held).build();
        let check = |permission| {
            authorize(&claims, true, Some(TENANT_ID.into()), vec![permission])
        };
        authorize_any(check(ADMIN_CEREMONY), check(TRUSTEE_CEREMONY))
    };
    assert_eq!(ceremony(&[ADMIN_CEREMONY]), Ok(()));
    assert_eq!(ceremony(&[TRUSTEE_CEREMONY]), Ok(()));
    assert_eq!(
        ceremony(&[]),
        Err((
            Status::Unauthorized,
            r#"Unathorized: ["trustee-ceremony"] not in {}"#.to_string()
        ))
    );
}
