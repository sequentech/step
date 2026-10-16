// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::voter_secret_attributes::strip_undeclared_secret_attributes;
use sequent_core::types::keycloak::User;
use serde_json::{json, Map, Value};
use std::collections::HashSet;

/// The `user` object that email, SMS and per-voter report templates render:
/// the standard fields, each attribute's first value as `user.<attribute>`
/// and every value list under `user.attributes`. Standard fields win over
/// attributes with the same name. The caller must have removed the secret
/// attributes the template may not read.
pub fn user_template_variables(user: &User) -> Value {
    let mut variables = Map::new();
    variables.insert("first_name".to_string(), json!(user.first_name));
    variables.insert("last_name".to_string(), json!(user.last_name));
    variables.insert("username".to_string(), json!(user.username));
    variables.insert("email".to_string(), json!(user.email));

    let attributes = user.attributes.clone().unwrap_or_default();
    for (name, values) in &attributes {
        if let Some(value) = values.first() {
            variables
                .entry(name.clone())
                .or_insert_with(|| json!(value));
        }
    }
    variables.insert("attributes".to_string(), json!(attributes));
    Value::Object(variables)
}

/// [`user_template_variables`] for a voter loaded from Keycloak: the
/// configured secret attributes that the template does not declare are
/// dropped first. Declared secrets must already be decrypted.
pub fn voter_template_variables(
    user: &User,
    configured_secret_names: &HashSet<String>,
    declared_secret_names: &HashSet<String>,
) -> Value {
    let mut user = user.clone();
    strip_undeclared_secret_attributes(&mut user, configured_secret_names, declared_secret_names);
    user_template_variables(&user)
}

#[cfg(test)]
mod tests {
    use super::{user_template_variables, voter_template_variables};
    use sequent_core::types::keycloak::User;
    use serde_json::json;
    use std::collections::{HashMap, HashSet};

    const CIPHERTEXT: &str = "seqenc:v1:c2VjcmV0LWNpcGhlcnRleHQ";

    fn voter() -> User {
        User {
            first_name: Some("Jane".to_string()),
            last_name: Some("Doe".to_string()),
            username: Some("jane.doe".to_string()),
            email: Some("jane@example.com".to_string()),
            attributes: Some(HashMap::from([
                ("ward".to_string(), vec!["Ward 2".to_string()]),
                (
                    "school_support_choice".to_string(),
                    vec!["English Public".to_string()],
                ),
                ("corr_unit".to_string(), Vec::new()),
                ("national_id".to_string(), vec![CIPHERTEXT.to_string()]),
            ])),
            ..User::default()
        }
    }

    fn names(names: &[&str]) -> HashSet<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn non_secret_attributes_are_available_without_declaring_them() {
        let variables = voter_template_variables(&voter(), &names(&["national_id"]), &names(&[]));

        assert_eq!(variables["first_name"], json!("Jane"));
        assert_eq!(variables["last_name"], json!("Doe"));
        assert_eq!(variables["username"], json!("jane.doe"));
        assert_eq!(variables["email"], json!("jane@example.com"));
        assert_eq!(variables["ward"], json!("Ward 2"));
        assert_eq!(variables["school_support_choice"], json!("English Public"));
        assert_eq!(variables["attributes"]["ward"], json!(["Ward 2"]));
        assert!(variables.get("corr_unit").is_none());
        assert_eq!(variables["attributes"]["corr_unit"], json!([]));
    }

    #[test]
    fn undeclared_secret_attributes_are_absent() {
        let variables = voter_template_variables(&voter(), &names(&["national_id"]), &names(&[]));

        assert!(variables.is_object());
        assert!(variables.get("national_id").is_none());
        assert!(variables["attributes"].get("national_id").is_none());
        assert!(!variables.to_string().contains(CIPHERTEXT));
    }

    #[test]
    fn declared_secret_attributes_are_present() {
        let mut user = voter();
        user.attributes
            .as_mut()
            .expect("voter has attributes")
            .insert("national_id".to_string(), vec!["X1234567".to_string()]);

        let variables =
            voter_template_variables(&user, &names(&["national_id"]), &names(&["national_id"]));

        assert_eq!(variables["national_id"], json!("X1234567"));
        assert_eq!(variables["attributes"]["national_id"], json!(["X1234567"]));
    }

    #[test]
    fn standard_fields_win_over_attributes_with_the_same_name() {
        let mut user = voter();
        user.attributes
            .as_mut()
            .expect("voter has attributes")
            .extend([
                (
                    "username".to_string(),
                    vec!["attribute-username".to_string()],
                ),
                ("first_name".to_string(), vec!["Attribute".to_string()]),
            ]);

        let variables = user_template_variables(&user);

        assert_eq!(variables["username"], json!("jane.doe"));
        assert_eq!(variables["first_name"], json!("Jane"));
        assert_eq!(
            variables["attributes"]["username"],
            json!(["attribute-username"])
        );
    }

    #[test]
    fn missing_standard_fields_are_null() {
        let variables = user_template_variables(&User::default());

        assert_eq!(variables["first_name"], json!(null));
        assert_eq!(variables["attributes"], json!({}));
    }
}
