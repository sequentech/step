use csv::{ReaderBuilder, StringRecord, WriterBuilder};
use std::cmp::Ordering;
use tempfile::NamedTempFile;

use anyhow::{anyhow, Context, Result};

/// Helper: Extracts the join key from a CSV record given a slice of column indices.
/// Returns a vector of &str corresponding to the join key columns.
fn get_join_key<'a>(
    record: &'a StringRecord,
    indices: &[usize],
) -> Result<Vec<&'a str>> {
    let mut key = Vec::with_capacity(indices.len());
    for &i in indices {
        key.push(
            record
                .get(i)
                .ok_or_else(|| anyhow!("Join column index {} out of bounds", i))?,
        );
    }
    Ok(key)
}

/// Performs an inner merge join of two CSV files that are assumed to be sorted by
/// their join keys. For each matching join key, the function writes a record containing
/// selected columns from each file into the output file.
///
/// # Arguments
/// * `file1_path` - Path to the first CSV file.
/// * `file2_path` - Path to the second CSV file.
/// * `file1_join_indices` - Slice of column indices in file1 to use as join keys.
/// * `file2_join_indices` - Slice of column indices in file2 to use as join keys.
/// * `output_file` - Path to the output CSV file.
/// * `file1_output_indices` - Slice of column indices from file1 to output.
/// * `file2_output_indices` - Slice of column indices from file2 to output.
///
/// # Returns
/// * `Ok(())` if the join is successful; otherwise, an error.
pub fn merge_join_csv(
    file1: &NamedTempFile,
    file2: &NamedTempFile,
    file1_join_indices: &[usize],
    file2_join_indices: &[usize],
    file1_output_index: usize,
) -> Result<Vec<String>> {
    // Ensure the join key slices have the same length.
    if file1_join_indices.len() != file2_join_indices.len() {
        return Err(anyhow!("Join key indices slices must have the same length"));
    }

    // Initialize the result vector
    let mut result = Vec::new();

    // Assume the CSV files do not have headers.
    let mut rdr1 = ReaderBuilder::new().has_headers(false).from_reader(file1);
    let mut rdr2 = ReaderBuilder::new().has_headers(false).from_reader(file2);

    // Create iterators over CSV records.
    let mut iter1 = rdr1.records();
    let mut iter2 = rdr2.records();

    // Read the first record from each file.
    let mut rec1_opt = iter1.next();
    let mut rec2_opt = iter2.next();

    // Continue while both files still have records.
    while rec1_opt.is_some() && rec2_opt.is_some() {
        // Unwrap the current records.
        let rec1 = rec1_opt.as_ref().and_then(|res| res.as_ref().ok()).expect("Could not unwrap record");
        let rec2 = rec2_opt.as_ref().and_then(|res| res.as_ref().ok()).expect("Could not unwrap record");

        // Extract the join keys.
        let key1 = get_join_key(rec1, file1_join_indices)?;
        let key2 = get_join_key(rec2, file2_join_indices)?;

        // Compare the join keys lexicographically.
        match key1.cmp(&key2) {
            Ordering::Less => {
                // Advance file1.
                rec1_opt = iter1.next();
            }
            Ordering::Greater => {
                // Advance file2.
                rec2_opt = iter2.next();
            }
            Ordering::Equal => {
                let value = rec1.get(file1_output_index).ok_or_else(|| {
                    anyhow!(
                        "Output column index {} out of bounds in file1",
                        file1_output_index
                    )
                })?;
                result.push(value.to_string());

                // Advance both iterators.
                rec1_opt = iter1.next();
                rec2_opt = iter2.next();
            }
        }
    }

    Ok(result)
}

// // --- Example Usage ---
// // This main function shows how you might call the join function.
// fn main() -> Result<(), Box<dyn Error>> {
//     // Define paths to the two CSV files and the output file.
//     let file1_path = Path::new("file1.csv");
//     let file2_path = Path::new("file2.csv");
//     let output_file = Path::new("joined_output.csv");

//     // Example:
//     // - Join on two columns: column 0 and column 2 in file1 and columns 1 and 3 in file2.
//     // - Output columns: from file1, output column 1; from file2, output column 4.
//     let file1_join_indices = &[0, 2];
//     let file2_join_indices = &[1, 3];
//     let file1_output_indices = &[1]; // For example, one column from file1.
//     let file2_output_indices = &[4]; // For example, one column from file2.

//     merge_join_csv(
//         file1_path,
//         file2_path,
//         file1_join_indices,
//         file2_join_indices,
//         output_file,
//         file1_output_indices,
//         file2_output_indices,
//     )?;

//     Ok(())
// }
