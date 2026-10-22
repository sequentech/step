// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Trustees reach the board service (b4) through Harvest: b4 sits on the
//! private network, and only a board's own trustees may write to it. The paths
//! mirror b4's, so a trustee's board URL is `<origin>/b4`. Requests and
//! answers pass through untouched: Harvest reads no protocol message, and the
//! caller's `Authorization` header never reaches b4. Creating boards stays
//! with Windmill, on the private network.

use crate::services::authorization::authorize;
use crate::services::dependencies::HarvestServices;
use deadpool_postgres::Client as DbClient;
use protocol_board::BoardName;
use rocket::data::{Data, ToByteUnit};
use rocket::http::uri::Origin;
use rocket::http::{ContentType, Status};
use rocket::response::{self, Responder, Response};
use rocket::{Request, State};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use std::io::Cursor;
use std::time::Duration;
use tracing::{instrument, warn};
use uuid::Uuid;
use windmill::services::trustee_boards::{self, BoardAccess};

/// The board service's address, shared with Windmill.
pub const URL_VARIABLE: &str = "WBRAID_B4_URL";
/// b4 is on the private network: a connection that takes longer is an outage.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// The longest b4 may take to answer; listing a large board presigns one
/// download URL per message.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
/// The largest body b4 reads: axum's default `Json` limit, 2 MiB.
const BODY_LIMIT_BYTES: u64 = 2 * 1024 * 1024;

/// What a forwarded request gets when b4 can't be reached. The reason is
/// logged, never answered: it names b4's private address.
const UNREACHABLE: &str = "the board service is unreachable";
/// Which other origins' pages may read boards (`*`, or a comma-separated
/// list); unset, only the admin portal's own.
pub const READ_ORIGINS_VARIABLE: &str = "HARVEST_B4_READ_ORIGINS";
const ALLOW_ORIGIN: &str = "Access-Control-Allow-Origin";
const ANY_ORIGIN: &str = "*";

/// Which pages may read the public board answers. The admin portal reads
/// `/b4` from its own origin and needs no CORS; verifiers on other origins
/// are the exception a deployment opts into.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadOrigins {
    /// Only the admin portal's origin, and clients that are not browsers.
    SameOrigin,
    /// Any page, as b4's own answers allowed.
    AnyOrigin,
    /// Pages on these origins (`scheme://host[:port]`).
    Listed(Vec<String>),
}

impl ReadOrigins {
    /// The policy `value` of [`READ_ORIGINS_VARIABLE`] names. Entries that
    /// are not an http(s) origin are logged and left out.
    pub fn parse(value: Option<&str>) -> Self {
        let value = value.unwrap_or_default().trim();
        if value.is_empty() {
            return Self::SameOrigin;
        }
        if value == ANY_ORIGIN {
            return Self::AnyOrigin;
        }
        let origins: Vec<String> = value
            .split(',')
            .map(|entry| entry.trim().trim_end_matches('/'))
            .filter(|entry| !entry.is_empty())
            .filter(|entry| {
                let origin = entry
                    .strip_prefix("https://")
                    .or_else(|| entry.strip_prefix("http://"))
                    .is_some_and(|host| {
                        !host.is_empty() && !host.contains('/')
                    });
                if !origin {
                    warn!(
                        "{READ_ORIGINS_VARIABLE}: {entry:?} is not an origin"
                    );
                }
                origin
            })
            .map(str::to_string)
            .collect();
        if origins.is_empty() {
            Self::SameOrigin
        } else {
            Self::Listed(origins)
        }
    }
}

/// The board service trustees' board requests are forwarded to
/// (`WBRAID_B4_URL`). Read once, into [`HarvestServices`].
#[derive(Clone)]
pub struct BoardService {
    connection: Connection,
    read_origins: ReadOrigins,
}

/// Whether Harvest can forward requests to b4.
#[derive(Clone)]
enum Connection {
    /// `WBRAID_B4_URL` is unset or blank.
    NotConfigured,
    /// The HTTP client could not be built, for this reason.
    ClientUnavailable(String),
    Ready {
        /// Without a trailing slash.
        base_url: String,
        client: reqwest::Client,
    },
}

impl BoardService {
    pub fn from_env() -> Self {
        Self::new(std::env::var(URL_VARIABLE).ok().as_deref())
            .with_read_origins(ReadOrigins::parse(
                std::env::var(READ_ORIGINS_VARIABLE).ok().as_deref(),
            ))
    }

    /// Which pages may read boards; [`ReadOrigins::SameOrigin`] unless set.
    pub fn with_read_origins(mut self, read_origins: ReadOrigins) -> Self {
        self.read_origins = read_origins;
        self
    }

    /// b4 at `base_url`; none or blank: not configured. Until it is, and
    /// if its client can't be built, every forwarded request is answered
    /// 502.
    pub fn new(base_url: Option<&str>) -> Self {
        let Some(base_url) = base_url
            .map(|url| url.trim().trim_end_matches('/').to_string())
            .filter(|url| !url.is_empty())
        else {
            warn!("{URL_VARIABLE} is not set: board requests are refused");
            return Self {
                connection: Connection::NotConfigured,
                read_origins: ReadOrigins::SameOrigin,
            };
        };
        // An internal service: never through a proxy, and its redirects (b4
        // sends none) are passed back rather than followed.
        let connection = match reqwest::Client::builder()
            .no_proxy()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .build()
        {
            Ok(client) => Connection::Ready { base_url, client },
            Err(error) => {
                tracing::error!(
                    "the board service client can't be built: {error}"
                );
                Connection::ClientUnavailable(error.to_string())
            }
        };
        Self {
            connection,
            read_origins: ReadOrigins::SameOrigin,
        }
    }

    /// A request to b4's `path`, which starts with a slash, or 502.
    fn request(
        &self,
        method: reqwest::Method,
        path: &str,
    ) -> Result<reqwest::RequestBuilder, (Status, String)> {
        match &self.connection {
            Connection::Ready { base_url, client } => {
                Ok(client.request(method, format!("{base_url}{path}")))
            }
            Connection::NotConfigured => {
                warn!("{URL_VARIABLE} is not set: {path} is not forwarded");
                Err((Status::BadGateway, UNREACHABLE.to_string()))
            }
            Connection::ClientUnavailable(reason) => {
                warn!("the board service client can't be built: {reason}");
                Err((Status::BadGateway, UNREACHABLE.to_string()))
            }
        }
    }

    /// b4's answer to `request`, or 502 when b4 can't be reached.
    async fn forward(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<BoardReply, (Status, String)> {
        let response = request.send().await.map_err(b4_unreachable)?;
        let status = Status::new(response.status().as_u16());
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<ContentType>().ok());
        let body = response.bytes().await.map_err(b4_unreachable)?.to_vec();
        Ok(BoardReply {
            status,
            content_type,
            body,
        })
    }

    /// GET b4's `path`, with the caller's raw `query`.
    async fn get(
        &self,
        path: &str,
        query: Option<&str>,
    ) -> Result<BoardReply, (Status, String)> {
        let path = match query {
            Some(query) => format!("{path}?{query}"),
            None => path.to_string(),
        };
        let request = self.request(reqwest::Method::GET, &path)?;
        self.forward(request).await
    }

    /// POST the JSON `body` to b4's `path`.
    async fn post(
        &self,
        path: &str,
        body: Vec<u8>,
    ) -> Result<BoardReply, (Status, String)> {
        let request = self
            .request(reqwest::Method::POST, path)?
            .header(
                reqwest::header::CONTENT_TYPE,
                ContentType::JSON.to_string(),
            )
            .body(body);
        self.forward(request).await
    }
}

fn b4_unreachable(error: reqwest::Error) -> (Status, String) {
    warn!("{UNREACHABLE}: {error}");
    (Status::BadGateway, UNREACHABLE.to_string())
}

/// A public board answer, readable by the pages [`ReadOrigins`] allows. A
/// simple GET needs no preflight.
pub struct PublicRead<R>(ReadOrigins, R);

impl<'r, 'o: 'r, R: Responder<'r, 'o>> Responder<'r, 'o> for PublicRead<R> {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'o> {
        let PublicRead(read_origins, answer) = self;
        let mut response = answer.respond_to(request)?;
        match read_origins {
            ReadOrigins::SameOrigin => {}
            ReadOrigins::AnyOrigin => {
                response.set_raw_header(ALLOW_ORIGIN, ANY_ORIGIN);
            }
            ReadOrigins::Listed(origins) => {
                // The answer differs by origin, so caches must keep them apart.
                response.set_raw_header("Vary", "Origin");
                if let Some(origin) =
                    request.headers().get_one("Origin").filter(|origin| {
                        origins
                            .iter()
                            .any(|listed| listed.eq_ignore_ascii_case(origin))
                    })
                {
                    response.set_raw_header(ALLOW_ORIGIN, origin.to_string());
                }
            }
        }
        Ok(response)
    }
}

/// b4's answer, passed back as it came: its status, content type and body.
pub struct BoardReply {
    status: Status,
    content_type: Option<ContentType>,
    body: Vec<u8>,
}

impl<'r> Responder<'r, 'static> for BoardReply {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        let mut builder = Response::build();
        builder
            .status(self.status)
            .sized_body(self.body.len(), Cursor::new(self.body));
        if let Some(content_type) = self.content_type {
            builder.header(content_type);
        }
        builder.ok()
    }
}

/// A board name the platform mints, or 404: anything else is no board of
/// ours, and never reaches b4's paths.
fn board_name(board: &str) -> Result<BoardName, (Status, String)> {
    BoardName::parse(board)
        .map_err(|error| (Status::NotFound, format!("{error:#}")))
}

/// A message id is the UUID b4's `initiate` handed out, or 404. b4 hands
/// out the hyphenated form, which is what is forwarded.
fn message_id(id: &str) -> Result<Uuid, (Status, String)> {
    Uuid::parse_str(id)
        .map_err(|_| (Status::NotFound, format!("{id:?} is not a message id")))
}

/// The caller may write to `board`: it has the trustee ceremony permission
/// in its own tenant, and the board is one of its boards there.
async fn authorize_member(
    claims: &JwtClaims,
    board: &BoardName,
    services: &HarvestServices,
) -> Result<(), (Status, String)> {
    authorize(
        claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TRUSTEE_CEREMONY],
    )?;
    let tenant_id = &claims.hasura_claims.tenant_id;

    let mut hasura_db_client: DbClient = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    match trustee_boards::authorize_board(
        &hasura_transaction,
        tenant_id,
        claims.trustee.as_deref(),
        board,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?
    {
        BoardAccess::Member(_) => Ok(()),
        BoardAccess::Refused(refusal) => {
            warn!(
                trustee = ?claims.trustee,
                "board {board} refused: {refusal}"
            );
            Err((Status::Forbidden, refusal.to_string()))
        }
    }
}

/// The body of a board write, at most as large as b4 reads.
async fn read_body(body: Data<'_>) -> Result<Vec<u8>, (Status, String)> {
    let body = body
        .open(BODY_LIMIT_BYTES.bytes())
        .into_bytes()
        .await
        .map_err(|e| (Status::BadRequest, format!("{e:?}")))?;
    if !body.is_complete() {
        return Err((
            Status::PayloadTooLarge,
            format!(
                "the body exceeds the board service's {BODY_LIMIT_BYTES} bytes"
            ),
        ));
    }
    Ok(body.into_inner())
}

/// A board's messages, as b4 lists them, with the query (b4's `last_id`
/// and `limit`) passed on unchanged. Public: verifiers read boards too;
/// pages on other origins only as [`READ_ORIGINS_VARIABLE`] allows.
#[instrument(skip(services))]
#[get("/b4/boards/<board>/messages")]
pub async fn board_messages(
    board: &str,
    uri: &Origin<'_>,
    services: &State<HarvestServices>,
) -> PublicRead<Result<BoardReply, (Status, String)>> {
    PublicRead(
        services.b4.read_origins.clone(),
        read_messages(board, uri, services).await,
    )
}

async fn read_messages(
    board: &str,
    uri: &Origin<'_>,
    services: &HarvestServices,
) -> Result<BoardReply, (Status, String)> {
    let board = board_name(board)?;
    services
        .b4
        .get(
            &format!("/boards/{board}/messages"),
            uri.query().map(|query| query.as_str()),
        )
        .await
}

/// Step 1 of a board write: b4 reserves a message id and an upload URL.
#[instrument(skip(body, claims, services))]
#[post(
    "/b4/boards/<board>/messages/initiate",
    format = "json",
    data = "<body>"
)]
pub async fn initiate_board_message(
    board: &str,
    body: Data<'_>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> Result<BoardReply, (Status, String)> {
    let board = board_name(board)?;
    authorize_member(&claims, &board, services).await?;
    let body = read_body(body).await?;
    services
        .b4
        .post(&format!("/boards/{board}/messages/initiate"), body)
        .await
}

/// Step 2 of a board write: b4 records the uploaded message.
#[instrument(skip(body, claims, services))]
#[post(
    "/b4/boards/<board>/messages/<id>/confirm",
    format = "json",
    data = "<body>"
)]
pub async fn confirm_board_message(
    board: &str,
    id: &str,
    body: Data<'_>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> Result<BoardReply, (Status, String)> {
    let board = board_name(board)?;
    let id = message_id(id)?;
    authorize_member(&claims, &board, services).await?;
    let body = read_body(body).await?;
    services
        .b4
        .post(
            &format!("/boards/{board}/messages/{}/confirm", id.as_hyphenated()),
            body,
        )
        .await
}

#[cfg(test)]
#[path = "../../tests/support/b4_routes.rs"]
mod b4_routes;
