// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
pub use sequent_template_renderer::helpers::*;
pub use sequent_template_renderer::{
    assets, bundle, platform_csv, prerender, sample_data,
};

#[cfg(test)]
mod extraction_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn production_entry_point_retains_helpers_and_escaping() {
        let variables = json!({"count": 1200, "name": "<voter>"})
            .as_object()
            .unwrap()
            .clone();
        assert_eq!(
            render_template_text("{{format_u64 count}} {{name}}", variables)
                .unwrap(),
            "1,200 &lt;voter&gt;"
        );
    }
    #[test]
    fn exported_localization_runs_through_production_entry_point() {
        let variables = json!({"count": 1200}).as_object().unwrap().clone();
        assert_eq!(
            render_template_text("{{studio_number \"es\" count}}", variables)
                .unwrap(),
            "1.200"
        );
    }
}
