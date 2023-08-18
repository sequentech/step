// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[macro_use]
extern crate lazy_static;

use bulletin_board::service::{
    BulletinBoard, BulletinBoardService, BulletinBoardServiceConfig,
};
use bulletin_board::signature::Signable;
use bulletin_board::{
    util::init_log, AddEntriesRequest, CreateBoardRequest, NewDataEntry,
    Permissions, Role, User, UserRole,
};
use criterion::async_executor::FuturesExecutor;
use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion,
    SamplingMode, Throughput,
};
use rand::{distributions::Standard, thread_rng, Rng};
use std::fs;
use std::sync::Once;
use std::time::Instant;
use strand::rnd::StrandRng;
use strand::signature::{
    StrandSignaturePk as PublicKey, StrandSignatureSk as SecretKey,
};
use tempfile::{Builder, TempDir};
use tonic::Request;
use uuid::Uuid;

static INIT: Once = Once::new();

lazy_static! {
    // Service user secret (and public) key, used for creating boards and
    // also for executing board actions
    static ref SECRET_KEY: SecretKey = {
        let mut generator = StrandRng;
        SecretKey::new(&mut generator)
    };
}

fn public_key() -> String {
    PublicKey::from(&*SECRET_KEY).clone().try_into().unwrap()
}

// Basic board permissions
fn basic_board_permisssions() -> Permissions {
    Permissions {
        users: vec![User {
            name: "admin".into(),
            public_key: public_key(),
            ..Default::default()
        }],
        roles: vec![Role {
            name: "admins".into(),
            permissions: vec!["AddEntries".into()],
            ..Default::default()
        }],
        user_roles: vec![UserRole {
            user_name: "admin".into(),
            role_names: vec!["admins".into()],
        }],
        ..Default::default()
    }
}

fn setup() -> (tempfile::TempDir, BulletinBoardService) {
    INIT.call_once(|| {
        // This can only be called once or it would fail
        init_log().unwrap();
    });

    let storage_dir = Builder::new().prefix("bb-bench").tempdir().unwrap();
    let service_config = BulletinBoardServiceConfig {
        storage_path: storage_dir.path().display().to_string(),
        server_url: "127.0.0.1:1322".to_string(),
        permissions: Permissions {
            users: vec![User {
                name: "admin".into(),
                public_key: public_key(),
                ..Default::default()
            }],
            roles: vec![Role {
                name: "admins".into(),
                permissions: vec!["CreateBoard".into()],
                ..Default::default()
            }],
            user_roles: vec![UserRole {
                user_name: "admin".into(),
                role_names: vec!["admins".into()],
            }],
            ..Default::default()
        },
    };

    let bulletin_board_service =
        BulletinBoardService::new(service_config).unwrap();

    return (storage_dir, bulletin_board_service);
}

/// Create an empty board in which to run a benchmark
async fn create_empty_board() -> (BulletinBoardService, String, TempDir) {
    let (storage_dir, bulletin_board_service) = setup();

    let board_uuid = Uuid::new_v4().to_string();
    let request = Request::new(
        CreateBoardRequest {
            board_uuid: board_uuid.clone(),
            board_name: "test_board".into(),
            permissions: Some(basic_board_permisssions()),
            ..Default::default()
        }
        .sign(&*SECRET_KEY)
        .unwrap(),
    );
    bulletin_board_service.create_board(request).await.unwrap();

    // Note that we also return the temp dir because otherwise it would be
    // deleted automatically when going out of scope
    (bulletin_board_service, board_uuid, storage_dir)
}

/// Benchmarks creating a bulletin board
pub fn bench_create_board(c: &mut Criterion) {
    let mut group = c.benchmark_group("service");
    group.sampling_mode(SamplingMode::Flat);
    group.throughput(Throughput::Elements(1));
    group.bench_function("create_board", |b| {
        b.to_async(FuturesExecutor).iter_custom(|iters| async move {
            // First create an empty storage dir and use it for a fresh new
            // bulletin board service
            let (storage_dir, bulletin_board_service) = setup();

            // Create the requests to be tested in this set of iterations before
            // we start the timer, so that we don't time the allocation of these
            // structs and get a cleaner timing
            let mut requests = Vec::new();
            for i in 0..iters {
                requests.push(Request::new(
                    CreateBoardRequest {
                        board_uuid: Uuid::new_v4().to_string(),
                        board_name: format!("test_board_{i}"),
                        permissions: Some(basic_board_permisssions()),
                        ..Default::default()
                    }
                    .sign(&*SECRET_KEY)
                    .unwrap(),
                ));
            }

            // Finally create all the boards, timing the whole group of calls
            // and returning the elapsed time
            let start = Instant::now();
            for request in requests {
                black_box(
                    bulletin_board_service.create_board(request).await.unwrap(),
                );
            }
            let result = start.elapsed();

            // remove the temporal directory as it's not being removed
            // automatically for whatever reason. Note that this is not part of
            // the timing calculation.
            fs::remove_dir_all(storage_dir).unwrap();

            result
        })
    });
}

/// Benchmarks adding a entries of different sizes to the bulletin board. Does
/// not benchmark the overhead of tokio/tonic server because we call the service
/// directly.
pub fn bench_add_entries(c: &mut Criterion) {
    let mut group = c.benchmark_group("service");
    group.sampling_mode(SamplingMode::Flat);
    group.throughput(Throughput::Elements(1));
    let bench_sizes_pairs: Vec<(usize, usize, String)> = vec![
        (1 * 1024, 1, "1x 1KiB".into()),
        (2 * 1024 * 1, 2, "2x 1KiB".into()),
        (8 * 1024 * 1, 8, "8x 1KiB".into()),
        (1 * 8 * 1024, 1, "1x 8KiB".into()),
        (2 * 8 * 1024, 2, "2x 8KiB".into()),
        (8 * 8 * 1024, 8, "8x 8KiB".into()),
        (1 * 128 * 1024, 1, "1x 128KiB".into()),
        (2 * 128 * 1024, 2, "2x 128KiB".into()),
        (8 * 128 * 1024, 8, "8x 128KiB".into()),
        (1 * 512 * 1024, 1, "1x 512KiB".into()),
        (2 * 512 * 1024, 2, "2x 512KiB".into()),
        (8 * 512 * 1024, 8, "8x 512KiB".into()),
    ];
    for (bench_size, num_entries, bench_size_str) in bench_sizes_pairs {
        group.bench_with_input(
            BenchmarkId::new("add_entry", bench_size_str),
            &bench_size,
            |b, bench_size| {
                b.to_async(FuturesExecutor).iter_custom(|iters| async move {
                    // Create an empty board in which to run the benchmark
                    let (bulletin_board_service, board_uuid, storage_dir) =
                        create_empty_board().await;

                    // Create the requests to be tested in this set of
                    // iterations before we start the timer,
                    // so that we don't time the allocation of these
                    // structs and get a cleaner timing
                    let mut requests = Vec::new();
                    let mut rng = thread_rng();
                    for _ in 0..(iters / num_entries as u64) {
                        requests.push(Request::new(AddEntriesRequest {
                            board_uuid: board_uuid.clone(),
                            entries: (0..num_entries)
                                .map(|_| {
                                    NewDataEntry {
                                        data: (&mut rng)
                                            .sample_iter(Standard)
                                            .take(*bench_size)
                                            .collect(),
                                        ..Default::default()
                                    }
                                    .sign(&*SECRET_KEY)
                                    .unwrap()
                                })
                                .collect::<Vec<NewDataEntry>>(),
                        }));
                    }

                    // Finally add all the entries, timing the whole group of
                    // calls and returning the elapsed time
                    let start = Instant::now();
                    for request in requests {
                        black_box(
                            bulletin_board_service
                                .add_entries(request)
                                .await
                                .unwrap(),
                        );
                    }
                    let result = start.elapsed();

                    // remove the temporal directory as it's not being removed
                    // automatically for whatever reason. Note that this is not
                    // part of the timing calculation.
                    fs::remove_dir_all(storage_dir).unwrap();

                    result
                })
            },
        );
    }
}

criterion_group!(benches, bench_add_entries, bench_create_board);
criterion_main!(benches);
