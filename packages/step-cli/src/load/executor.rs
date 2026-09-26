// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Local processes, Docker containers and indexed Kubernetes pods share shard ownership.
use super::{config::Settings, coordinator, files, input::Input, report, worker};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

/// Translate the longest enclosing bind mount from devcontainer to daemon-host paths.
pub fn mapped_mount(directory: &Path, mounts: &[Value]) -> Result<PathBuf> {
    let best = mounts
        .iter()
        .filter_map(|mount| {
            let destination = Path::new(mount["Destination"].as_str()?);
            let relative = directory.strip_prefix(destination).ok()?;
            Some((
                destination.components().count(),
                Path::new(mount["Source"].as_str()?).join(relative),
            ))
        })
        .max_by_key(|(length, _)| *length)
        .context("Run is outside shared mounts; set execution.docker_mount_source")?;
    Ok(best.1)
}
fn docker_mount(directory: &Path, settings: &Settings) -> Result<PathBuf> {
    if let Some(path) = &settings.execution.docker_mount_source {
        return Ok(path.clone());
    }
    if !Path::new("/.dockerenv").exists() {
        return Ok(directory.into());
    }
    let hostname = fs::read_to_string("/etc/hostname")?;
    let output = Command::new("docker")
        .args(["inspect", hostname.trim(), "--format", "{{json .Mounts}}"])
        .output()
        .context("Cannot run docker; install the CLI on the coordinator")?;
    ensure!(
        output.status.success(),
        "Cannot inspect coordinator mounts; configure execution.docker_mount_source"
    );
    mapped_mount(
        directory,
        &serde_json::from_slice::<Vec<Value>>(&output.stdout)?,
    )
}

/// Match worker ownership to the coordinator's actual effective filesystem identity.
fn identity() -> (u32, u32) {
    // These POSIX calls read process metadata and cannot fail or dereference pointers.
    unsafe { (libc::geteuid(), libc::getegid()) }
}

/// Render an indexed Job with no automatic retries or per-voter Kubernetes objects.
pub fn job(settings: &Settings, name: &str, workers: usize, uid: u32, gid: u32) -> Value {
    json!({"apiVersion":"batch/v1","kind":"Job","metadata":{"name":name},"spec":{
        "completionMode":"Indexed","completions":workers,"parallelism":workers,"backoffLimit":0,
        "template":{"spec":{"restartPolicy":"Never","securityContext":{"runAsUser":uid,"runAsGroup":gid,"fsGroup":gid},
            "containers":[{"name":"worker","image":settings.execution.image,
                "command":["/usr/local/bin/step-load-worker","/load","--workers",workers.to_string()],
                "env":[{"name":"JOB_COMPLETION_INDEX","valueFrom":{"fieldRef":{"fieldPath":"metadata.annotations['batch.kubernetes.io/job-completion-index']"}}},
                    {"name":settings.workload.password_env,"valueFrom":{"secretKeyRef":{"name":name,"key":"password"}}}],
                "resources":settings.execution.resources,"volumeMounts":[{"name":"load","mountPath":"/load"}]}],
            "volumes":[{"name":"load","persistentVolumeClaim":{"claimName":name}}]}}}})
}

/// Send manifest data through stdin; passwords never appear in process arguments.
fn kubectl(settings: &Settings, args: &[&str], manifest: Option<&Value>) -> Result<()> {
    let mut command = Command::new("kubectl");
    command
        .args(["--namespace", &settings.execution.namespace])
        .args(args);
    if let Some(manifest) = manifest {
        let mut child = command.stdin(Stdio::piped()).spawn()?;
        child
            .stdin
            .take()
            .context("kubectl stdin unavailable")?
            .write_all(serde_json::to_string(manifest)?.as_bytes())?;
        ensure!(child.wait()?.success(), "kubectl operation failed");
    } else {
        ensure!(command.status()?.success(), "kubectl operation failed");
    }
    Ok(())
}

/// Preserve resources after failure and collect partial results before returning.
fn kubernetes(directory: &Path, settings: &Settings, workers: usize) -> Result<()> {
    ensure!(
        !settings.execution.storage_class.is_empty(),
        "execution.storage_class must support ReadWriteMany"
    );
    let name = format!(
        "voting-load-{}",
        &hex::encode(Sha256::digest(directory.as_os_str().as_encoded_bytes()))[..12]
    );
    let inputs = directory.join("inputs");
    let input: Input = files::read(&inputs.join("config.json"))?;
    let (uid, gid) = identity();
    let secret = json!({"apiVersion":"v1","kind":"Secret","metadata":{"name":name},"stringData":{"password":input.password()?}});
    let volume = json!({"apiVersion":"v1","kind":"PersistentVolumeClaim","metadata":{"name":name},"spec":{"accessModes":["ReadWriteMany"],"storageClassName":settings.execution.storage_class,"resources":{"requests":{"storage":settings.execution.storage_size}}}});
    let transfer = json!({"apiVersion":"v1","kind":"Pod","metadata":{"name":name},"spec":{"securityContext":{"runAsUser":uid,"runAsGroup":gid,"fsGroup":gid},
        "containers":[{"name":"transfer","image":settings.execution.image,"command":["sleep","infinity"],"volumeMounts":[{"name":"load","mountPath":"/load"}]}],
        "volumes":[{"name":"load","persistentVolumeClaim":{"claimName":name}}]}});
    println!(
        "Run resources: {}/{name}; retained until explicitly cleaned up",
        settings.execution.namespace
    );
    for resource in [&secret, &volume, &transfer] {
        kubectl(settings, &["create", "-f", "-"], Some(resource))?;
    }
    let result = (|| -> Result<()> {
        kubectl(
            settings,
            &[
                "wait",
                &format!("pod/{name}"),
                "--for=condition=Ready",
                "--timeout",
                &settings.execution.wait_timeout,
            ],
            None,
        )?;
        kubectl(
            settings,
            &[
                "cp",
                &format!("{}/.", inputs.display()),
                &format!("{name}:/load"),
            ],
            None,
        )?;
        let job = job(settings, &name, workers, uid, gid);
        files::save(&directory.join("job.json"), &job)?;
        kubectl(settings, &["create", "-f", "-"], Some(&job))?;
        let wait = kubectl(
            settings,
            &[
                "wait",
                &format!("job/{name}"),
                "--for=condition=Complete",
                "--timeout",
                &settings.execution.wait_timeout,
            ],
            None,
        );
        let collect = kubectl(
            settings,
            &[
                "cp",
                &format!("{name}:/load/."),
                inputs.to_str().context("Run path must be UTF-8")?,
            ],
            None,
        );
        collect?;
        wait
    })();
    // The PVC retains diagnostics; an idle transfer pod retains no extra evidence.
    let cleanup = kubectl(
        settings,
        &["delete", "pod", &name, "--ignore-not-found"],
        None,
    );
    result?;
    cleanup
}

/// Copy aggregate artifacts to the run root even when goals or workers failed.
pub fn report(directory: &Path, dsn: Option<&str>) -> Result<()> {
    let result = report::generate(&directory.join("inputs"), dsn);
    for name in [
        "report.html",
        "performance.svg",
        "performance.md",
        "results.json",
    ] {
        let source = directory.join("inputs").join(name);
        if source.exists() {
            fs::copy(source, directory.join(name))?;
        }
    }
    println!("Report: {}", directory.join("report.html").display());
    result
}

/// Start all workers, join them, and publish a report regardless of the outcome.
pub fn run(
    directory: &Path,
    settings: &Settings,
    workers: usize,
    executor: &str,
    assets: &Path,
) -> Result<()> {
    ensure!(workers > 0, "workers must be positive");
    let directory = directory.canonicalize()?;
    let inputs = directory.join("inputs");
    // A run-wide claim also prevents competing Kubernetes Jobs from changing ownership.
    files::create(&directory.join("attempted"))?.sync_all()?;
    files::save(
        &directory.join("execution.json"),
        &json!({"workers":workers,"executor":executor}),
    )?;
    let result = (|| match executor {
        "local" => coordinator::parallel(workers, |index| {
            worker::node(&inputs, index, workers, assets)
        }),
        "docker" => {
            let mount = docker_mount(&inputs, settings)?;
            let (uid, gid) = identity();
            coordinator::parallel(workers, |index| {
                ensure!(
                    Command::new("docker")
                        .args([
                            "run",
                            "--rm",
                            "--user",
                            &format!("{uid}:{gid}"),
                            "--network",
                            &settings.execution.network,
                            "-e",
                            &settings.workload.password_env,
                            "-v",
                            &format!("{}:/load", mount.display()),
                            &settings.execution.image,
                            "/load",
                            "--workers",
                            &workers.to_string(),
                            "--index",
                            &index.to_string()
                        ])
                        .status()?
                        .success(),
                    "Container worker {index} failed"
                );
                Ok(())
            })
        }
        "kubernetes" => kubernetes(&directory, settings, workers),
        _ => anyhow::bail!("Unknown executor: {executor}"),
    })();
    let report = report(&directory, None);
    result?;
    report
}
