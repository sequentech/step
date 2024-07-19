use std::env;
use std::path::{Path, PathBuf};
use sequent_core::util::path::{get_folder_name, list_subfolders};
use crate::pipes::do_tally::{ContestResult, OUTPUT_CONTEST_RESULT_FILE};
use crate::pipes::error::{Error, Result};
use crate::utils::parse_file;
use std::{cmp::Ordering, fs};

pub fn ciccp_consolidation(base_path: &str, folder_common: &str) -> Result<ContestResult> {
    let base_path_path = Path::new(base_path);

    let subfolders = list_subfolders(&base_path_path);
    let base_folders: Vec<PathBuf> = subfolders
        .into_iter()
        .filter(|path| {
            let Some(folder_name) = get_folder_name(path) else {
                return false;
            };
            folder_name.starts_with(folder_common)
        })
        .collect();

    let base_files: Vec<PathBuf> = base_folders
        .into_iter()
        .map(|path| path.join(OUTPUT_CONTEST_RESULT_FILE))
        .collect();

    let mut contest_results: Vec<ContestResult> =  base_files
        .into_iter()
        .map(|file_path| -> Result<ContestResult> {
            let contest_results_file = fs::File::open(&file_path)
                .map_err(|e| Error::FileAccess(file_path.clone(), e))?;
            let contest_result: ContestResult = parse_file(contest_results_file)?;
            Ok(contest_result)
        })
        .collect::<Result<Vec<ContestResult>>>()?;

    let first = contest_results.remove(0);

    let aggregate: ContestResult = contest_results
            .iter()
            .fold(first, |acc, x| acc.aggregate(x, true));
    Ok(aggregate)
}
