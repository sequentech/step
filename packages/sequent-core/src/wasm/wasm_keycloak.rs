// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const IUSER: &'static str = r#"
interface IUser {
    id?: string;
    attributes?: {[key: string]: object};
    email?: string;
    email_verified?: boolean;
    enabled?: boolean;
    first_name?: string;
    groups?: Array<string>;
    last_name?: string;
    username?: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IUser")]
    pub type IUser;
}

#[wasm_bindgen(typescript_custom_section)]
const IPERMISSION: &'static str = r#"
interface IPermission {
    id?: string;
    attributes: {[key: string]: object};
    container_id?: string;
    description?: string;
    name?: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IPermission")]
    pub type IPermission;
}

#[wasm_bindgen(typescript_custom_section)]
const IROLE: &'static str = r#"
pub struct IRole {
    id?: string;
    name?: string;
    permissions?: Array<string>;
    access?: {[key: string]: object};
    attributes?: {[key: string]: object};
    client_roles?: {[key: string]: object};
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IRole")]
    pub type IRole;
}
