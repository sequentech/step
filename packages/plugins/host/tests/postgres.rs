// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Integration contract against PostgreSQL. `PLUGIN_TEST_DATABASE_URL` names a
//! disposable server; each test creates and drops its own database with a stub
//! tenant table and the real migration. No other database is read or changed.
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sequent_plugin_api::{
    Audit, AuditEvent, Clock, HistoryScope, HostPorts, Manifest, Plugin, PluginError, Record,
    RecordStore,
};
use sequent_plugin_host::{
    postgres::{Compatibility, PluginState, Registry, RoleProvisioner},
    Identity,
};
use sequent_plugin_projects::{application::MANIFEST, Projects};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio_postgres::{Client, Config, NoTls};
use uuid::Uuid;

const MIGRATION: &str =
    include_str!("../../../../hasura/migrations/backend-db/1792266000000_plugin_platform/up.sql");
const STUB_SCHEMA: &str = "CREATE SCHEMA sequent_backend;
    CREATE TABLE sequent_backend.tenant (id uuid PRIMARY KEY);
    CREATE TABLE sequent_backend.scheduled_event (id uuid PRIMARY KEY, value text);
    INSERT INTO sequent_backend.scheduled_event
        VALUES ('00000000-0000-4000-8000-000000000001', 'untouched');
    -- The admin audit trigger comes from earlier platform migrations; these
    -- tests check the plugin tables, so a stand-in that logs nothing is enough.
    CREATE FUNCTION sequent_backend.audit_change() RETURNS trigger
        LANGUAGE plpgsql AS $$ BEGIN RETURN NULL; END $$;";
const PROJECTS: &str = "projects";
const ALL_ROLES: [&str; 5] = [
    "admin-user",
    "tenant-write",
    "projects-read",
    "projects-write",
    "projects-manage",
];

/// A disposable database, dropped when the test finishes.
struct TestDb {
    client: Client,
    admin: Client,
    name: String,
}

impl TestDb {
    async fn new() -> Self {
        let url = std::env::var("PLUGIN_TEST_DATABASE_URL")
            .expect("PLUGIN_TEST_DATABASE_URL of a disposable PostgreSQL server");
        let config: Config = url.parse().expect("a PostgreSQL URL");
        let (admin, connection) = config.connect(NoTls).await.unwrap();
        tokio::spawn(connection);
        let name = format!("plugin_test_{}", Uuid::new_v4().simple());
        admin
            .batch_execute(&format!("CREATE DATABASE {name}"))
            .await
            .unwrap();
        let mut scoped = config.clone();
        scoped.dbname(&name);
        let (client, connection) = scoped.connect(NoTls).await.unwrap();
        tokio::spawn(connection);
        client.batch_execute(STUB_SCHEMA).await.unwrap();
        client.batch_execute(MIGRATION).await.unwrap();
        Self {
            client,
            admin,
            name,
        }
    }

    async fn tenant(&self) -> Identity {
        let identity = Identity {
            tenant_id: Uuid::new_v4(),
            user_id: "test-admin".into(),
            roles: ALL_ROLES.map(String::from).to_vec(),
        };
        self.client
            .execute(
                "INSERT INTO sequent_backend.tenant VALUES ($1)",
                &[&identity.tenant_id],
            )
            .await
            .unwrap();
        identity
    }

    async fn count(&self, table: &str, tenant: Uuid) -> i64 {
        self.client
            .query_one(
                &format!("SELECT count(*) FROM sequent_backend.{table} WHERE tenant_id=$1"),
                &[&tenant],
            )
            .await
            .unwrap()
            .get(0)
    }

    async fn remove(self) {
        drop(self.client);
        self.admin
            .batch_execute(&format!("DROP DATABASE {} WITH (FORCE)", self.name))
            .await
            .unwrap();
    }
}

enum Provisioning {
    Succeeds,
    Fails,
}

#[async_trait]
impl RoleProvisioner for Provisioning {
    async fn ensure_admin_roles(&self, _: Uuid, _: &[&str]) -> Result<(), PluginError> {
        match self {
            Self::Succeeds => Ok(()),
            Self::Fails => Err(PluginError::Storage("Keycloak is down".into())),
        }
    }
}

/// Commits only on success, like the gateway.
async fn execute(
    db: &mut TestDb,
    registry: &Registry,
    who: &Identity,
    plugin: &str,
    op: &str,
    input: Value,
) -> Result<Value, PluginError> {
    let tx = db.client.transaction().await.unwrap();
    let result = registry.execute(&tx, who, plugin, op, input).await?;
    tx.commit().await.unwrap();
    Ok(result)
}

async fn set_state(
    db: &mut TestDb,
    registry: &Registry,
    who: &Identity,
    plugin: &str,
    state: PluginState,
    version: i32,
    roles: Provisioning,
) -> Result<(), PluginError> {
    let tx = db.client.transaction().await.unwrap();
    registry
        .set_state(&tx, who, plugin, state, version, &roles)
        .await?;
    tx.commit().await.unwrap();
    Ok(())
}

async fn enable(db: &mut TestDb, registry: &Registry, who: &Identity) {
    set_state(
        db,
        registry,
        who,
        PROJECTS,
        PluginState::Enabled,
        0,
        Provisioning::Succeeds,
    )
    .await
    .unwrap();
}

fn record(value: Value) -> Record {
    serde_json::from_value(value).unwrap()
}

fn task(title: &str) -> Value {
    json!({
        "title": title, "type": "task",
        "start": "2028-03-01T09:00:00+08:00", "end": "2028-03-01T10:00:00+08:00",
        "status": "not_started", "notes": "original", "order": 0,
    })
}

async fn new_project(db: &mut TestDb, registry: &Registry, who: &Identity) -> Record {
    record(
        execute(
            db,
            registry,
            who,
            PROJECTS,
            "create-project",
            json!({"name": "Plan", "timezone": "Asia/Manila"}),
        )
        .await
        .unwrap(),
    )
}

static OTHER: Manifest = Manifest {
    id: "other",
    ..MANIFEST
};

/// Another plugin with the same record kinds, to prove namespace isolation.
struct OtherPlugin;

#[async_trait]
impl Plugin for OtherPlugin {
    fn manifest(&self) -> &'static Manifest {
        &OTHER
    }

    async fn execute(
        &self,
        operation: &str,
        input: Value,
        host: &dyn HostPorts,
    ) -> Result<Value, PluginError> {
        Projects.execute(operation, input, host).await
    }
}

static FAILING: Manifest = Manifest {
    id: "failing",
    ..MANIFEST
};

/// Delegates to Projects, but its host fails on the third record created.
struct FailingWrites;

struct FailOnThirdCreate<'a> {
    host: &'a dyn HostPorts,
    created: AtomicUsize,
}

#[async_trait]
impl RecordStore for FailOnThirdCreate<'_> {
    async fn get(&self, kind: &str, id: Uuid) -> Result<Record, PluginError> {
        self.host.get(kind, id).await
    }
    async fn list(
        &self,
        kind: &str,
        parent: Option<Uuid>,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<Record>, PluginError> {
        self.host.list(kind, parent, offset, limit).await
    }
    async fn create(
        &self,
        kind: &str,
        id: Uuid,
        parent: Option<Uuid>,
        data: Value,
    ) -> Result<Record, PluginError> {
        if self.created.fetch_add(1, Ordering::SeqCst) == 2 {
            return Err(PluginError::Storage("disk full".into()));
        }
        self.host.create(kind, id, parent, data).await
    }
    async fn replace(
        &self,
        kind: &str,
        id: Uuid,
        version: i32,
        data: Value,
    ) -> Result<Record, PluginError> {
        self.host.replace(kind, id, version, data).await
    }
    async fn delete(&self, kind: &str, id: Uuid, version: i32) -> Result<(), PluginError> {
        self.host.delete(kind, id, version).await
    }
}

#[async_trait]
impl Audit for FailOnThirdCreate<'_> {
    async fn record(
        &self,
        action: &str,
        record_id: Option<Uuid>,
        before: Option<Value>,
        after: Option<Value>,
    ) -> Result<(), PluginError> {
        self.host.record(action, record_id, before, after).await
    }
    async fn history(
        &self,
        scope: HistoryScope,
        before_id: Option<i64>,
        limit: u32,
    ) -> Result<Vec<AuditEvent>, PluginError> {
        self.host.history(scope, before_id, limit).await
    }
}

impl Clock for FailOnThirdCreate<'_> {
    fn now(&self) -> DateTime<Utc> {
        self.host.now()
    }
    fn new_id(&self) -> Uuid {
        self.host.new_id()
    }
}

#[async_trait]
impl Plugin for FailingWrites {
    fn manifest(&self) -> &'static Manifest {
        &FAILING
    }

    async fn execute(
        &self,
        operation: &str,
        input: Value,
        host: &dyn HostPorts,
    ) -> Result<Value, PluginError> {
        let host = FailOnThirdCreate {
            host,
            created: AtomicUsize::new(0),
        };
        Projects.execute(operation, input, &host).await
    }
}

fn registry() -> Registry {
    Registry::new(vec![
        Box::new(Projects),
        Box::new(OtherPlugin),
        Box::new(FailingWrites),
    ])
    .unwrap()
}

#[tokio::test]
#[ignore = "needs PLUGIN_TEST_DATABASE_URL of a disposable PostgreSQL server"]
async fn tenants_plugins_and_roles_are_isolated() {
    let mut db = TestDb::new().await;
    let registry = registry();
    let (a, b) = (db.tenant().await, db.tenant().await);
    assert!(matches!(
        execute(&mut db, &registry, &a, PROJECTS, "list-projects", json!({})).await,
        Err(PluginError::Disabled)
    ));
    enable(&mut db, &registry, &a).await;
    enable(&mut db, &registry, &b).await;
    set_state(
        &mut db,
        &registry,
        &a,
        "other",
        PluginState::Enabled,
        0,
        Provisioning::Succeeds,
    )
    .await
    .unwrap();
    let project = new_project(&mut db, &registry, &a).await;
    let saved = record(
        execute(
            &mut db,
            &registry,
            &a,
            PROJECTS,
            "save-task",
            json!({"project_id": project.id, "task": task("Readiness")}),
        )
        .await
        .unwrap(),
    );
    // Neither another tenant nor another plugin can read or change it.
    for (who, plugin) in [(&b, PROJECTS), (&a, "other")] {
        assert!(matches!(
            execute(
                &mut db,
                &registry,
                who,
                plugin,
                "get-project",
                json!({"id": project.id})
            )
            .await,
            Err(PluginError::NotFound)
        ));
        assert!(matches!(
            execute(
                &mut db,
                &registry,
                who,
                plugin,
                "save-task",
                json!({"project_id": project.id, "id": saved.id,
                       "expected_version": 1, "task": task("Stolen")}),
            )
            .await,
            Err(PluginError::NotFound)
        ));
    }
    let listed = execute(&mut db, &registry, &b, PROJECTS, "list-projects", json!({}))
        .await
        .unwrap();
    assert_eq!(listed["items"].as_array().unwrap().len(), 0);

    let mut reader = a.clone();
    reader.roles = vec!["admin-user".into(), "projects-read".into()];
    assert!(execute(
        &mut db,
        &registry,
        &reader,
        PROJECTS,
        "get-project",
        json!({"id": project.id})
    )
    .await
    .is_ok());
    for (op, input) in [
        (
            "save-task",
            json!({"project_id": project.id, "task": task("New")}),
        ),
        ("create-project", json!({"name": "Plan", "timezone": "UTC"})),
    ] {
        assert!(matches!(
            execute(&mut db, &registry, &reader, PROJECTS, op, input).await,
            Err(PluginError::Forbidden)
        ));
    }
    let mut editor = a.clone();
    editor.roles = vec![
        "admin-user".into(),
        "projects-read".into(),
        "projects-write".into(),
    ];
    assert!(matches!(
        execute(
            &mut db,
            &registry,
            &editor,
            PROJECTS,
            "set-project-state",
            json!({"id": project.id, "expected_version": 1, "state": "archived"})
        )
        .await,
        Err(PluginError::Forbidden)
    ));
    let mut no_admin = a.clone();
    no_admin.roles.retain(|role| role != "admin-user");
    assert!(matches!(
        execute(
            &mut db,
            &registry,
            &no_admin,
            PROJECTS,
            "list-projects",
            json!({})
        )
        .await,
        Err(PluginError::Forbidden)
    ));
    // Strict decoding refuses attempts to smuggle a tenant or actor.
    assert!(matches!(
        execute(
            &mut db,
            &registry,
            &a,
            PROJECTS,
            "create-project",
            json!({"name": "Bad", "timezone": "UTC", "tenant_id": b.tenant_id})
        )
        .await,
        Err(PluginError::InvalidInput(_))
    ));
    db.remove().await;
}

#[tokio::test]
#[ignore = "needs PLUGIN_TEST_DATABASE_URL of a disposable PostgreSQL server"]
async fn lifecycle_keeps_data_and_failed_provisioning_stays_disabled() {
    let mut db = TestDb::new().await;
    let registry = registry();
    let a = db.tenant().await;
    let failed = set_state(
        &mut db,
        &registry,
        &a,
        PROJECTS,
        PluginState::Enabled,
        0,
        Provisioning::Fails,
    )
    .await;
    assert!(matches!(failed, Err(PluginError::Storage(_))));
    assert_eq!(db.count("plugin_tenant", a.tenant_id).await, 0);
    assert_eq!(db.count("plugin_record_event", a.tenant_id).await, 0);

    let mut settings_reader = a.clone();
    settings_reader.roles.retain(|role| role != "tenant-write");
    assert!(matches!(
        set_state(
            &mut db,
            &registry,
            &settings_reader,
            PROJECTS,
            PluginState::Enabled,
            0,
            Provisioning::Succeeds
        )
        .await,
        Err(PluginError::Forbidden)
    ));

    enable(&mut db, &registry, &a).await;
    assert!(matches!(
        set_state(
            &mut db,
            &registry,
            &a,
            PROJECTS,
            PluginState::Disabled,
            0,
            Provisioning::Succeeds
        )
        .await,
        Err(PluginError::StateConflict)
    ));
    let project = new_project(&mut db, &registry, &a).await;
    set_state(
        &mut db,
        &registry,
        &a,
        PROJECTS,
        PluginState::Disabled,
        1,
        Provisioning::Succeeds,
    )
    .await
    .unwrap();
    assert!(matches!(
        execute(
            &mut db,
            &registry,
            &a,
            PROJECTS,
            "get-project",
            json!({"id": project.id})
        )
        .await,
        Err(PluginError::Disabled)
    ));
    // Turning it off again provisions nothing, so a Keycloak outage cannot block it.
    set_state(
        &mut db,
        &registry,
        &a,
        PROJECTS,
        PluginState::Enabled,
        2,
        Provisioning::Succeeds,
    )
    .await
    .unwrap();
    let restored = execute(
        &mut db,
        &registry,
        &a,
        PROJECTS,
        "get-project",
        json!({"id": project.id}),
    )
    .await
    .unwrap();
    assert_eq!(restored["project"]["data"]["name"], "Plan");
    set_state(
        &mut db,
        &registry,
        &a,
        PROJECTS,
        PluginState::Disabled,
        3,
        Provisioning::Fails,
    )
    .await
    .unwrap();

    // A newer stored data version than this release understands is refused.
    db.client
        .execute(
            "UPDATE sequent_backend.plugin_tenant SET data_version=2 WHERE tenant_id=$1",
            &[&a.tenant_id],
        )
        .await
        .unwrap();
    let tx = db.client.transaction().await.unwrap();
    let listed = registry.list(&tx, &a).await.unwrap();
    drop(tx);
    let projects = listed.iter().find(|p| p.manifest.id == PROJECTS).unwrap();
    assert_eq!(projects.compatibility, Compatibility::Incompatible);
    assert!(matches!(
        set_state(
            &mut db,
            &registry,
            &a,
            PROJECTS,
            PluginState::Enabled,
            4,
            Provisioning::Succeeds
        )
        .await,
        Err(PluginError::Incompatible)
    ));
    db.client
        .execute(
            "UPDATE sequent_backend.plugin_tenant SET state='enabled' WHERE tenant_id=$1",
            &[&a.tenant_id],
        )
        .await
        .unwrap();
    assert!(matches!(
        execute(&mut db, &registry, &a, PROJECTS, "list-projects", json!({})).await,
        Err(PluginError::Incompatible)
    ));
    db.remove().await;
}

#[tokio::test]
#[ignore = "needs PLUGIN_TEST_DATABASE_URL of a disposable PostgreSQL server"]
async fn conflicts_archive_and_delete() {
    let mut db = TestDb::new().await;
    let registry = registry();
    let a = db.tenant().await;
    enable(&mut db, &registry, &a).await;
    let project = new_project(&mut db, &registry, &a).await;
    let saved = record(
        execute(
            &mut db,
            &registry,
            &a,
            PROJECTS,
            "save-task",
            json!({"project_id": project.id, "task": task("Readiness")}),
        )
        .await
        .unwrap(),
    );
    let update = |version: i32| {
        json!({"project_id": project.id, "id": saved.id,
               "expected_version": version, "task": task("Updated")})
    };
    let updated = execute(&mut db, &registry, &a, PROJECTS, "save-task", update(1))
        .await
        .unwrap();
    assert_eq!(updated["version"], 2);
    let stale = execute(&mut db, &registry, &a, PROJECTS, "save-task", update(1)).await;
    assert!(
        matches!(stale, Err(PluginError::Conflict(current)) if current.version == 2 && current.data["title"] == "Updated")
    );
    assert!(matches!(
        execute(
            &mut db,
            &registry,
            &a,
            PROJECTS,
            "delete-task",
            json!({"project_id": project.id, "id": saved.id, "expected_version": 1})
        )
        .await,
        Err(PluginError::Conflict(_))
    ));

    let archived = execute(
        &mut db,
        &registry,
        &a,
        PROJECTS,
        "set-project-state",
        json!({"id": project.id, "expected_version": 1, "state": "archived"}),
    )
    .await
    .unwrap();
    assert_eq!(archived["data"]["state"], "archived");
    for (op, input) in [
        ("save-task", update(2)),
        (
            "save-task",
            json!({"project_id": project.id, "task": task("New")}),
        ),
        (
            "delete-task",
            json!({"project_id": project.id, "id": saved.id, "expected_version": 2}),
        ),
    ] {
        assert!(matches!(
            execute(&mut db, &registry, &a, PROJECTS, op, input).await,
            Err(PluginError::InvalidInput(_))
        ));
    }
    assert!(execute(
        &mut db,
        &registry,
        &a,
        PROJECTS,
        "export",
        json!({"id": project.id})
    )
    .await
    .is_ok());
    execute(
        &mut db,
        &registry,
        &a,
        PROJECTS,
        "set-project-state",
        json!({"id": project.id, "expected_version": 2, "state": "active"}),
    )
    .await
    .unwrap();
    execute(
        &mut db,
        &registry,
        &a,
        PROJECTS,
        "delete-task",
        json!({"project_id": project.id, "id": saved.id, "expected_version": 2}),
    )
    .await
    .unwrap();
    let detail = execute(
        &mut db,
        &registry,
        &a,
        PROJECTS,
        "get-project",
        json!({"id": project.id}),
    )
    .await
    .unwrap();
    assert!(detail["tasks"].as_array().unwrap().is_empty());
    db.remove().await;
}

#[tokio::test]
#[ignore = "needs PLUGIN_TEST_DATABASE_URL of a disposable PostgreSQL server"]
async fn task_history_survives_deletion_and_stays_in_its_project() {
    let mut db = TestDb::new().await;
    let registry = registry();
    let a = db.tenant().await;
    enable(&mut db, &registry, &a).await;
    let project = new_project(&mut db, &registry, &a).await;
    let other = new_project(&mut db, &registry, &a).await;
    let saved = record(
        execute(
            &mut db,
            &registry,
            &a,
            PROJECTS,
            "save-task",
            json!({"project_id": project.id, "task": task("Readiness")}),
        )
        .await
        .unwrap(),
    );
    execute(
        &mut db,
        &registry,
        &a,
        PROJECTS,
        "save-task",
        json!({"project_id": project.id, "id": saved.id, "expected_version": 1,
               "task": task("Renamed")}),
    )
    .await
    .unwrap();
    execute(
        &mut db,
        &registry,
        &a,
        PROJECTS,
        "delete-task",
        json!({"project_id": project.id, "id": saved.id, "expected_version": 2}),
    )
    .await
    .unwrap();

    let mut reader = a.clone();
    reader.roles = vec!["admin-user".into(), "projects-read".into()];
    let task_history = execute(
        &mut db,
        &registry,
        &reader,
        PROJECTS,
        "history",
        json!({"project_id": project.id, "task_id": saved.id}),
    )
    .await
    .unwrap();
    let actions: Vec<&str> = task_history["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event["action"].as_str().unwrap())
        .collect();
    assert_eq!(actions, ["delete", "update", "create"]);
    assert_eq!(
        task_history["events"][0]["before"]["data"]["title"],
        "Renamed"
    );
    assert_eq!(task_history["events"][0]["actor"], "test-admin");

    let project_history = execute(
        &mut db,
        &registry,
        &reader,
        PROJECTS,
        "history",
        json!({"project_id": project.id}),
    )
    .await
    .unwrap();
    let events = project_history["events"].as_array().unwrap();
    assert_eq!(events.len(), 4, "{events:?}");
    assert_eq!(events[0]["action"], "delete");
    assert_eq!(events[3]["record_id"], json!(project.id));

    let paged = execute(
        &mut db,
        &registry,
        &reader,
        PROJECTS,
        "history",
        json!({"project_id": project.id, "before_id": events[1]["id"], "limit": 1}),
    )
    .await
    .unwrap();
    assert_eq!(paged["events"].as_array().unwrap().len(), 1);
    assert_eq!(paged["events"][0]["id"], events[2]["id"]);

    // The task never belonged to the other project.
    let foreign = execute(
        &mut db,
        &registry,
        &reader,
        PROJECTS,
        "history",
        json!({"project_id": other.id, "task_id": saved.id}),
    )
    .await
    .unwrap();
    assert!(foreign["events"].as_array().unwrap().is_empty());
    assert!(db
        .client
        .execute(
            "UPDATE sequent_backend.plugin_record_event SET actor='spoofed'",
            &[]
        )
        .await
        .is_err());
    assert!(db
        .client
        .execute("DELETE FROM sequent_backend.plugin_record_event", &[])
        .await
        .is_err());
    db.remove().await;
}

#[tokio::test]
#[ignore = "needs PLUGIN_TEST_DATABASE_URL of a disposable PostgreSQL server"]
async fn imports_are_atomic_and_exports_round_trip() {
    let mut db = TestDb::new().await;
    let registry = registry();
    let a = db.tenant().await;
    enable(&mut db, &registry, &a).await;
    set_state(
        &mut db,
        &registry,
        &a,
        "failing",
        PluginState::Enabled,
        0,
        Provisioning::Succeeds,
    )
    .await
    .unwrap();
    let project = new_project(&mut db, &registry, &a).await;
    for title in ["Readiness", "=SUM(A1)"] {
        execute(
            &mut db,
            &registry,
            &a,
            PROJECTS,
            "save-task",
            json!({"project_id": project.id, "task": task(title)}),
        )
        .await
        .unwrap();
    }
    let records = db.count("plugin_record", a.tenant_id).await;
    let events = db.count("plugin_record_event", a.tenant_id).await;

    let invalid = "id,title,type,start,end,status,notes\n\
        ,Valid,milestone,2028-03-01T09:00:00+08:00,,done,\n\
        ,Invalid,task,wrong,,done,\n";
    assert!(matches!(
        execute(
            &mut db,
            &registry,
            &a,
            PROJECTS,
            "commit-import",
            json!({"name": "Invalid", "timezone": "Asia/Manila", "csv": invalid})
        )
        .await,
        Err(PluginError::InvalidInput(_))
    ));
    let mut reader = a.clone();
    reader.roles = vec!["admin-user".into(), "projects-read".into()];
    let exported = execute(
        &mut db,
        &registry,
        &reader,
        PROJECTS,
        "export",
        json!({"id": project.id}),
    )
    .await
    .unwrap();
    assert_eq!(exported["filename"], "plan.csv");
    assert!(exported["csv"].as_str().unwrap().contains(",'=SUM(A1),"));

    // A storage failure after the project and first task were written.
    assert!(matches!(
        execute(
            &mut db,
            &registry,
            &a,
            "failing",
            "commit-import",
            json!({"name": "Copy", "timezone": "Asia/Manila", "csv": exported["csv"]})
        )
        .await,
        Err(PluginError::Storage(_))
    ));
    assert_eq!(db.count("plugin_record", a.tenant_id).await, records);
    // Only the export event was added.
    assert_eq!(
        db.count("plugin_record_event", a.tenant_id).await,
        events + 1
    );

    let imported = record(
        execute(
            &mut db,
            &registry,
            &a,
            PROJECTS,
            "commit-import",
            json!({"name": "Copy", "timezone": "Asia/Manila", "csv": exported["csv"]}),
        )
        .await
        .unwrap(),
    );
    assert_ne!(imported.id, project.id);
    let copy = execute(
        &mut db,
        &registry,
        &a,
        PROJECTS,
        "get-project",
        json!({"id": imported.id}),
    )
    .await
    .unwrap();
    let titles: Vec<&str> = copy["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|task| task["data"]["title"].as_str().unwrap())
        .collect();
    assert_eq!(titles, ["Readiness", "=SUM(A1)"]);
    let listed = execute(&mut db, &registry, &a, PROJECTS, "list-projects", json!({}))
        .await
        .unwrap();
    assert_eq!(listed["items"][1]["task_count"], 2);
    assert_eq!(listed["items"][1]["start"], "2028-03-01T09:00:00+08:00");
    db.remove().await;
}

#[tokio::test]
#[ignore = "needs PLUGIN_TEST_DATABASE_URL of a disposable PostgreSQL server"]
async fn election_tables_are_untouched_and_tenant_deletion_cascades() {
    let mut db = TestDb::new().await;
    let registry = registry();
    let (a, b) = (db.tenant().await, db.tenant().await);
    enable(&mut db, &registry, &a).await;
    enable(&mut db, &registry, &b).await;
    for who in [&a, &b] {
        let project = new_project(&mut db, &registry, who).await;
        execute(
            &mut db,
            &registry,
            who,
            PROJECTS,
            "save-task",
            json!({"project_id": project.id, "task": task("Readiness")}),
        )
        .await
        .unwrap();
    }
    let core: String = db
        .client
        .query_one("SELECT value FROM sequent_backend.scheduled_event", &[])
        .await
        .unwrap()
        .get(0);
    assert_eq!(core, "untouched");

    db.client
        .execute(
            "DELETE FROM sequent_backend.tenant WHERE id=$1",
            &[&a.tenant_id],
        )
        .await
        .unwrap();
    for table in ["plugin_tenant", "plugin_record", "plugin_record_event"] {
        assert_eq!(db.count(table, a.tenant_id).await, 0, "{table}");
        assert!(db.count(table, b.tenant_id).await > 0, "{table}");
    }
    db.remove().await;
}
