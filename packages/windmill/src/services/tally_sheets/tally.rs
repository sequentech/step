// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use sequent_core::types::hasura::core::TallySheet;
use std::collections::HashMap;
use tracing::instrument;

// Returns a Map<(area_id,contest_id), Vec<tally_sheet>>
#[instrument(skip_all)]
pub fn create_tally_sheets_map(
    tally_sheets: &Vec<TallySheet>,
) -> HashMap<(String, String), Vec<TallySheet>> {
    let mut area_contest_tally_sheet_map: HashMap<(String, String), Vec<TallySheet>> =
        HashMap::new();
    for tally_sheet in tally_sheets {
        area_contest_tally_sheet_map
            .entry((tally_sheet.area_id.clone(), tally_sheet.contest_id.clone()))
            .and_modify(|tally_sheets_vec| {
                tally_sheets_vec.push(tally_sheet.clone());
            })
            .or_insert_with(|| vec![tally_sheet.clone()]);
    }
    area_contest_tally_sheet_map
}
