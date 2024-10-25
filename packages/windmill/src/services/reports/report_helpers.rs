// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//

use handlebars::{
    Context, Handlebars, Helper, HelperDef, RenderContext, RenderError, RenderErrorReason,
    ScopedJson,
};
use serde_json::value::Value;
use serde_json::{json, Map};
use std::collections::HashMap;

// Define the helper struct
#[derive(Clone, Copy)]
pub struct ArrayHelper;

impl HelperDef for ArrayHelper {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _r: &'reg Handlebars<'reg>,
        _ctx: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
    ) -> Result<ScopedJson<'rc>, RenderError> {
        // Optionally, get parameters from the helper
        let whatever = h
            .hash_get("whatever")
            .and_then(|v| v.value().as_str())
            .unwrap_or("default");

        // Build the array (you can use 'whatever' if needed)
        let value = json!([
            {
                "name": "Nombre 1",
            },
            {
                "name": "Nombre 2",
            },
        ]);

        // Return the value wrapped in ScopedJson
        Ok(value.into())
    }
}

pub fn get_helpers() -> HashMap<String, Box<dyn HelperDef + Send + Sync>> {
    let mut helpers: HashMap<String, Box<dyn HelperDef + Send + Sync>> = HashMap::new();
    helpers.insert(
        "array_helper".to_string(),
        Box::new(ArrayHelper) as Box<dyn HelperDef + Send + Sync>,
    );
    helpers
}
