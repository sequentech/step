// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::ballot_tracker_url::{build_ballot_tracker_url, BallotTrackerPath};
use super::template_renderer::*;
use super::template_time::event_presentation;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::vitrine::get_vitrine_event;
use crate::services::ballot_box_reads::get_event_ballot_box;
use crate::services::receive_ballot::tracking_id_of_content;
use crate::services::temp_path::*;
use crate::services::vitrine::hub_sites::{hubs_for_receipt, HubLink};

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use electoral_log::adapters::ballot_box_reads::{BallotIdMatch, StoredBallot};
use sequent_core::ballot_receipt::{normalize_tracking_id, tracking_id_starts_with};
use sequent_core::util::temp_path::*;

use sequent_core::services::pdf;
use sequent_core::services::s3::get_minio_url;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::templates::EmailConfig;
use sequent_core::util::date_time::get_date_and_time;
use serde::{Deserialize, Serialize};
use tracing::{instrument, warn};
use uuid::Uuid;

/// Wrapper struct for data specific to the ballot and the voter
/// which won't be needed in the preview mode.
#[derive(Serialize, Deserialize, Debug)]
pub struct BallotData {
    pub area_id: String,
    pub voter_id: String,
    pub ballot_id: String,
    pub ballot_tracker_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub ballot_id: String,
    /// The ballot's full Tracking ID, in its 13 groups of four. Empty when the
    /// stored ballot can't be read (and in older preview data), and the
    /// template prints `ballot_id` instead.
    #[serde(default)]
    pub tracking_id: String,
    pub ballot_tracker_url: String,
    /// The hubs of the event's policy that have a site, as links that watch
    /// this ballot, in a random order. Empty unless the event is transparent
    /// and its `hub_updates.cast_email` is not `off`.
    #[serde(default)]
    pub hubs: Vec<HubLink>,
    pub qrcode: String,
    pub logo: String,
    pub timestamp: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub title: String,
    pub file_logo: String,
    pub file_qrcode_lib: String,
}

/// The text the cast email adds below the template's own: the ballot's full
/// Tracking ID and a link to each of the policy's hubs, in the order the
/// receipt drew. Plain text, one link to a line, with no labels that rank a
/// hub; the PDF says the same.
pub fn cast_email_text(tracking_id: &str, hubs: &[HubLink]) -> String {
    let mut text = format!(
        "Your Tracking ID: {tracking_id}\n\n\
         Check it yourself. You don't have to trust this email or this \
         election's website. You can check your ballot on any of the \
         election's hubs:\n"
    );
    for hub in hubs {
        text.push_str(&format!(
            "\n{}, run by {}\n{}\n",
            hub.name, hub.operator, hub.url
        ));
    }
    text.push_str(
        "\nOn a hub you can also choose to get an email when your ballot is \
         confirmed and when it is counted. The links carry your Tracking ID \
         after the #, so no server sees it until you act. This email is a \
         pointer, not proof: a hub's page checks the record again in your \
         browser.\n",
    );
    text
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The same, as HTML, for an email that has an HTML body.
fn cast_email_html(tracking_id: &str, hubs: &[HubLink]) -> String {
    let mut html = format!(
        "<p>Your Tracking ID: <strong>{}</strong></p>\n\
         <p>Check it yourself. You don't have to trust this email or this \
         election's website. You can check your ballot on any of the \
         election's hubs:</p>\n",
        escape_html(tracking_id)
    );
    for hub in hubs {
        html.push_str(&format!(
            "<p><a href=\"{}\">{}, run by {}</a></p>\n",
            escape_html(&hub.url),
            escape_html(&hub.name),
            escape_html(&hub.operator)
        ));
    }
    html.push_str(
        "<p>On a hub you can also choose to get an email when your ballot is \
         confirmed and when it is counted. The links carry your Tracking ID \
         after the #, so no server sees it until you act. This email is a \
         pointer, not proof: a hub's page checks the record again in your \
         browser.</p>\n",
    );
    html
}

/// The cast email's bodies with the Tracking ID and the hubs' links added.
/// With no hubs (the event isn't transparent, or `hub_updates.cast_email` is
/// `off`, or no hub has a site), the email is the template's, untouched.
pub fn with_cast_email_hubs(mut email: EmailConfig, data: &UserData) -> EmailConfig {
    if data.hubs.is_empty() {
        return email;
    }
    let tracking_id = if data.tracking_id.is_empty() {
        data.ballot_id.as_str()
    } else {
        data.tracking_id.as_str()
    };
    let text = cast_email_text(tracking_id, &data.hubs);
    email.plaintext_body = if email.plaintext_body.trim().is_empty() {
        text
    } else {
        format!("{}\n\n{text}", email.plaintext_body.trim_end())
    };
    if let Some(html) = email.html_body.take() {
        let section = cast_email_html(tracking_id, &data.hubs);
        email.html_body = Some(match html.rfind("</body>") {
            Some(at) => format!("{}{section}{}", &html[..at], &html[at..]),
            None => format!("{html}\n{section}"),
        });
    }
    email
}

#[derive(Debug)]
pub struct BallotTemplate {
    ids: ReportOrigins,
    pub ballot_data: Option<BallotData>,
}

impl BallotTemplate {
    pub fn new(ids: ReportOrigins, ballot_data: Option<BallotData>) -> Self {
        BallotTemplate { ids, ballot_data }
    }
}

#[async_trait]
impl TemplateRenderer for BallotTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type(&self) -> ReportType {
        ReportType::BALLOT_RECEIPT
    }

    fn base_name(&self) -> String {
        "ballot_receipt".to_string()
    }

    fn prefix(&self) -> String {
        format!("ballot_receipt_{}", self.ids.election_event_id,)
    }

    fn get_tenant_id(&self) -> String {
        self.ids.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.ids.election_event_id.clone()
    }

    fn get_initial_template_alias(&self) -> Option<String> {
        self.ids.template_alias.clone()
    }

    fn get_report_origin(&self) -> ReportOriginatedFrom {
        self.ids.report_origin
    }

    fn get_election_id(&self) -> Option<String> {
        self.ids.election_id.clone()
    }

    #[instrument(err, skip_all)]
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        _keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        let Some(election_id) = &self.ids.election_id else {
            return Err(anyhow!("Empty election_id"));
        };

        let (area_id, voter_id, ballot_id, ballot_tracker_url) = match &self.ballot_data {
            Some(ballot_data) => (
                ballot_data.area_id.as_str(),
                ballot_data.voter_id.as_str(),
                ballot_data.ballot_id.as_str(),
                ballot_data.ballot_tracker_url.as_str(),
            ),
            None => {
                return Err(anyhow!(
                    "Cannot verify ballot id becasue Ballot data is missing"
                ));
            }
        };

        let tennant_uuid = parse_uuid_v4(self.get_tenant_id().as_str())
            .map_err(|err| anyhow!("Error parsing tenant id: {:?}", err))?;
        let election_event_uuid = parse_uuid_v4(self.get_election_event_id().as_str())
            .map_err(|err| anyhow!("Error parsing election event id: {:?}", err))?;

        let ballot_uui = parse_uuid_v4(election_id.as_str())
            .map_err(|err| anyhow!("Error parsing election id: {:?}", err))?;

        let store = get_event_ballot_box(
            hasura_transaction,
            &tennant_uuid.to_string(),
            &election_event_uuid.to_string(),
        )
        .await?
        .store;
        let (event, election) = (election_event_uuid.to_string(), ballot_uui.to_string());
        // The authenticated receipt route resolves privacy identities before
        // queueing this task, so its payload never carries the real voter ID.
        let in_box =
            |ballot: &StoredBallot| ballot.area_id == area_id && ballot.election_id == election;
        // A ballot goes by its Tracking ID, or by its hash.
        let found = match normalize_tracking_id(ballot_id) {
            Some(typed) => store
                .voter_ballot_contents(&event, voter_id, &election, BallotIdMatch::Any)
                .await?
                .into_iter()
                .find(|found| {
                    in_box(&found.ballot)
                        && tracking_id_of_content(&found.ballot.ballot_id, &found.content)
                            .is_some_and(|tracking_id| {
                                tracking_id_starts_with(&tracking_id, &typed)
                            })
                }),
            None => store
                .voter_ballot_contents(&event, voter_id, &election, BallotIdMatch::Exact(ballot_id))
                .await?
                .into_iter()
                .find(|found| in_box(&found.ballot)),
        };

        // Verify that the vote has been casted
        let Some(found) = found else {
            return Err(anyhow!(
                "The ballot was not found in this voter's cast votes"
            ));
        };
        let tracking_id = tracking_id_of_content(&found.ballot.ballot_id, &found.content);

        let election_event = get_election_event_by_id(
            hasura_transaction,
            &tennant_uuid.to_string(),
            &election_event_uuid.to_string(),
        )
        .await
        .context("Error loading the election event for the receipt's hub updates")?;
        let vitrine = get_vitrine_event(
            hasura_transaction,
            &tennant_uuid.to_string(),
            &election_event_uuid.to_string(),
        )
        .await?;
        let hubs = hubs_for_receipt(
            &event_presentation(election_event.presentation.as_ref()),
            vitrine.as_ref(),
            tracking_id.as_deref(),
            &mut rand::rng(),
        );

        let kiosk_base = match std::env::var("KIOSK_VOTING_PORTAL_URL") {
            Err(std::env::VarError::NotPresent) => None,
            kiosk_base => Some(kiosk_base.context("Invalid KIOSK_VOTING_PORTAL_URL env var")?),
        };

        Ok(UserData {
            ballot_id: ballot_id.to_string(),
            tracking_id: tracking_id.unwrap_or_default(),
            hubs,
            ballot_tracker_url: build_ballot_tracker_url(
                &std::env::var("VOTING_PORTAL_URL").context("VOTING_PORTAL_URL env var missing")?,
                kiosk_base.as_deref(),
                ballot_tracker_url,
                BallotTrackerPath {
                    tenant_id: &self.ids.tenant_id,
                    election_event_id: &self.ids.election_event_id,
                    election_id,
                    ballot_id,
                },
            )?,
            qrcode: QR_CODE_TEMPLATE.to_string(),
            logo: LOGO_TEMPLATE.to_string(),
            // A UTC instant: the template prints it in the election's zone.
            timestamp: get_date_and_time(),
        })
    }

    /// The cast email carries the ballot's full Tracking ID and a link to
    /// each hub of the policy when `hub_updates.cast_email` allows it. If
    /// the ballot can't be read again, the email goes as the template has it.
    async fn email_config_for(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
        email_config: EmailConfig,
    ) -> Result<EmailConfig> {
        match self
            .prepare_user_data(hasura_transaction, keycloak_transaction)
            .await
        {
            Ok(data) => Ok(with_cast_email_hubs(email_config, &data)),
            Err(error) => {
                warn!("The cast email goes without the hubs' links: {error:?}");
                Ok(email_config)
            }
        }
    }

    #[instrument(err, skip_all)]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        let public_assets_path = get_public_assets_path_env_var()?;
        let minio_endpoint_base =
            get_minio_url().with_context(|| "Error getting minio endpoint")?;

        if pdf::doc_renderer_backend() == pdf::DocRendererBackend::InPlace {
            Ok(SystemData {
                rendered_user_template,
                file_logo: format!(
                    "{}/{}/{}",
                    minio_endpoint_base, public_assets_path, PUBLIC_ASSETS_LOGO_IMG
                ),
                file_qrcode_lib: format!(
                    "{}/{}/{}",
                    minio_endpoint_base, public_assets_path, PUBLIC_ASSETS_QRCODE_LIB
                ),
                title: "Ballot receipt - Sequentech".to_string(),
            })
        } else {
            // If we are rendering with a lambda, the QRCode lib is
            // already included in the lambda container image.
            Ok(SystemData {
                rendered_user_template,
                file_logo: format!(
                    "{}/{}/{}",
                    minio_endpoint_base, public_assets_path, PUBLIC_ASSETS_LOGO_IMG
                ),
                file_qrcode_lib: "/assets/qrcode.min.js".to_string(),
                title: "Ballot receipt - Sequentech".to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::services::reports::{render_template_text, template_time_variables};
    use serde_json::{json, Value};

    const TEMPLATE: &str =
        include_str!("../../../../../.devcontainer/minio/public-assets/ballot_receipt_user.hbs");
    const PREVIEW: &str =
        include_str!("../../../../../.devcontainer/minio/public-assets/ballot_receipt.json");

    const TRACKING_ID: &str = "FTBE-MHRX-71J7-5ZBD-QPTD-YDMN-B47Q-MGAD-0ANJ-96AV-XZ2T-JG5R-KPGG";

    fn hub(name: &str, operator: &str) -> HubLink {
        HubLink {
            name: name.to_string(),
            operator: operator.to_string(),
            url: format!(
                "https://{name}.example/#policy=sha256%3A{}&watch-id={TRACKING_ID}",
                "ab".repeat(32)
            ),
        }
    }

    fn user_data(tracking_id: &str, hubs: Vec<HubLink>) -> UserData {
        UserData {
            ballot_id: "45a83c5c252ecfacbc363380d5c69fd589e90ed49770c9956897a4c8988f39a1"
                .to_string(),
            tracking_id: tracking_id.to_string(),
            hubs,
            ballot_tracker_url: "https://voting.example/ballot-locator/x".to_string(),
            qrcode: "<div id=\"qrcode\"></div>".to_string(),
            logo: "<div class=\"logo\"></div>".to_string(),
            timestamp: "2026-01-15T12:00:00Z".to_string(),
        }
    }

    /// The default template rendered with the data the task gives it.
    fn render(data: &UserData) -> String {
        let Value::Object(mut variables) = serde_json::to_value(data).unwrap() else {
            unreachable!("user data is an object");
        };
        variables.extend(template_time_variables(None, None, &json!({})));
        render_template_text(TEMPLATE, variables).unwrap()
    }

    /// What a browser reads from an attribute of the rendered HTML.
    fn unescaped(html: &str) -> String {
        html.replace("&#x3D;", "=").replace("&amp;", "&")
    }

    #[test]
    fn the_receipt_prints_the_full_tracking_id() {
        let html = render(&user_data(TRACKING_ID, vec![]));
        assert!(html.contains(TRACKING_ID), "{html}");
        assert!(
            !html.contains("45a83c5c252e"),
            "the hash isn't printed beside it"
        );
    }

    #[test]
    fn a_receipt_without_a_full_tracking_id_prints_the_ballot_id_it_was_asked_for() {
        let html = render(&user_data("", vec![]));
        assert!(html.contains("45a83c5c252ecfacbc363380d5c69fd589e90ed49770c9956897a4c8988f39a1"));
    }

    #[test]
    fn a_receipt_without_hubs_has_no_hub_section() {
        let html = render(&user_data(TRACKING_ID, vec![]));
        assert!(!html.contains("Check it yourself"), "{html}");
        assert!(!html.contains("run by"));
    }

    #[test]
    fn the_receipt_links_to_every_hub_in_the_order_given() {
        let hubs = vec![
            hub("hub-b", "Partner B"),
            hub("hub-a", "Partner A"),
            hub("sequent", "Sequent"),
        ];
        let html = render(&user_data(TRACKING_ID, hubs.clone()));
        let page = unescaped(&html);

        let mut at = 0;
        for hub in &hubs {
            let link = format!(
                "<a class=\"value\" href=\"{}\">{}, run by {}</a>",
                hub.url, hub.name, hub.operator
            );
            let found = page[at..]
                .find(&link)
                .unwrap_or_else(|| panic!("{link} in order\n{page}"));
            at += found + link.len();
        }
        assert_eq!(page.matches("<p>or</p>").count(), 2);
        assert!(page.contains("Check it yourself"));
    }

    #[test]
    fn the_receipt_escapes_what_a_hub_is_called() {
        let html = render(&user_data(
            TRACKING_ID,
            vec![hub("<script>x</script>", "A & B")],
        ));
        assert!(!html.contains("<script>x</script>"));
        assert!(html.contains("A &amp; B"));
    }

    #[test]
    fn the_receipt_never_calls_a_hub_independent() {
        let html = render(&user_data(TRACKING_ID, vec![hub("hub-a", "Partner A")]));
        assert!(!html.to_lowercase().contains("independent"));
    }

    #[test]
    fn the_tracker_script_stays_the_last_script_of_the_template() {
        let at = TEMPLATE.rfind("<script>").unwrap();
        assert!(!TEMPLATE[at..].contains("{{#each hubs}}"));
        assert_eq!(TEMPLATE.matches("<script>").count(), 1);
    }

    #[test]
    fn user_data_from_before_the_hub_links_still_reads() {
        let old = json!({
            "ballot_id": "abc",
            "ballot_tracker_url": "https://voting.example/x",
            "qrcode": "q",
            "logo": "l",
            "timestamp": "2026-01-15T12:00:00Z",
        });
        let data: UserData = serde_json::from_value(old).unwrap();
        assert_eq!(data.tracking_id, "");
        assert!(data.hubs.is_empty());
    }

    #[test]
    fn the_preview_data_reads_and_shows_the_hubs() {
        let data: UserData = serde_json::from_str(PREVIEW).unwrap();
        assert_eq!(data.hubs.len(), 2);
        assert_eq!(data.tracking_id.split('-').count(), 13);
        let page = unescaped(&render(&data));
        for hub in &data.hubs {
            assert!(
                page.contains(&format!("href=\"{}\"", hub.url)),
                "{}",
                hub.url
            );
        }
    }

    fn email(plaintext: &str, html: Option<&str>) -> EmailConfig {
        EmailConfig {
            subject: "Ballot Receipt".to_string(),
            plaintext_body: plaintext.to_string(),
            html_body: html.map(str::to_string),
        }
    }

    #[test]
    fn the_cast_email_gets_the_tracking_id_and_every_hubs_link() {
        let hubs = vec![hub("hub-b", "Partner B"), hub("hub-a", "Partner A")];
        let data = user_data(TRACKING_ID, hubs.clone());
        let sent = with_cast_email_hubs(email("Thank you for voting.", None), &data);
        assert_eq!(sent.subject, "Ballot Receipt");
        assert!(sent.plaintext_body.starts_with("Thank you for voting.\n\n"));
        assert!(sent
            .plaintext_body
            .contains(&format!("Your Tracking ID: {TRACKING_ID}\n")));
        for h in &hubs {
            assert!(
                sent.plaintext_body
                    .contains(&format!("{}, run by {}\n{}\n", h.name, h.operator, h.url)),
                "{}",
                sent.plaintext_body
            );
        }
        // The links come in the order the receipt drew, one to a line.
        let b = sent.plaintext_body.find("hub-b, run by").unwrap();
        let a = sent.plaintext_body.find("hub-a, run by").unwrap();
        assert!(b < a);
        assert_eq!(sent.html_body, None);
    }

    #[test]
    fn the_cast_email_is_plain_text_with_no_badges_or_claims_of_independence() {
        let data = user_data(TRACKING_ID, vec![hub("hub-a", "Partner A")]);
        let sent = with_cast_email_hubs(
            email("", Some("<html><body><p>Hi</p></body></html>")),
            &data,
        );
        let text = sent.plaintext_body.to_lowercase();
        assert!(!text.contains("independent"));
        assert!(!sent.plaintext_body.contains('<'));
        assert!(sent.plaintext_body.starts_with("Your Tracking ID:"));
        assert!(text.contains("pointer, not proof"));
        let html = sent.html_body.unwrap();
        assert!(!html.to_lowercase().contains("independent"));
        // The hubs' section goes inside the body of the HTML.
        assert!(html.find("Partner A").unwrap() < html.find("</body>").unwrap());
        assert!(html.contains(&TRACKING_ID.to_string()));
        assert!(html.contains("&amp;watch-id="), "{html}");
    }

    #[test]
    fn the_cast_email_is_untouched_when_there_are_no_hubs() {
        let data = user_data(TRACKING_ID, vec![]);
        let template = email("Thank you for voting.", Some("<p>Thanks</p>"));
        let sent = with_cast_email_hubs(template.clone(), &data);
        assert_eq!(sent.plaintext_body, template.plaintext_body);
        assert_eq!(sent.html_body, template.html_body);
    }

    #[test]
    fn the_cast_email_falls_back_to_the_ballot_id_without_a_tracking_id() {
        let data = user_data("", vec![hub("hub-a", "Partner A")]);
        let sent = with_cast_email_hubs(email("", None), &data);
        assert!(sent.plaintext_body.contains(&data.ballot_id));
    }

    #[test]
    fn html_in_a_hubs_name_is_escaped_in_the_email() {
        let mut odd = hub("hub-a", "Partner <b>A</b>");
        odd.url = "https://hub-a.example/#policy=x&watch-id=y\"z".to_string();
        let data = user_data(TRACKING_ID, vec![odd]);
        let sent = with_cast_email_hubs(email("", Some("<p>x</p>")), &data);
        let html = sent.html_body.unwrap();
        assert!(!html.contains("<b>"), "{html}");
        assert!(html.contains("Partner &lt;b&gt;A&lt;/b&gt;"));
        assert!(html.contains("&quot;z"));
    }
}
