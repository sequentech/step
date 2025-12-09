// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const IAUDIENCE_SELECTION: &'static str = r#"
enum IAudienceSelection {
    ALL_USERS = "ALL_USERS",
    NOT_VOTED = "NOT_VOTED",
    VOTED = "VOTED",
    SELECTED = "SELECTED"
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IAudienceSelection")]
    pub type IAudienceSelection;
}

#[wasm_bindgen(typescript_custom_section)]
const Itype: &'static str = r#"
enum ITemplateType {
    CREDENTIALS = "CREDENTIALS",
    BALLOT_RECEIPT = "BALLOT_RECEIPT",
    PARTICIPATION_REPORT = "PARTICIPATION_REPORT",
    ELECTORAL_RESULTS = "ELECTORAL_RESULTS",
    OTP = "OTP",
    MANUALLY_VERIFY_VOTER = "MANUALLY_VERIFY_VOTER",
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ITemplateType")]
    pub type ITemplateType;
}

#[wasm_bindgen(typescript_custom_section)]
const ITemplate_METHOD: &'static str = r#"
enum ITemplateMethod {
    EMAIL = "EMAIL",
    SMS = "SMS"
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ITemplateMethod")]
    pub type ITemplateMethod;
}

#[wasm_bindgen(typescript_custom_section)]
const IEMAIL_CONFIG: &'static str = r#"
interface IEmailConfig {
    subject: string;
    plaintext_body: string;
    html_body: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IEmailConfig")]
    pub type IEmailConfig;
}

#[wasm_bindgen(typescript_custom_section)]
const ISMS_CONFIG: &'static str = r#"
interface ISmsConfig {
    message: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ISmsConfig")]
    pub type ISmsConfig;
}

#[wasm_bindgen(typescript_custom_section)]
const ICOMM_TEMPLATES: &'static str = r#"
interface ICommTemplates {
    email: IEmail;
    sms: ISmsConfig;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ICommTemplates")]
    pub type ICommTemplates;
}

#[wasm_bindgen(typescript_custom_section)]
const IEXTRA_CONFIG: &'static str = r#"
interface IExtraConfig {
    pdf_options: string;
    report_options: string;
    communication_templates: ICommTemplates;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IExtraConfig")]
    pub type IExtraConfig;
}

#[wasm_bindgen(typescript_custom_section)]
const ISEND_Template_BODY: &'static str = r#"
interface ISendTemplateBody {
    audience_selection: IAudienceSelection;
    audience_voter_ids?: Array<string>;
    type: ITemplateType;
    communication_method: ITemplateMethod;
    schedule_now: boolean;
    schedule_date?: string;
    name?: string;
    alias?: string;
    document?: string;
    extra_config?: IExtraConfig;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ISendTemplateBody")]
    pub type ISendTemplateBody;
}
