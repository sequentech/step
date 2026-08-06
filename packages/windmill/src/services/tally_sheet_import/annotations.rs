// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashSet;

use sequent_core::types::tally_sheet_import::TallySheetImportSourceFormat;

use crate::services::ess_xml_converter::EssAnnotationField;

/// The extra, non-canonical `field` values a source format's converter is
/// allowed to emit, carried through the import as unvalidated annotation
/// data rather than as canonical scalars.
///
/// The canonical CSV parser deliberately doesn't know this set: the extra
/// fields belong to whichever converter produced the file, so each source
/// format declares its own here and the parser is handed the result. That
/// keeps `parse_canonical_csv` generic, and means anything outside the
/// declared set — a mistyped canonical field name, most importantly — is
/// reported as `invalid_field` instead of being silently absorbed.
///
/// A canonical CSV source has no converter, so it declares nothing: every
/// field in such a file must be a canonical one.
pub fn allowed_annotation_fields(source_format: &TallySheetImportSourceFormat) -> HashSet<String> {
    match source_format {
        TallySheetImportSourceFormat::CANONICAL_CSV => HashSet::new(),
        TallySheetImportSourceFormat::ESS_ENHANCED_XML => EssAnnotationField::all_names(),
    }
}
