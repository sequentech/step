// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use sequent_plugin_api::{Manifest, Operation, PluginError};
use sequent_plugin_host::{authorize, Identity};
use uuid::Uuid;

fn manifest() -> Manifest {
    Manifest {
        id: "example",
        version: "1.0.0",
        api_major: 1,
        data_version: 1,
        label_key: "example.title",
        route: "/plugins/example",
        read_permission: "example-read",
        permissions: &["example-read", "example-write"],
        record_kinds: &["item"],
        operations: &[
            Operation {
                name: "list",
                permission: "example-read",
            },
            Operation {
                name: "save",
                permission: "example-write",
            },
        ],
    }
}

#[test]
fn reader_cannot_write_and_admin_user_alone_grants_no_plugin_permission() {
    let mut identity = Identity {
        tenant_id: Uuid::new_v4(),
        user_id: "user".into(),
        roles: vec!["admin-user".into(), "example-read".into()],
    };
    assert!(authorize(&manifest(), "list", &identity).is_ok());
    assert!(matches!(
        authorize(&manifest(), "save", &identity),
        Err(PluginError::Forbidden)
    ));
    identity.roles = vec!["admin-user".into()];
    assert!(matches!(
        authorize(&manifest(), "list", &identity),
        Err(PluginError::Forbidden)
    ));
}

#[test]
fn plugin_roles_do_not_admit_voter_tokens_or_unknown_operations() {
    let identity = Identity {
        tenant_id: Uuid::new_v4(),
        user_id: "voter".into(),
        roles: vec!["example-read".into()],
    };
    assert!(matches!(
        authorize(&manifest(), "list", &identity),
        Err(PluginError::Forbidden)
    ));
    assert!(matches!(
        authorize(&manifest(), "sql", &identity),
        Err(PluginError::NotFound)
    ));
}

#[test]
fn unsupported_api_is_rejected_before_dispatch() {
    let identity = Identity {
        tenant_id: Uuid::new_v4(),
        user_id: "user".into(),
        roles: vec!["admin-user".into(), "example-read".into()],
    };
    let mut plugin = manifest();
    plugin.api_major = 2;
    assert!(matches!(
        authorize(&plugin, "list", &identity),
        Err(PluginError::Incompatible)
    ));
}
