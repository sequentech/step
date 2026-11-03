// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Use cases consume scoped capabilities, never election or platform tables.
use crate::{
    csv_io,
    domain::{Project, ProjectState, Task, MAX_TASKS},
};
use async_trait::async_trait;
use sequent_plugin_api::{
    HistoryScope, HostPorts, Manifest, Operation, Plugin, PluginError, Record,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

/// Operation names, part of the plugin's versioned API.
pub mod operations {
    pub const LIST_PROJECTS: &str = "list-projects";
    pub const GET_PROJECT: &str = "get-project";
    pub const CREATE_PROJECT: &str = "create-project";
    pub const SET_PROJECT_STATE: &str = "set-project-state";
    pub const SAVE_TASK: &str = "save-task";
    pub const DELETE_TASK: &str = "delete-task";
    pub const PREVIEW_IMPORT: &str = "preview-import";
    pub const COMMIT_IMPORT: &str = "commit-import";
    pub const EXPORT: &str = "export";
    pub const HISTORY: &str = "history";
}
use operations::*;

pub const READ: &str = "projects-read";
pub const WRITE: &str = "projects-write";
pub const MANAGE: &str = "projects-manage";
const PROJECT: &str = "project";
const TASK: &str = "task";
const ACTION_IMPORT: &str = "import";
const ACTION_EXPORT: &str = "export";
const DEFAULT_PAGE_SIZE: u32 = 50;

pub static MANIFEST: Manifest = Manifest {
    id: "projects",
    version: "1.0.0",
    api_major: 1,
    data_version: 1,
    label_key: "projects.title",
    route: "/plugins/projects",
    read_permission: READ,
    permissions: &[READ, WRITE, MANAGE],
    record_kinds: &[PROJECT, TASK],
    operations: &[
        Operation {
            name: LIST_PROJECTS,
            permission: READ,
        },
        Operation {
            name: GET_PROJECT,
            permission: READ,
        },
        Operation {
            name: CREATE_PROJECT,
            permission: MANAGE,
        },
        Operation {
            name: SET_PROJECT_STATE,
            permission: MANAGE,
        },
        Operation {
            name: SAVE_TASK,
            permission: WRITE,
        },
        Operation {
            name: DELETE_TASK,
            permission: WRITE,
        },
        Operation {
            name: PREVIEW_IMPORT,
            permission: WRITE,
        },
        Operation {
            name: COMMIT_IMPORT,
            permission: WRITE,
        },
        Operation {
            name: EXPORT,
            permission: READ,
        },
        Operation {
            name: HISTORY,
            permission: READ,
        },
    ],
};

pub struct Projects;

fn input<T: DeserializeOwned>(value: Value) -> Result<T, PluginError> {
    serde_json::from_value(value).map_err(|error| PluginError::InvalidInput(error.to_string()))
}

fn output<T: Serialize>(value: T) -> Result<Value, PluginError> {
    serde_json::to_value(value).map_err(|error| PluginError::Storage(error.to_string()))
}

fn invalid(message: &str) -> PluginError {
    PluginError::InvalidInput(message.into())
}

fn default_page_size() -> u32 {
    DEFAULT_PAGE_SIZE
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Id {
    id: Uuid,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Page {
    #[serde(default)]
    offset: u32,
    #[serde(default = "default_page_size")]
    limit: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewProject {
    name: String,
    timezone: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SetProjectState {
    id: Uuid,
    expected_version: i32,
    state: ProjectState,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveTask {
    project_id: Uuid,
    id: Option<Uuid>,
    expected_version: Option<i32>,
    task: Task,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeleteTask {
    project_id: Uuid,
    id: Uuid,
    expected_version: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Csv {
    csv: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Import {
    name: String,
    timezone: String,
    csv: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct History {
    project_id: Uuid,
    /// A task of the project, which may have been deleted. Without it, the
    /// history covers the project and every task it has had.
    task_id: Option<Uuid>,
    before_id: Option<i64>,
    #[serde(default = "default_page_size")]
    limit: u32,
}

/// One row of the projects list.
#[derive(Serialize)]
struct ProjectSummary {
    #[serde(flatten)]
    project: Record,
    task_count: usize,
    start: Option<String>,
    end: Option<String>,
}

async fn project(host: &dyn HostPorts, id: Uuid) -> Result<(Record, Project), PluginError> {
    let saved = host.get(PROJECT, id).await?;
    let data: Project = input(saved.data.clone())?;
    Ok((saved, data))
}

async fn editable(host: &dyn HostPorts, id: Uuid) -> Result<(), PluginError> {
    if project(host, id).await?.1.state == ProjectState::Archived {
        return Err(invalid(
            "Restore this archived project before editing tasks",
        ));
    }
    Ok(())
}

async fn tasks(host: &dyn HostPorts, project: Uuid) -> Result<Vec<Record>, PluginError> {
    let limit =
        u32::try_from(MAX_TASKS).map_err(|error| PluginError::Storage(error.to_string()))?;
    let mut rows = host.list(TASK, Some(project), 0, limit).await?;
    rows.sort_by_key(|row| {
        (
            row.data.get("order").and_then(Value::as_i64).unwrap_or(0),
            row.created_at,
            row.id,
        )
    });
    Ok(rows)
}

async fn task_in_project(
    host: &dyn HostPorts,
    project: Uuid,
    id: Uuid,
) -> Result<Record, PluginError> {
    let task = host.get(TASK, id).await?;
    if task.parent_id != Some(project) {
        return Err(PluginError::NotFound);
    }
    Ok(task)
}

async fn create_project(
    host: &dyn HostPorts,
    name: String,
    timezone: String,
) -> Result<Record, PluginError> {
    let data = Project {
        name: name.trim().into(),
        timezone,
        state: ProjectState::Active,
    };
    data.validate()?;
    host.create(PROJECT, host.new_id(), None, output(data)?)
        .await
}

async fn summary(host: &dyn HostPorts, project: Record) -> Result<ProjectSummary, PluginError> {
    let tasks: Vec<Task> = tasks(host, project.id)
        .await?
        .into_iter()
        .map(|row| input(row.data))
        .collect::<Result<_, _>>()?;
    let (start, end) = Task::span(&tasks)?;
    Ok(ProjectSummary {
        project,
        task_count: tasks.len(),
        start,
        end,
    })
}

#[async_trait]
impl Plugin for Projects {
    fn manifest(&self) -> &'static Manifest {
        &MANIFEST
    }

    async fn execute(
        &self,
        operation: &str,
        value: Value,
        host: &dyn HostPorts,
    ) -> Result<Value, PluginError> {
        match operation {
            LIST_PROJECTS => {
                let page: Page = input(value)?;
                let mut items = Vec::new();
                for project in host.list(PROJECT, None, page.offset, page.limit).await? {
                    items.push(summary(host, project).await?);
                }
                output(json!({ "items": items }))
            }
            GET_PROJECT => {
                let request: Id = input(value)?;
                let (project, _) = project(host, request.id).await?;
                output(json!({
                    "project": project,
                    "tasks": tasks(host, request.id).await?,
                }))
            }
            CREATE_PROJECT => {
                let request: NewProject = input(value)?;
                output(create_project(host, request.name, request.timezone).await?)
            }
            SET_PROJECT_STATE => {
                let request: SetProjectState = input(value)?;
                let (_, mut data) = project(host, request.id).await?;
                data.state = request.state;
                output(
                    host.replace(PROJECT, request.id, request.expected_version, output(data)?)
                        .await?,
                )
            }
            SAVE_TASK => {
                let request: SaveTask = input(value)?;
                request.task.validate()?;
                editable(host, request.project_id).await?;
                let saved = match (request.id, request.expected_version) {
                    (Some(id), Some(version)) if version > 0 => {
                        task_in_project(host, request.project_id, id).await?;
                        host.replace(TASK, id, version, output(request.task)?)
                            .await?
                    }
                    (None, None) => {
                        if tasks(host, request.project_id).await?.len() >= MAX_TASKS
                        {
                            return Err(invalid(
                                "A project can contain at most 1000 tasks",
                            ));
                        }
                        host.create(
                            TASK,
                            host.new_id(),
                            Some(request.project_id),
                            output(request.task)?,
                        )
                        .await?
                    }
                    _ => {
                        return Err(invalid(
                            "Updating a task requires its id and expected version; new tasks supply neither",
                        ))
                    }
                };
                output(saved)
            }
            DELETE_TASK => {
                let request: DeleteTask = input(value)?;
                editable(host, request.project_id).await?;
                task_in_project(host, request.project_id, request.id).await?;
                host.delete(TASK, request.id, request.expected_version)
                    .await?;
                Ok(json!({ "deleted": request.id }))
            }
            PREVIEW_IMPORT => {
                let request: Csv = input(value)?;
                output(csv_io::preview(&request.csv)?)
            }
            COMMIT_IMPORT => {
                let request: Import = input(value)?;
                // Revalidation and all writes occur inside one host transaction.
                // A preview response is never accepted as proof of validity.
                let preview = csv_io::preview(&request.csv)?;
                if !preview.valid {
                    return Err(invalid(
                        "The CSV has invalid rows or contains no tasks. Correct it and preview again",
                    ));
                }
                let project = create_project(host, request.name, request.timezone).await?;
                let count = preview.rows.len();
                for row in preview.rows {
                    let task = row.task.ok_or_else(|| invalid("Invalid CSV row"))?;
                    host.create(TASK, host.new_id(), Some(project.id), output(task)?)
                        .await?;
                }
                host.record(
                    ACTION_IMPORT,
                    Some(project.id),
                    None,
                    Some(json!({ "task_count": count })),
                )
                .await?;
                output(project)
            }
            EXPORT => {
                let request: Id = input(value)?;
                let (_, data) = project(host, request.id).await?;
                let tasks: Vec<(Uuid, Task)> = tasks(host, request.id)
                    .await?
                    .into_iter()
                    .map(|row| Ok((row.id, input(row.data)?)))
                    .collect::<Result<_, PluginError>>()?;
                let csv = csv_io::export(&tasks, &data.timezone)?;
                host.record(
                    ACTION_EXPORT,
                    Some(request.id),
                    None,
                    Some(json!({ "task_count": tasks.len() })),
                )
                .await?;
                Ok(json!({
                    "csv": csv,
                    "filename": csv_io::filename(&data.name),
                }))
            }
            HISTORY => {
                let request: History = input(value)?;
                project(host, request.project_id).await?;
                let scope = match request.task_id {
                    Some(id) => HistoryScope::Child {
                        parent: request.project_id,
                        id,
                    },
                    None => HistoryScope::RecordAndChildren(request.project_id),
                };
                output(json!({
                    "events": host
                        .history(scope, request.before_id, request.limit)
                        .await?,
                }))
            }
            _ => Err(PluginError::NotFound),
        }
    }
}
