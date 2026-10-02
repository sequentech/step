//! Example: Consistency Monitoring (CT-style Auditor)
//!
//! This example demonstrates how to continuously monitor a merkle log for consistency.
//! It periodically checks if the log has grown, and when it has, it verifies that the
//! new root is consistent with the previous root using a consistency proof.
//!
//! This is similar to how Certificate Transparency monitors work - they ensure that
//! logs are append-only and detect any attempts to rewrite history.
//!
//! Usage:
//!   cargo run --example monitor -- `<log_name>`
//!
//! Example:
//!   cargo run --example monitor -- `example_post_log`
#![allow(clippy::pedantic)]
#![allow(clippy::print_stdout)]
#![allow(clippy::print_stderr)]
#![allow(clippy::arithmetic_side_effects)]

use anyhow::Result;
use base64::Engine;
use std::time::Duration;
use trellis::{
    service::{Client, client::RootInfo},
    tree::CtMerkleTree,
};

/// A checkpoint always binds its root to its tree size.
type LogState = RootInfo;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    println!("=== Trellis Consistency Monitor ===\n");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        let program_name = args.first().map(String::as_str).unwrap_or("monitor");
        eprintln!("Usage: {program_name} <log_name>");
        eprintln!("\nExample: {program_name} example_post_log");
        std::process::exit(1);
    }

    let log_name = args.get(1).expect("log_name argument required");

    // Configuration
    let server_url =
        std::env::var("TRELLIS_SERVER_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let poll_interval_secs: u64 = std::env::var("MONITOR_INTERVAL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);

    println!("📡 Monitoring log: {log_name}");
    println!("🔗 Server: {server_url}");
    println!("⏱️  Poll interval: {poll_interval_secs}s");
    println!();

    // Create client
    let client = Client::new(&server_url)?;

    // Get initial state
    println!("🔍 Fetching initial state...");
    let mut state = match fetch_log_state(&client, log_name).await {
        Ok(s) => {
            println!("✅ Initial state:");
            print_state(&s);
            println!();
            s
        }
        Err(e) => {
            eprintln!("❌ Failed to fetch initial state: {e}");
            eprintln!("   Make sure the log exists and the server is running.");
            std::process::exit(1);
        }
    };

    println!("👁️  Monitoring for changes... (Ctrl+C to stop)\n");

    let mut check_count = 0;
    loop {
        tokio::time::sleep(Duration::from_secs(poll_interval_secs)).await;
        check_count += 1;

        match fetch_log_state(&client, log_name).await {
            Ok(new_state) => {
                if new_state.tree_size != state.tree_size {
                    println!(
                        "📊 [Check #{}] Size changed: {} → {}",
                        check_count, state.tree_size, new_state.tree_size
                    );

                    let old_size = state.tree_size;
                    match advance_state(&client, log_name, &mut state, new_state).await {
                        Ok(()) => {
                            println!("   ✅ Consistency proof VERIFIED");
                            println!(
                                "   → Log correctly appended {} new entries",
                                state.tree_size - old_size
                            );
                        }
                        Err(e) => {
                            println!("   ❌ CONSISTENCY VERIFICATION FAILED!: {e}");
                        }
                    }

                    print_state(&state);
                    println!();
                } else if new_state.root != state.root {
                    println!(
                        "⚠️  [Check #{check_count}] Root changed but size unchanged! (Possible issue)"
                    );
                    print_state(&new_state);
                    println!();
                    // Keep the trusted checkpoint after a same-size root mismatch.
                } else {
                    // No change - print periodic status
                    println!(
                        "💤 [Check #{}] No changes (size: {})",
                        check_count, state.tree_size
                    );
                }
            }
            Err(e) => {
                println!("⚠️  [Check #{check_count}] Failed to fetch state: {e}");
            }
        }
    }
}

/// Fetch a coherent root/size pair rather than combining two different snapshots.
async fn fetch_log_state(client: &Client, log_name: &str) -> Result<LogState> {
    if client.get_log_size(log_name).await? == 0 {
        return Ok(LogState {
            tree_size: 0,
            root: CtMerkleTree::new().root(),
        });
    }
    client.get_root(log_name).await
}

/// Advance only after verification; failures leave the saved checkpoint untouched.
async fn advance_state(
    client: &Client,
    log_name: &str,
    state: &mut LogState,
    observed: LogState,
) -> Result<()> {
    verify_consistency(client, log_name, state, &observed).await?;
    *state = observed;
    Ok(())
}

/// Verify ordered checkpoints, including any intermediate roots seen during growth.
/// Each proof is anchored to the saved size as well as its root. When requests race
/// with appends, both observations must be ordered prefixes of a common successor.
/// Continuous growth is retried on the next poll after a bounded number of requests.
async fn verify_consistency(
    client: &Client,
    log_name: &str,
    old_state: &LogState,
    new_state: &LogState,
) -> Result<()> {
    let mut old = old_state.clone();
    let mut new = new_state.clone();
    for _ in 0..8 {
        anyhow::ensure!(new.tree_size >= old.tree_size, "Tree size decreased");
        anyhow::ensure!(new.root.len() == 32, "Invalid root length");
        if new.tree_size == old.tree_size {
            anyhow::ensure!(new.root == old.root, "Tree root mismatch with same size");
            return Ok(());
        }
        if old.tree_size == 0 {
            anyhow::ensure!(
                old.root == CtMerkleTree::new().root(),
                "Invalid empty checkpoint"
            );
            return Ok(());
        }
        let successor = client.verify_tree_consistency(log_name, &old).await?;
        anyhow::ensure!(
            successor.tree_size >= new.tree_size,
            "Proof successor precedes the observed checkpoint"
        );
        if successor.tree_size == new.tree_size {
            anyhow::ensure!(successor.root == new.root, "Proof successor root mismatch");
            return Ok(());
        }
        // Their authenticated sizes order old < new < successor. Now authenticate
        // new as a prefix of that successor, retaining all metadata through races.
        old = new;
        new = successor;
    }
    anyhow::bail!("Tree kept growing during verification; retaining trusted checkpoint")
}

/// Prints a formatted representation of log state
fn print_state(state: &LogState) {
    println!("   Size: {}", state.tree_size);
    if !state.root.is_empty() {
        let root_b64 = base64::engine::general_purpose::STANDARD.encode(&state.root);
        println!("   Root: {}...", &root_b64[..min(16, root_b64.len())]);
    } else {
        println!("   Root: (empty)");
    }
}

/// Returns the minimum of two values.
const fn min(a: usize, b: usize) -> usize {
    if a < b { a } else { b }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Json, Router, routing::get};
    use std::{
        collections::VecDeque,
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
    };
    use trellis::{
        ConsistencyProof, LeafHash,
        service::responses::{ApiResponse, ConsistencyProofResponse},
    };

    fn history(count: u8) -> (CtMerkleTree, Vec<LogState>) {
        let mut tree = CtMerkleTree::new();
        let mut states = vec![];
        for leaf in 0..count {
            tree.push(LeafHash::new(vec![leaf]));
            states.push(LogState {
                tree_size: tree.len(),
                root: tree.root(),
            });
        }
        (tree, states)
    }

    fn proof(tree: &CtMerkleTree, old: &LogState, new: &LogState) -> ConsistencyProof {
        ConsistencyProof {
            old_tree_size: old.tree_size,
            new_tree_size: new.tree_size,
            new_root: new.root.clone(),
            proof_bytes: tree
                .prove_consistency_between(&old.root, &new.root)
                .expect("valid test fixture")
                .as_bytes()
                .to_vec(),
        }
    }

    async fn server(
        proofs: Vec<ConsistencyProof>,
    ) -> (Client, tokio::task::JoinHandle<()>, Arc<AtomicUsize>) {
        let pending = Arc::new(Mutex::new(VecDeque::from(proofs)));
        let requests = Arc::new(AtomicUsize::new(0));
        let count = requests.clone();
        let app = Router::new().route(
            "/logs/test/consistency",
            get(move || {
                let pending = pending.clone();
                let count = count.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Json(ApiResponse::success(ConsistencyProofResponse {
                        log_name: "test".into(),
                        proof: pending
                            .lock()
                            .expect("valid test fixture")
                            .pop_front()
                            .expect("unexpected proof request"),
                    }))
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("valid test fixture");
        let client = Client::new(&format!(
            "http://{}",
            listener.local_addr().expect("valid test fixture")
        ))
        .expect("valid test fixture");
        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("valid test fixture");
        });
        (client, task, requests)
    }

    #[tokio::test]
    async fn rollback_via_common_successor_keeps_trusted_checkpoint() {
        let (tree, states) = history(3);
        let b = &states[0];
        let a = &states[1];
        // Both proofs are cryptographically valid: A->A and B->A. The observed
        // root B is a rollback disguised as growth by claiming size three.
        let same = proof(&tree, a, a);
        let earlier = proof(&tree, b, a);
        assert!(same.verify(&a.root, a.tree_size).is_ok());
        assert!(earlier.verify(&b.root, b.tree_size).is_ok());
        let (client, task, requests) = server(vec![same, earlier]).await;
        let mut saved = a.clone();
        let forged = LogState {
            tree_size: 3,
            root: b.root.clone(),
        };
        assert!(
            advance_state(&client, "test", &mut saved, forged)
                .await
                .is_err()
        );
        assert_eq!(saved, *a);
        assert_eq!(requests.load(Ordering::SeqCst), 1);
        task.abort();
    }

    #[tokio::test]
    async fn honest_growth_and_racing_successor_are_verified() {
        let (tree, states) = history(4);
        let (client, task, requests) = server(vec![
            proof(&tree, &states[1], &states[3]),
            proof(&tree, &states[2], &states[3]),
            proof(&tree, &states[2], &states[3]),
        ])
        .await;
        let mut saved = states[1].clone();
        advance_state(&client, "test", &mut saved, states[2].clone())
            .await
            .expect("valid test fixture");
        assert_eq!(saved, states[2]);
        advance_state(&client, "test", &mut saved, states[3].clone())
            .await
            .expect("valid test fixture");
        assert_eq!(saved, states[3]);
        assert_eq!(requests.load(Ordering::SeqCst), 3);
        task.abort();
    }

    #[tokio::test]
    async fn invalid_observations_never_replace_the_saved_checkpoint() {
        let (tree, states) = history(3);
        let mut invalid = proof(&tree, &states[1], &states[2]);
        invalid.proof_bytes = vec![0; 32];
        let (client, task, _) = server(vec![invalid]).await;
        let mut saved = states[1].clone();
        for observation in [
            states[0].clone(),
            LogState {
                tree_size: 2,
                root: states[0].root.clone(),
            },
            states[2].clone(),
        ] {
            assert!(
                advance_state(&client, "test", &mut saved, observation)
                    .await
                    .is_err()
            );
            assert_eq!(saved, states[1]);
        }
        // An unchanged authenticated state is harmless.
        advance_state(&client, "test", &mut saved, states[1].clone())
            .await
            .expect("valid test fixture");
        task.abort();
    }

    #[tokio::test]
    async fn continuous_growth_is_bounded_and_retried_without_state_loss() {
        let (tree, states) = history(11);
        let proofs = (1..9)
            .map(|i| proof(&tree, &states[i], &states[i + 2]))
            .collect();
        let (client, task, requests) = server(proofs).await;
        let mut saved = states[1].clone();
        assert!(
            advance_state(&client, "test", &mut saved, states[2].clone())
                .await
                .is_err()
        );
        assert_eq!(saved, states[1]);
        assert_eq!(requests.load(Ordering::SeqCst), 8);
        task.abort();
    }
}
