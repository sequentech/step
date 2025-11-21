// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const IUserArea: &'static str = r#"
interface IUserArea {
    id?: string;
    name?: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IUserArea")]
    pub type IUserArea;
}

#[wasm_bindgen(typescript_custom_section)]
const IVotesInfo: &'static str = r#"
interface IVotesInfo {
    election_id: string;
    num_votes: Int;
    last_voted_at: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IVotesInfo")]
    pub type IVotesInfo;
}

#[wasm_bindgen(typescript_custom_section)]
const IUSER: &'static str = r#"
interface IUser {
    id?: string;
    attributes?: {[key: string]: any};
    email?: string;
    email_verified?: boolean;
    enabled?: boolean;
    first_name?: string;
    last_name?: string;
    username?: string;
    password?: string;
    area?: IUserArea;
    votes_info?: Array<IVotesInfo>;
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
    attributes: {[key: string]: any};
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
interface IRole {
    id?: string;
    name?: string;
    permissions?: Array<string>;
    access?: {[key: string]: any};
    attributes?: {[key: string]: any};
    client_roles?: {[key: string]: any};
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IRole")]
    pub type IRole;
}
