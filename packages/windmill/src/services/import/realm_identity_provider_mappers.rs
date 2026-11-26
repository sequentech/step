// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Which identity-provider mappers an imported election-event realm may keep
//! although the platform template doesn't define them.
//!
//! Brokered events need their own mappers: the voter's username, group, tenant
//! and area come from the identity provider. A mapper is kept when it can only
//! give a brokered user what the import already grants: a template group, the
//! importing tenant, an area of the imported event, or user attributes no
//! token claim reads. Anything that could grant roles or other token claims
//! is still removed.

use keycloak::types::{
    GroupRepresentation, IdentityProviderMapperRepresentation, RealmRepresentation,
};
use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

/// Sequent's lookup mapper (`packages/keycloak-extensions/idp-attribute-mapper`).
const LOOKUP_MAPPER: &str = "sequent-attribute-lookup-idp-mapper";
const HARDCODED_ATTRIBUTE_MAPPER: &str = "hardcoded-attribute-idp-mapper";
const HARDCODED_GROUP_MAPPER: &str = "oidc-hardcoded-group-idp-mapper";
/// Mappers that copy a value sent by the identity provider into a user attribute.
const ATTRIBUTE_IMPORTERS: [&str; 2] = [
    "saml-user-attribute-idp-mapper",
    "oidc-user-attribute-idp-mapper",
];
/// Mappers that only build the username of a user created on first login.
const USERNAME_MAPPERS: [&str; 2] = ["saml-username-idp-mapper", "oidc-username-idp-mapper"];

const TENANT_ID_ATTRIBUTE: &str = "tenant-id";
const AREA_ID_ATTRIBUTE: &str = "area-id";

static UUID_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[0-9a-fA-F]{8}(-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}").expect("valid UUID regex")
});

/// What the import grants, against which imported mappers are checked.
#[derive(Debug, Clone, Default)]
pub struct ImportedRealmScope {
    /// Tenant the event is imported into.
    pub tenant_id: String,
    /// Ids of the imported event's areas, after id replacement.
    pub area_ids: HashSet<String>,
}

/// What the template allows, read once per import.
pub(super) struct IdpMapperPolicy<'a> {
    scope: &'a ImportedRealmScope,
    /// User attributes the template's token-claim mappers read.
    claim_attributes: HashSet<String>,
    /// Paths of the template's groups.
    template_groups: HashSet<String>,
    /// Identity providers of the imported realm.
    identity_providers: HashSet<String>,
}

impl<'a> IdpMapperPolicy<'a> {
    pub(super) fn new(
        template: &RealmRepresentation,
        imported: &RealmRepresentation,
        claim_attributes: HashSet<String>,
        scope: &'a ImportedRealmScope,
    ) -> Self {
        let mut template_groups = HashSet::new();
        collect_group_paths(
            template.groups.as_deref().unwrap_or_default(),
            &mut template_groups,
        );
        let identity_providers = imported
            .identity_providers
            .iter()
            .flatten()
            .filter_map(|provider| provider.alias.clone())
            .collect();
        Self {
            scope,
            claim_attributes,
            template_groups,
            identity_providers,
        }
    }

    /// `None` when the mapper may stay, otherwise why it is removed.
    pub(super) fn rejection(
        &self,
        mapper: &IdentityProviderMapperRepresentation,
    ) -> Option<String> {
        let alias = mapper
            .identity_provider_alias
            .as_deref()
            .unwrap_or_default();
        if !self.identity_providers.contains(alias) {
            return Some(format!(
                "its identity provider '{alias}' isn't in the realm"
            ));
        }
        let kind = mapper
            .identity_provider_mapper
            .as_deref()
            .unwrap_or_default();
        let config = |key: &str| {
            mapper
                .config
                .as_ref()
                .and_then(|config| config.get(key))
                .map(String::as_str)
                .unwrap_or_default()
        };
        match kind {
            LOOKUP_MAPPER => {
                if config("target").trim().eq_ignore_ascii_case("USERNAME") {
                    return None;
                }
                // The values it can write: the table's and the default.
                let values = format!("{}\n{}", config("lookup.table"), config("default.value"));
                self.attribute_rejection(
                    config("target.attribute"),
                    AttributeValues::Listed(&values),
                )
            }
            HARDCODED_ATTRIBUTE_MAPPER => self.attribute_rejection(
                config("attribute"),
                AttributeValues::Exactly(config("attribute.value")),
            ),
            kind if ATTRIBUTE_IMPORTERS.contains(&kind) => {
                self.attribute_rejection(config("user.attribute"), AttributeValues::FromProvider)
            }
            kind if USERNAME_MAPPERS.contains(&kind) => None,
            HARDCODED_GROUP_MAPPER => {
                let group = config("group");
                (!self.template_groups.contains(group))
                    .then(|| format!("group '{group}' isn't a template group"))
            }
            _ => Some(format!("mapper type '{kind}' can grant roles or claims")),
        }
    }

    fn attribute_rejection(&self, attribute: &str, values: AttributeValues) -> Option<String> {
        let attribute = attribute.trim();
        if attribute.is_empty() {
            return Some("it names no user attribute".to_string());
        }
        if !self.claim_attributes.contains(attribute) {
            return None;
        }
        let allowed = match (attribute, values) {
            (TENANT_ID_ATTRIBUTE, AttributeValues::Exactly(value)) => {
                value.trim() == self.scope.tenant_id
            }
            (AREA_ID_ATTRIBUTE, AttributeValues::Exactly(value)) => {
                self.scope.area_ids.contains(value.trim())
            }
            // Every id the mapper can write must be one of this event's areas.
            (AREA_ID_ATTRIBUTE, AttributeValues::Listed(values)) => UUID_PATTERN
                .find_iter(values)
                .all(|id| self.scope.area_ids.contains(id.as_str())),
            _ => false,
        };
        (!allowed).then(|| {
            format!(
                "it sets token-claim attribute '{attribute}' to values outside the imported event"
            )
        })
    }
}

enum AttributeValues<'a> {
    /// A single fixed value.
    Exactly(&'a str),
    /// Text holding every value the mapper can write.
    Listed(&'a str),
    /// Whatever the identity provider sends.
    FromProvider,
}

fn collect_group_paths(groups: &[GroupRepresentation], paths: &mut HashSet<String>) {
    for group in groups {
        if let Some(path) = &group.path {
            paths.insert(path.clone());
        } else if let Some(name) = &group.name {
            paths.insert(format!("/{name}"));
        }
        collect_group_paths(group.sub_groups.as_deref().unwrap_or_default(), paths);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keycloak::types::IdentityProviderRepresentation;
    use std::collections::HashMap;

    const TENANT: &str = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
    const OTHER_TENANT: &str = "11111111-2222-4333-8444-555555555555";
    const AREA_A: &str = "2e4c79f5-be4f-4ab3-8c9b-acc000000001";
    const AREA_B: &str = "2e4c79f5-be4f-4ab3-8c9b-acc000000002";
    const FOREIGN_AREA: &str = "dda7b96c-9a29-494a-8b86-02b000000003";

    fn scope() -> ImportedRealmScope {
        ImportedRealmScope {
            tenant_id: TENANT.to_string(),
            area_ids: [AREA_A, AREA_B].map(str::to_string).into(),
        }
    }

    fn template() -> RealmRepresentation {
        RealmRepresentation {
            groups: Some(vec![GroupRepresentation {
                name: Some("voter".to_string()),
                path: Some("/voter".to_string()),
                ..Default::default()
            }]),
            ..Default::default()
        }
    }

    fn imported() -> RealmRepresentation {
        RealmRepresentation {
            identity_providers: Some(vec![IdentityProviderRepresentation {
                alias: Some("idp".to_string()),
                ..Default::default()
            }]),
            ..Default::default()
        }
    }

    fn claim_attributes() -> HashSet<String> {
        ["tenant-id", "area-id", "authorized-election-ids"]
            .map(str::to_string)
            .into()
    }

    fn mapper(kind: &str, config: &[(&str, &str)]) -> IdentityProviderMapperRepresentation {
        IdentityProviderMapperRepresentation {
            name: Some("m".to_string()),
            identity_provider_alias: Some("idp".to_string()),
            identity_provider_mapper: Some(kind.to_string()),
            config: Some(
                config
                    .iter()
                    .map(|(key, value)| (key.to_string(), value.to_string()))
                    .collect::<HashMap<_, _>>(),
            ),
            ..Default::default()
        }
    }

    fn rejection(mapper: &IdentityProviderMapperRepresentation) -> Option<String> {
        let scope = scope();
        IdpMapperPolicy::new(&template(), &imported(), claim_attributes(), &scope).rejection(mapper)
    }

    #[test]
    fn keeps_a_lookup_into_the_events_areas() {
        let table = format!("# area = keys\n{AREA_A} = 01:0001..01:0100\n02:0001,{AREA_B}");
        assert_eq!(
            rejection(&mapper(
                LOOKUP_MAPPER,
                &[("target.attribute", "area-id"), ("lookup.table", &table)]
            )),
            None
        );
    }

    #[test]
    fn drops_a_lookup_or_default_into_another_events_area() {
        let table = format!("{AREA_A} = 1\n{FOREIGN_AREA} = 2");
        assert!(rejection(&mapper(
            LOOKUP_MAPPER,
            &[("target.attribute", "area-id"), ("lookup.table", &table)]
        ))
        .is_some());
        assert!(rejection(&mapper(
            LOOKUP_MAPPER,
            &[
                ("target.attribute", "area-id"),
                ("lookup.table", &format!("{AREA_A} = 1")),
                ("on.miss", "DEFAULT"),
                ("default.value", FOREIGN_AREA),
            ]
        ))
        .is_some());
    }

    #[test]
    fn keeps_username_and_non_claim_lookups() {
        assert_eq!(
            rejection(&mapper(
                LOOKUP_MAPPER,
                &[("target", "USERNAME"), ("sources", "$nameid")]
            )),
            None
        );
        assert_eq!(
            rejection(&mapper(
                LOOKUP_MAPPER,
                &[("target.attribute", "precinct"), ("sources", "precinct")]
            )),
            None
        );
    }

    #[test]
    fn drops_lookups_into_other_claim_attributes() {
        for attribute in ["tenant-id", "authorized-election-ids"] {
            assert!(rejection(&mapper(
                LOOKUP_MAPPER,
                &[
                    ("target.attribute", attribute),
                    ("lookup.table", &format!("{TENANT} = 1"))
                ]
            ))
            .is_some());
        }
    }

    #[test]
    fn keeps_the_importing_tenant_and_drops_another() {
        let hardcoded = |value| {
            mapper(
                HARDCODED_ATTRIBUTE_MAPPER,
                &[("attribute", "tenant-id"), ("attribute.value", value)],
            )
        };
        assert_eq!(rejection(&hardcoded(TENANT)), None);
        assert!(rejection(&hardcoded(OTHER_TENANT)).is_some());
    }

    #[test]
    fn keeps_template_groups_only() {
        assert_eq!(
            rejection(&mapper(HARDCODED_GROUP_MAPPER, &[("group", "/voter")])),
            None
        );
        assert!(rejection(&mapper(HARDCODED_GROUP_MAPPER, &[("group", "/admins")])).is_some());
    }

    #[test]
    fn importers_may_not_write_claim_attributes() {
        let importer = |attribute| {
            mapper(
                "saml-user-attribute-idp-mapper",
                &[("attribute.name", "x"), ("user.attribute", attribute)],
            )
        };
        assert_eq!(rejection(&importer("firstName")), None);
        assert!(rejection(&importer("area-id")).is_some());
    }

    #[test]
    fn drops_role_mappers_unknown_types_and_orphans() {
        assert!(rejection(&mapper("oidc-role-idp-mapper", &[("role", "admin-user")])).is_some());
        assert!(rejection(&mapper("hardcoded-role-idp-mapper", &[("role", "user")])).is_some());
        let mut orphan = mapper(USERNAME_MAPPERS[0], &[]);
        orphan.identity_provider_alias = Some("gone".to_string());
        assert!(rejection(&orphan).is_some());
    }
}
