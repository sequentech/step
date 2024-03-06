# Import Election Events

## JSON Schema

We import `election_events` to a tenant.

```json
[
  {
    "tenant_id": "",
    "election_events": [
      {
        "keycloak_realm": {},
        "data": { "id": "" },
        "elections": [
          {
            "id": "",
            "data": {},
            "contests": [
              {
                "id": "",
                "data": {},
                "candidates": [{ "id": "", "data": {} }]
              }
            ]
          }
        ]
      }
    ],
    "areas": [],
    "area_contest": []
  }
]
```

```rust
pub struct ElectionEvent {
    id: Uuid,
    tenant_id: Uuid,
    data: ElectionEventData,
    elections: Vec<Election>,
    keycloak_event_realm: RealmRepresentation,
}

pub struct Election {
    id: Uuid,
    data: ElectionData,
    contests: Vec<Contest>,
}

pub struct Contest {
    id: Uuid,
    data: ContestData,
    candidates: Vec<Candidate>,
    area_id: Uuid,
}

pub struct Candidate {
    id: Uuid,
    data: CandidateData,
}

pub struct AreaContest {
    area_id: Uuid,
    contest: Uuid,
}

pub struct JsonSchemaImportElectionEvents {
    events: Vec<ElectionEvent>,
    areas: Vec<AreaData>,
    area_contest: Vec<AreaContest>,
}
```

## Import

1. Validate JSON schema from user input
2. Import and create Keycloak election event realm
3. Import election events
4. Import elections
5. Import contests
6. Import candidates

## Flow

User send a request to *Harvest*. *Harvest* create a task on *Windmill*. *Windmill* executes the importation.

### Schema Validation

We can use this [validator crate](https://crates.io/crates/validator).

### Keycloak Realms

```.env.development
# Path to the default configuration file when creating a new realm related to an
# election event. For production, this can be coming from a mounted volume, so
# that it can be changed without requiring a new OCI/Docker image.
KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_PATH=/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-33f18502-a67c-4853-8333-a58630663559.json

# Path to the default configuration file when creating a new realm related to
# a tenant. For production, this can be coming from a mounted volume, so that
# it can be changed without requiring a new OCI/Docker image.
KEYCLOAK_TENANT_REALM_CONFIG_PATH=/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json
```

The master realm: `master`

The tenant realm:
tenant_id: `tenant-{uuid}`

The election event realm:
tenant_id, event_id: `tenant-{uuid}-event-{uuid}`

```rust
    let realm_name = get_event_realm(tenant_id, election_event_id);
```

Each tenant has its own Keycloak realm, called tenant/<tenant-id> and each election event has its own realm, called tenant/<tenant-id>/event/<election-event-id>.

<!-- #### What is jwk? -->
<!---->
<!-- JWK_URL stands for JSON Web Key URL. It's a URL that points to a set of public keys in a JSON Web Key (JWK) format. These public keys are used to verify the signature of JSON Web Tokens (JWTs) issued by an authorization server or identity provider, like Keycloak. -->
<!---->
<!-- We have one Keycloak realm per Election Event. -->
<!---->
<!-- Keycloak provides a JWK_URL per realm. For example, the master realm has the following JWK URL in the devcontainer docker compose: [http://keycloak:8090/realms/admin/protocol/openid-connect/certs]. We could easily just configure HASURA_GRAPHQL_JWT_SECRET to thus be: -->
<!---->
<!-- {"jwk_url": "http://keycloak:8090/realms/electoral-process/protocol/openid-connect/certs"} -->
<!---->
<!-- The content of that url would be something like what follows: `.devcontainer/minio/certs.json` -->

### Election Events

```rust
pub struct ElectionEventData {
    pub id: Uuid,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub tenant_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub presentation: Option<Value>,
    pub bulletin_board_reference: Option<Value>,
    pub is_archived: bool,
    pub voting_channels: Option<Value>,
    pub dates: Option<Value>,
    pub status: Option<Value>,
    pub user_boards: Option<String>,
    pub encryption_protocol: String,
    pub is_audit: Option<bool>,
    pub audit_election_event_id: Option<Uuid>,
    pub public_key: Option<String>,
}
```

### Elections

```rust
pub struct ElectionData {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub created_at: Option<NaiveDateTime>,
    pub last_updated_at: Option<NaiveDateTime>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub name: String,
    pub description: Option<String>,
    pub presentation: Option<Value>,
    pub dates: Option<Value>,
    pub status: Option<Value>,
    pub eml: Option<String>,
    pub num_allowed_revotes: Option<i64>,
    pub is_consolidated_ballot_encoding: Option<bool>,
    pub spoil_ballot_option: Option<bool>,
    pub is_kiosk: Option<bool>,
    pub alias: Option<String>,
    pub voting_channels: Option<Value>,
    pub image_document_id: Option<String>,
    pub statistics: Option<Value>,
    pub receipts: Option<Value>,
}
```

### Contests

```rust
pub struct ContestData {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub election_id: Uuid,
    pub created_at: Option<NaiveDateTime>,
    pub last_updated_at: Option<NaiveDateTime>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub is_acclaimed: Option<bool>,
    pub is_active: Option<bool>,
    pub name: Option<String>,
    pub alias: Option<String>,
    pub description: Option<String>,
    pub presentation: Option<Value>,
    pub min_votes: Option<i64>,
    pub max_votes: Option<i64>,
    pub winning_candidates_num: Option<i64>,
    pub voting_type: Option<String>,
    pub counting_algorithm: Option<String>,
    pub is_encrypted: Option<bool>,
    pub tally_configuration: Option<Value>,
    pub conditions: Option<Value>,
}
```

### Candidates

```rust
pub struct CandidateData {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub contest_id: Option<Uuid>,
    pub created_at: Option<NaiveDateTime>,
    pub last_updated_at: Option<NaiveDateTime>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub name: Option<String>,
    pub alias: Option<String>,
    pub description: Option<String>,
    pub r#type: Option<String>,
    pub presentation: Option<Value>,
    pub is_public: Option<bool>,
}
```

### Areas

```rust
pub struct AreaData {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub created_at: Option<NaiveDateTime>,
    pub last_updated_at: Option<NaiveDateTime>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub r#type: Option<String>,
}
```
