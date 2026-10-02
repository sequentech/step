//! Integration tests for trellis core functionality
//!
//! Tests the complete workflow including database operations, merkle tree building,
//! proof generation and verification. Uses `serial_test` to ensure tests run
//! sequentially since they share database state.
//!
//! Run with: `cargo test --test basic -- --include-ignored --nocapture`
#![allow(clippy::pedantic)]
#![allow(clippy::print_stdout)]
#![allow(clippy::print_stderr)]
#![allow(clippy::arithmetic_side_effects)]

use anyhow::Result;
use base64::Engine;
use serial_test::serial;

mod test_client;
use test_client::TestClient;
use tokio::time::sleep;

async fn setup_client() -> Result<TestClient> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    let client = TestClient::new("http://localhost:3000").await?;
    // Idempotent setup - safe to call multiple times
    client.setup_test_environment().await?;
    Ok(client)
}

fn hash(input: &str) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hasher.finalize().to_vec()
}

#[ignore]
#[serial]
#[tokio::test]
async fn test_inclusion_proofs() -> Result<()> {
    let client = setup_client().await?;

    // Get initial log size (may not be 0 if tests ran before)
    let initial_size = client.client.get_log_size("test_log_single_source").await?;
    println!("📊 Initial log size: {initial_size}");

    // Add several entries
    let entries = vec!["data1", "data2", "data3", "data4", "data5"];
    let mut hashes = Vec::new();

    // Add entries and collect their hashes
    for entry in &entries {
        let hash = hash(entry);
        hashes.push(hash);

        client.add_entry(entry).await?;
    }

    // Wait until all entries are processed
    let expected_size = initial_size + entries.len() as u64;
    println!("⏳ Waiting for log to reach size {expected_size}...");
    match client
        .wait_until_log_size("test_log_single_source", expected_size)
        .await
    {
        Ok(_) => {}
        Err(e) => {
            // Show diagnostic info on failure
            client.show_recent_entries(20).await?;
            return Err(e);
        }
    }

    // Get the current root
    let root = client.client.get_root("test_log_single_source").await?.root;

    // Verify inclusion proofs for all entries using their hashes
    for (entry, hash) in entries.iter().zip(hashes.iter()) {
        let proof = client
            .client
            .get_inclusion_proof("test_log_single_source", hash)
            .await?;

        // Verify proof has correct root
        assert_eq!(
            proof.root, root,
            "Proof root should match current tree root for entry '{entry}'"
        );

        // Verify the proof itself
        assert!(
            proof.verify(hash).is_ok(),
            "Proof verification should succeed for entry '{entry}'"
        );

        println!(
            "✅ Verified inclusion proof for '{}' at index {}",
            entry, proof.index
        );
    }

    Ok(())
}

#[ignore]
#[serial]
#[tokio::test]
async fn test_has_leaf_and_has_root() -> Result<()> {
    let client = setup_client().await?;

    println!("\n🧪 Testing has_leaf and has_root endpoints...");

    // Get initial state
    let initial_size = client.client.get_log_size("test_log_single_source").await?;
    let initial_root = if initial_size > 0 {
        Some(client.client.get_root("test_log_single_source").await?.root)
    } else {
        None
    };

    // Add test entries
    println!("➕ Adding test entries...");
    client.add_entry("has_test_1").await?;
    client.add_entry("has_test_2").await?;

    // Wait for processing
    let expected_size = initial_size + 2;
    client
        .wait_until_log_size("test_log_single_source", expected_size)
        .await?;

    // Test has_leaf - should find our entries
    println!("\n🔍 Testing has_leaf...");
    let has_leaf1 = client
        .client
        .has_leaf("test_log_single_source", "has_test_1")
        .await?;
    assert!(has_leaf1, "has_test_1 should exist in the tree");
    println!("   ✓ has_test_1 exists: {has_leaf1}");

    let has_leaf2 = client
        .client
        .has_leaf("test_log_single_source", "has_test_2")
        .await?;
    assert!(has_leaf2, "has_test_2 should exist in the tree");
    println!("   ✓ has_test_2 exists: {has_leaf2}");

    // Test has_leaf - should not find non-existent entry
    let has_nonexistent = client
        .client
        .has_leaf("test_log_single_source", "nonexistent_entry")
        .await?;
    assert!(
        !has_nonexistent,
        "nonexistent_entry should not exist in the tree"
    );
    println!("   ✓ nonexistent_entry exists: {has_nonexistent}");

    // Test has_root - check if initial root exists (if we had one)
    println!("\n🔍 Testing has_root...");
    if let Some(old_root) = initial_root {
        let has_old_root = client
            .client
            .has_root("test_log_single_source", old_root.clone())
            .await?;
        assert!(has_old_root, "Old root should exist in root history");
        println!("   ✓ Old root exists: {has_old_root}");
    } else {
        println!("   ⏩ Skipping old root check (started with empty tree)");
    }

    // Test has_root - check current root
    let current_root = client.client.get_root("test_log_single_source").await?.root;
    let has_current_root = client
        .client
        .has_root("test_log_single_source", current_root)
        .await?;
    assert!(has_current_root, "Current root should exist");
    println!("   ✓ Current root exists: {has_current_root}");

    // Test has_root - check non-existent root
    let fake_root = vec![9u8; 32];
    let has_fake_root = client
        .client
        .has_root("test_log_single_source", fake_root)
        .await?;
    assert!(!has_fake_root, "Fake root should not exist");
    println!("   ✓ Fake root exists: {has_fake_root}");

    println!("\n✅ has_leaf and has_root tests complete!");

    Ok(())
}

#[ignore]
#[serial]
#[tokio::test]
async fn test_has_log() -> Result<()> {
    let client = setup_client().await?;
    sleep(std::time::Duration::from_secs(1)).await;

    println!("\n🧪 Testing has_log endpoint...");

    // Test 1: Check that a log that exists returns true
    println!("\n🔍 Checking existing log...");
    let exists = client.client.has_log("test_log_single_source").await?;
    assert!(exists, "test_log_single_source should exist");
    println!("   ✓ test_log_single_source exists: {exists}");

    let exists_multi = client.client.has_log("test_log_multi_source").await?;
    assert!(exists_multi, "test_log_multi_source should exist");
    println!("   ✓ test_log_multi_source exists: {exists_multi}");

    // Test 2: Check that a non-existent log returns false
    println!("\n🔍 Checking non-existent log...");
    let nonexistent = client.client.has_log("this_log_does_not_exist").await?;
    assert!(!nonexistent, "this_log_does_not_exist should not exist");
    println!("   ✓ this_log_does_not_exist exists: {nonexistent}");

    // Test 3: Check another non-existent log with different name pattern
    let another_nonexistent = client.client.has_log("fake_log_12345").await?;
    assert!(!another_nonexistent, "fake_log_12345 should not exist");
    println!("   ✓ fake_log_12345 exists: {another_nonexistent}");

    println!("\n✅ has_log tests complete!");

    Ok(())
}

#[ignore]
#[serial]
#[tokio::test]
async fn test_consistency_proofs() -> Result<()> {
    let client = setup_client().await?;
    let mut historical_roots = Vec::new();

    // Track current size for efficient waiting
    let mut current_size = client.client.get_log_size("test_log_single_source").await?;
    println!("📊 Initial log size: {current_size}");

    // Add entries and store intermediate roots
    for i in 0..3 {
        let mut batch_roots = Vec::new();
        for j in 0..3 {
            let entry = format!("batch{}-entry{}", i + 1, j + 1);

            // Add entry and compute its hash
            client.add_entry(&entry).await?;

            // Wait for this single entry to be processed
            current_size += 1;
            println!(
                "⏳ Waiting for entry {}-{} (size {})...",
                i + 1,
                j + 1,
                current_size
            );
            client
                .wait_until_log_size("test_log_single_source", current_size)
                .await?;

            // Store root after each entry
            let root = client.client.get_root("test_log_single_source").await?.root;
            println!(
                "📸 Storing intermediate root after entry {}-{}",
                i + 1,
                j + 1
            );
            batch_roots.push(root);
        }

        // Store the last root from this batch
        historical_roots.push(batch_roots.pop().expect("Should have at least one root"));
        println!("📸 Storing final root for batch {}", i + 1);
    }

    // Add a final entry to change the tree state
    let entry = "last".to_string();
    client.add_entry(&entry).await?;
    current_size += 1;
    client
        .wait_until_log_size("test_log_single_source", current_size)
        .await?;

    // Get final tree state
    let final_root = client.client.get_root("test_log_single_source").await?.root;

    // Verify consistency between each historical root and final state
    for (i, historical_root) in historical_roots.iter().enumerate() {
        println!("🔍 Verifying consistency with batch {}", i + 1);
        let new_root = client
            .client
            .verify_tree_consistency("test_log_single_source", historical_root.clone())
            .await?;
        assert_eq!(
            new_root,
            final_root,
            "Tree should be consistent with historical root from batch {}",
            i + 1
        );
    }

    // Test consistency of root with itself - should return valid empty proof
    println!("🔍 Testing consistency of root with itself (should succeed with empty proof)");
    let same_root_proof = client
        .client
        .get_consistency_proof("test_log_single_source", final_root.clone())
        .await?;
    
    // Proof should be empty (trivially valid)
    assert!(
        same_root_proof.proof_bytes.is_empty(),
        "Same root consistency proof should have empty proof_bytes"
    );
    
    // Proof should verify successfully
    same_root_proof.verify(&final_root)
        .expect("Same root consistency proof should verify");
    println!("   ✓ Same root returns empty proof that verifies");

    Ok(())
}

#[ignore]
#[serial]
#[tokio::test]
async fn test_burst_operations() -> Result<()> {
    use sha2::{Digest, Sha256};

    let client = setup_client().await?;

    // Get initial log size
    let initial_size = client.client.get_log_size("test_log_single_source").await?;
    println!("📊 Initial log size: {initial_size}");

    // Create a batch of entries and calculate their hashes
    let entries: Vec<String> = (1..=10).map(|i| format!("burst-entry-{i}")).collect();

    let mut hashes = Vec::new();

    // Add all entries as quickly as possible
    println!("🌊 Starting burst of {} entries...", entries.len());
    for entry in &entries {
        // Calculate hash before adding
        let mut hasher = Sha256::new();
        hasher.update(entry.as_bytes());
        let hash = hasher.finalize();
        hashes.push(hash.to_vec());

        client.add_entry(entry).await?;
    }

    // Wait until all entries are processed
    let expected_size = initial_size + entries.len() as u64;
    println!("⏳ Waiting for log to reach size {expected_size}...");
    client
        .wait_until_log_size("test_log_single_source", expected_size)
        .await?;

    // Get final root
    let root = client.client.get_root("test_log_single_source").await?.root;
    println!(
        "🌳 Final merkle root after burst: {}",
        base64::engine::general_purpose::STANDARD.encode(&root)
    );

    // Verify all entries are properly included using their hashes
    for (entry, hash) in entries.iter().zip(hashes.iter()) {
        let proof = client
            .client
            .get_inclusion_proof("test_log_single_source", hash)
            .await?;
        assert_eq!(
            root, proof.root,
            "Merkle root mismatch for entry: {entry}"
        );
        assert!(
            proof.verify(hash).is_ok(),
            "Proof verification failed for entry: {entry}"
        );
        println!("✅ Verified proof for '{}' at index {}", entry, proof.index);
    }

    Ok(())
}

#[ignore]
#[serial]
#[tokio::test]
async fn test_large_batch_performance() -> Result<()> {
    use sha2::{Digest, Sha256};

    let client = setup_client().await?;
    let num_entries = 10_000;

    // Get initial log size
    let initial_size = client.client.get_log_size("test_log_single_source").await?;
    println!("📊 Initial log size: {initial_size}");

    println!(
        "🔥 Starting performance test with {num_entries} entries..."
    );

    // Add entries in chunks to avoid overwhelming the system
    let chunk_size = 1000;
    let mut all_hashes = Vec::with_capacity(num_entries);

    for chunk_start in (0..num_entries).step_by(chunk_size) {
        let chunk_end = (chunk_start + chunk_size).min(num_entries);
        println!("📦 Adding entries {chunk_start}-{chunk_end}");

        let chunk_start_time = std::time::Instant::now();
        let mut chunk_hashes = Vec::with_capacity(chunk_size);

        // Add entries and calculate hashes in this chunk
        for i in chunk_start..chunk_end {
            let entry = format!("perf-entry-{i}");

            // Calculate hash
            let mut hasher = Sha256::new();
            hasher.update(entry.as_bytes());
            let hash = hasher.finalize();

            // Add entry to system first
            client.add_entry(&entry).await?;

            // Store entry and hash for later verification
            chunk_hashes.push((entry, hash.to_vec()));
        }

        // Extend our total hash collection
        all_hashes.extend(chunk_hashes);

        let chunk_duration = chunk_start_time.elapsed();
        println!("  ⏱️ Chunk insert time: {chunk_duration:?}");
        println!(
            "  📊 Average insert time: {:?} per entry",
            chunk_duration / chunk_size as u32
        );
    }

    // Wait for final processing and get root
    let expected_size = initial_size + num_entries as u64;
    println!("\n⏳ Waiting for log to reach size {expected_size}...");
    client
        .wait_until_log_size("test_log_single_source", expected_size)
        .await?;

    // Get and verify final root
    let root = client.client.get_root("test_log_single_source").await?.root;

    println!(
        "Final root: {}",
        base64::engine::general_purpose::STANDARD.encode(&root)
    );

    // Verify some sample entries
    println!("\n🔍 Verifying sample entries...");
    let sample_indices = vec![
        0,
        num_entries / 4,
        num_entries / 2,
        (3 * num_entries) / 4,
        num_entries - 1,
    ];

    for idx in sample_indices {
        let (entry, hash) = all_hashes
            .get(idx)
            .expect("sample index should be within bounds");
        let proof = client
            .client
            .get_inclusion_proof("test_log_single_source", hash)
            .await?;
        assert!(
            proof.verify(hash).is_ok(),
            "Proof verification failed for entry: {entry}"
        );
        println!("✅ Verified entry at index {idx}");
    }

    Ok(())
}

#[ignore]
#[serial]
#[tokio::test]
async fn test_source_log_setup() -> Result<()> {
    // Use the multi-source log for this test
    let client = setup_client().await?;

    // Get initial size
    let initial_size = client.client.get_log_size("test_log_multi_source").await?;
    println!("📊 Initial log size: {initial_size}");

    // Add entries to all three sources with interleaved timing
    println!("\n📝 Adding entries to multiple sources...");
    client
        .add_entry_to_source("source_log", "entry-s-1")
        .await?;
    client
        .add_entry_to_source("test_source_a", "entry-a-1")
        .await?;
    client
        .add_entry_to_source("test_source_b", "entry-b-1")
        .await?;
    client
        .add_entry_to_source("source_log", "entry-s-2")
        .await?;
    client
        .add_entry_to_source("test_source_a", "entry-a-2")
        .await?;
    println!("✅ Added 5 entries across 3 source tables");

    // Wait for batch processor to process all entries
    let expected_size = initial_size + 5;
    println!("\n⏳ Waiting for log to reach size {expected_size}...");
    client
        .wait_until_log_size("test_log_multi_source", expected_size)
        .await?;

    // Verify entries were processed into per-log merkle table from all sources
    let rows = client.get_sources("test_log_multi_source").await?;

    assert!(rows.len() >= 5, "Should have processed at least 5 entries");

    // Verify IDs are strictly increasing (monotonic) within the log
    // With per-log tables, IDs are sequential (no gaps from other logs)
    for i in 1..rows.len() {
        let prev_id: i64 = rows.get(i - 1).expect("prev row exists").get(2);
        let curr_id: i64 = rows.get(i).expect("curr row exists").get(2);
        assert!(
            curr_id > prev_id,
            "Log IDs should be strictly increasing: {curr_id} should be > {prev_id}"
        );
    }

    println!("\n✅ Multi-source test complete!");
    println!("   - {} total entries in merkle_log_test_log_multi_source", rows.len());
    println!(
        "   - Entries from {} different sources",
        rows.iter()
            .map(|r| r.get::<_, String>(0))
            .collect::<std::collections::HashSet<_>>()
            .len()
    );
    println!("   - Merkle log IDs are strictly increasing");
    println!("   - Phase 1 validation: SUCCESS! ✨");

    Ok(())
}

#[ignore]
#[serial]
#[tokio::test]
async fn test_universal_ordering() -> Result<()> {
    let client = setup_client().await?;

    println!("\n🧪 Testing universal ordering with mixed timestamp/non-timestamp sources");

    // Get initial log size
    let initial_size = client.client.get_log_size("test_log_multi_source").await?;
    println!("📊 Initial log size: {initial_size}");

    // Insert entries sequentially - timestamped sources will get increasing timestamps automatically
    println!("\n📝 Inserting entries into timestamped sources:");

    let id1 = client.add_entry_to_source("source_log", "entry_t1").await?;
    println!("  ✓ source_log id={id1} (timestamped)");

    let id2 = client
        .add_entry_to_source("test_source_a", "entry_t2")
        .await?;
    println!("  ✓ test_source_a id={id2} (timestamped)");

    let id3 = client
        .add_entry_to_source("test_source_b", "entry_t3")
        .await?;
    println!("  ✓ test_source_b id={id3} (timestamped)");

    println!("\n📝 Inserting entries into non-timestamped source:");

    let id4 = client
        .add_entry_to_source("source_no_timestamp", "entry_no_ts_1")
        .await?;
    println!("  ✓ source_no_timestamp id={id4} (no timestamp)");

    let id5 = client
        .add_entry_to_source("source_no_timestamp", "entry_no_ts_2")
        .await?;
    println!("  ✓ source_no_timestamp id={id5} (no timestamp)");

    // Step 3: Wait for processing
    let expected_size = initial_size + 5;
    println!(
        "\n⏳ Waiting for {expected_size} entries to be processed..."
    );
    client
        .wait_until_log_size("test_log_multi_source", expected_size)
        .await?;

    // Step 4: Get inclusion proofs to determine exact ordering
    println!("\n🔍 Checking entry positions in the tree:");

    let proof1 = client
        .client
        .get_inclusion_proof("test_log_multi_source", &hash("entry_t1"))
        .await?;
    let proof2 = client
        .client
        .get_inclusion_proof("test_log_multi_source", &hash("entry_t2"))
        .await?;
    let proof3 = client
        .client
        .get_inclusion_proof("test_log_multi_source", &hash("entry_t3"))
        .await?;
    let proof4 = client
        .client
        .get_inclusion_proof("test_log_multi_source", &hash("entry_no_ts_1"))
        .await?;
    let proof5 = client
        .client
        .get_inclusion_proof("test_log_multi_source", &hash("entry_no_ts_2"))
        .await?;

    println!(
        "  entry_t1 (timestamped, source_log:{}): index {}",
        id1, proof1.index
    );
    println!(
        "  entry_t2 (timestamped, test_source_a:{}): index {}",
        id2, proof2.index
    );
    println!(
        "  entry_t3 (timestamped, test_source_b:{}): index {}",
        id3, proof3.index
    );
    println!(
        "  entry_no_ts_1 (no timestamp, source_no_timestamp:{}): index {}",
        id4, proof4.index
    );
    println!(
        "  entry_no_ts_2 (no timestamp, source_no_timestamp:{}): index {}",
        id5, proof5.index
    );

    // Step 5: Verify ordering
    println!("\n✅ Verifying ordering constraints:");

    // Constraint 1: All timestamped entries come before non-timestamped
    assert!(
        proof1.index < proof4.index && proof1.index < proof5.index,
        "Timestamped entry (t1) should come before non-timestamped entries"
    );
    assert!(
        proof2.index < proof4.index && proof2.index < proof5.index,
        "Timestamped entry (t2) should come before non-timestamped entries"
    );
    assert!(
        proof3.index < proof4.index && proof3.index < proof5.index,
        "Timestamped entry (t3) should come before non-timestamped entries"
    );
    println!("  ✓ All timestamped entries come before non-timestamped entries");

    // Constraint 2: Timestamped entries are ordered chronologically
    assert!(
        proof1.index < proof2.index,
        "Earlier timestamp (t1) should come before later timestamp (t2)"
    );
    assert!(
        proof2.index < proof3.index,
        "Earlier timestamp (t2) should come before later timestamp (t3)"
    );
    println!("  ✓ Timestamped entries are in chronological order");

    // Constraint 3: Non-timestamped entries are ordered by id (same table)
    assert!(
        proof4.index < proof5.index,
        "Lower id ({id4}) should come before higher id ({id5}) in same table"
    );
    println!("  ✓ Non-timestamped entries maintain id order within same table");

    // Constraint 4: Verify they are consecutive (no gaps from initial_size)
    let expected_indices: Vec<u64> = (initial_size..initial_size + 5).collect();
    let actual_indices = vec![
        proof1.index,
        proof2.index,
        proof3.index,
        proof4.index,
        proof5.index,
    ];
    let mut sorted_indices = actual_indices.clone();
    sorted_indices.sort();

    assert_eq!(
        sorted_indices, expected_indices,
        "Indices should be consecutive starting from {initial_size}"
    );
    println!("  ✓ All indices are consecutive with no gaps");

    println!("\n✅ Universal ordering test complete!");
    println!("   - Chronological ordering: VERIFIED ✨");
    println!("   - Timestamp/non-timestamp separation: VERIFIED ✨");
    println!("   - ID-based ordering for non-timestamp: VERIFIED ✨");

    Ok(())
}

#[ignore]
#[serial]
#[tokio::test]
async fn test_no_timestamp_ordering() -> Result<()> {
    let client = setup_client().await?;

    println!("\n🧪 Testing ordering with ONLY non-timestamped sources (no timestamps at all)");

    // Get initial log size
    let initial_size = client.client.get_log_size("test_log_no_timestamp").await? as u64;
    println!("📊 Initial log size: {initial_size}");

    // Insert entries with specific ids to test ordering
    // We'll insert in a non-sequential order to verify the processor sorts correctly
    println!("\n📝 Inserting entries in non-sequential order:");

    // Insert into source_no_timestamp_b with id that will be higher
    let id1 = client
        .add_entry_to_source("source_no_timestamp_b", "entry_b_1")
        .await?;
    println!("  ✓ source_no_timestamp_b id={id1}");

    // Insert into source_no_timestamp with lower id
    let id2 = client
        .add_entry_to_source("source_no_timestamp", "entry_a_1")
        .await?;
    println!("  ✓ source_no_timestamp id={id2}");

    // Insert another into source_no_timestamp_b
    let id3 = client
        .add_entry_to_source("source_no_timestamp_b", "entry_b_2")
        .await?;
    println!("  ✓ source_no_timestamp_b id={id3}");

    // Insert another into source_no_timestamp
    let id4 = client
        .add_entry_to_source("source_no_timestamp", "entry_a_2")
        .await?;
    println!("  ✓ source_no_timestamp id={id4}");

    // Wait for processing
    let expected_size = initial_size + 4;
    println!(
        "\n⏳ Waiting for {expected_size} entries to be processed..."
    );
    client
        .wait_until_log_size("test_log_no_timestamp", expected_size)
        .await?;

    // Get inclusion proofs to determine exact ordering
    println!("\n🔍 Checking entry positions in the tree:");

    let proof1 = client
        .client
        .get_inclusion_proof("test_log_no_timestamp", &hash("entry_b_1"))
        .await?;
    let proof2 = client
        .client
        .get_inclusion_proof("test_log_no_timestamp", &hash("entry_a_1"))
        .await?;
    let proof3 = client
        .client
        .get_inclusion_proof("test_log_no_timestamp", &hash("entry_b_2"))
        .await?;
    let proof4 = client
        .client
        .get_inclusion_proof("test_log_no_timestamp", &hash("entry_a_2"))
        .await?;

    println!(
        "  entry_b_1 (source_no_timestamp_b:{}): index {}",
        id1, proof1.index
    );
    println!(
        "  entry_a_1 (source_no_timestamp:{}): index {}",
        id2, proof2.index
    );
    println!(
        "  entry_b_2 (source_no_timestamp_b:{}): index {}",
        id3, proof3.index
    );
    println!(
        "  entry_a_2 (source_no_timestamp:{}): index {}",
        id4, proof4.index
    );

    // Verify ordering
    println!("\n✅ Verifying ordering constraints:");

    // Create a list of (id, table_name, proof_index) for verification
    let mut entries = [
        (id1, "source_no_timestamp_b", proof1.index),
        (id2, "source_no_timestamp", proof2.index),
        (id3, "source_no_timestamp_b", proof3.index),
        (id4, "source_no_timestamp", proof4.index),
    ];

    // Sort by (id, table_name) to get expected order
    entries.sort_by(|a, b| match a.0.cmp(&b.0) {
        std::cmp::Ordering::Equal => a.1.cmp(b.1),
        other => other,
    });

    println!("  Expected order by (id, table_name):");
    for (i, (id, table, _)) in entries.iter().enumerate() {
        println!("    Position {}: {}:{}", initial_size + i as u64, table, id);
    }

    // Verify indices match expected order
    for (i, (id, table, actual_index)) in entries.iter().enumerate() {
        let expected_index = initial_size + i as u64;
        assert_eq!(
            *actual_index, expected_index,
            "Entry {table}:{id} should be at index {expected_index} but is at {actual_index}"
        );
    }
    println!("  ✓ All entries are in correct (id, table_name) order");

    // Verify they are consecutive
    let indices = vec![proof1.index, proof2.index, proof3.index, proof4.index];
    let mut sorted_indices = indices.clone();
    sorted_indices.sort();

    let expected_indices: Vec<u64> = (initial_size..initial_size + 4).collect();
    assert_eq!(
        sorted_indices, expected_indices,
        "Indices should be consecutive starting from {initial_size}"
    );
    println!("  ✓ All indices are consecutive with no gaps");

    println!("\n✅ No-timestamp ordering test complete!");
    println!("   - Pure ID-based ordering: VERIFIED ✨");
    println!("   - Table name tertiary sort: VERIFIED ✨");

    Ok(())
}
