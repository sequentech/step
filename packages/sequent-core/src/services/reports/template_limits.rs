// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Limits on how deeply a template may recurse.
//!
//! The template parser and the renderer recurse on nested blocks, `else`
//! clauses, subexpressions, literals and partials, and the stack of a worker
//! thread is finite. The limits are therefore checked before a template is
//! parsed or rendered: first on the text, to bound what the parser recurses
//! on, and then on the parsed template, to reject partials that include
//! themselves and to bound how deep partials render.

use handlebars::template::{DecoratorTemplate, Parameter, TemplateElement};
use handlebars::{Handlebars, RenderError, RenderErrorReason, Template};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Deepest nesting of blocks, `else` clauses, partials, subexpressions and
/// literals that a template may have. It keeps the recursion of the parser
/// and of the renderer within the stack of a worker thread.
const MAX_TEMPLATE_DEPTH: usize = 100;
const INLINE_DECORATOR: &str = "inline";
const PARTIAL_BLOCK: &str = "@partial-block";
const TAG_OPEN: &[u8] = b"{{";
const TAG_CLOSE: &[u8] = b"}}";
const RAW_BLOCK_OPEN: &[u8] = b"{{{{";
const RAW_BLOCK_CLOSE: &[u8] = b"}}}}";
const COMMENT_OPEN: &[u8] = b"{{!";
const LONG_COMMENT_DASHES: &[u8] = b"--";
const LONG_COMMENT_CLOSE: &[u8] = b"--}}";
const QUOTED_PARTIAL_ESCAPE: &[u8] = b"\\'";
const ELSE_KEYWORD: &[u8] = b"else";

fn template_depth_error() -> RenderError {
    RenderErrorReason::Other(format!(
        "template nests blocks, partials, subexpressions or literals more \
         than {MAX_TEMPLATE_DEPTH} levels deep"
    ))
    .into()
}

fn check_depth(depth: usize) -> Result<(), RenderError> {
    if depth > MAX_TEMPLATE_DEPTH {
        return Err(template_depth_error());
    }
    Ok(())
}

fn is_grammar_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r')
}

fn is_tag_padding(byte: u8) -> bool {
    byte == b'~' || is_grammar_whitespace(byte)
}

fn skip_while(text: &[u8], mut pos: usize, skip: fn(u8) -> bool) -> usize {
    while text.get(pos).is_some_and(|byte| skip(*byte)) {
        pos += 1;
    }
    pos
}

fn find_bytes(text: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    text.get(from..)?
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|index| from + index)
}

/// Position of the next `delimiter` that the template grammar does not read
/// as part of a backslash escape.
fn find_unescaped(
    text: &[u8],
    mut pos: usize,
    delimiter: &[u8],
) -> Option<usize> {
    while pos < text.len() {
        if text[pos] == b'\\' {
            let run_end = skip_while(text, pos, |byte| byte == b'\\');
            let escapes_tag = run_end - pos == 1;
            pos = run_end;
            if escapes_tag && text[pos..].starts_with(TAG_OPEN) {
                pos += TAG_OPEN.len();
                if text[pos..].starts_with(TAG_OPEN) {
                    pos += TAG_OPEN.len();
                }
            }
        } else if text[pos..].starts_with(delimiter) {
            return Some(pos);
        } else {
            pos += 1;
        }
    }
    None
}

/// Position just past the string literal that starts at `pos`.
fn string_end(text: &[u8], pos: usize) -> Option<usize> {
    let quote = *text.get(pos)?;
    let mut end = pos + 1;
    while let Some(byte) = text.get(end) {
        if *byte == quote {
            return Some(end + 1);
        }
        end += if *byte == b'\\' { 2 } else { 1 };
    }
    None
}

/// Position just past the partial name that starts at `pos`, which may be
/// single-quoted with only the quote itself escaped.
fn partial_name_end(text: &[u8], pos: usize) -> Option<usize> {
    let name = skip_while(text, pos, is_grammar_whitespace);
    if text.get(name) != Some(&b'\'') {
        return Some(name);
    }
    let mut end = name + 1;
    while let Some(byte) = text.get(end) {
        if *byte == b'\'' {
            return Some(end + 1);
        }
        end += if text[end..].starts_with(QUOTED_PARTIAL_ESCAPE) {
            QUOTED_PARTIAL_ESCAPE.len()
        } else {
            1
        };
    }
    None
}

/// Position just past the array or object literal that starts at `pos`, or
/// `None` where the grammar cannot read one.
fn literal_end(
    text: &[u8],
    mut pos: usize,
    depth: usize,
) -> Result<Option<usize>, RenderError> {
    let mut open = Vec::new();
    while let Some(byte) = text.get(pos) {
        match byte {
            b'[' | b'{' => {
                open.push(*byte);
                check_depth(depth + open.len())?;
            }
            b']' | b'}' => {
                let opener = if *byte == b']' { b'[' } else { b'{' };
                if open.pop() != Some(opener) {
                    return Ok(None);
                }
                if open.is_empty() {
                    return Ok(Some(pos + 1));
                }
            }
            b'"' | b'\'' => {
                let Some(end) = string_end(text, pos) else {
                    return Ok(None);
                };
                pos = end;
                continue;
            }
            _ => {}
        }
        pos += 1;
    }
    Ok(None)
}

/// Whether a literal can begin with `byte`, which an array literal needs right
/// after its `[`.
fn can_start_literal(byte: u8) -> bool {
    byte.is_ascii_digit() || b"\"'[]{},-ntf".contains(&byte)
}

/// Position just past the bracket that starts at `pos`. The grammar reads a
/// bracket as an array literal or, failing that, as a path segment that ends
/// at the first `]`. A segment that cannot begin a literal is certainly a path
/// segment. Otherwise only the literal can hold strings and nested brackets,
/// which would end it later, so a bracket that does not close at that first
/// `]` is rejected: the end of its tag could not be told otherwise.
fn bracket_end(
    text: &[u8],
    pos: usize,
    depth: usize,
) -> Result<Option<usize>, RenderError> {
    let Some(close) = find_bytes(text, pos, b"]") else {
        return Ok(None);
    };
    let first = skip_while(text, pos + 1, is_grammar_whitespace);
    if text
        .get(first)
        .is_some_and(|byte| !can_start_literal(*byte))
    {
        return Ok(Some(close + 1));
    }
    match literal_end(&text[..=close], pos, depth)? {
        Some(end) => Ok(Some(end)),
        None => Err(RenderErrorReason::Other(
            "template brackets must be closed by the first ']' that follows \
             them"
                .to_string(),
        )
        .into()),
    }
}

/// Position of the `}}` that closes the tag whose content starts at `pos`,
/// or `None` where the grammar cannot read the tag. `depth` is the nesting of
/// the tag itself.
fn tag_close(
    text: &[u8],
    mut pos: usize,
    depth: usize,
) -> Result<Option<usize>, RenderError> {
    let mut subexpressions = 0;
    while let Some(byte) = text.get(pos) {
        match byte {
            b'}' if subexpressions == 0
                && text[pos..].starts_with(TAG_CLOSE) =>
            {
                return Ok(Some(pos));
            }
            b'"' | b'\'' => {
                let Some(end) = string_end(text, pos) else {
                    return Ok(None);
                };
                pos = end;
                continue;
            }
            b'(' => {
                subexpressions += 1;
                check_depth(depth + subexpressions)?;
            }
            b')' => {
                if subexpressions == 0 {
                    return Ok(None);
                }
                subexpressions -= 1;
            }
            b'[' => {
                let Some(end) = bracket_end(text, pos, depth + subexpressions)?
                else {
                    return Ok(None);
                };
                pos = end;
                continue;
            }
            b'{' => {
                let Some(end) = literal_end(text, pos, depth + subexpressions)?
                else {
                    return Ok(None);
                };
                pos = end;
                continue;
            }
            _ => {}
        }
        pos += 1;
    }
    Ok(None)
}

/// Position just past the raw block that opens at `open`, or `None` where the
/// grammar cannot read one.
fn raw_block_end(
    text: &[u8],
    open: usize,
    depth: usize,
) -> Result<Option<usize>, RenderError> {
    let Some(start_close) =
        tag_close(text, open + RAW_BLOCK_OPEN.len(), depth)?
    else {
        return Ok(None);
    };
    if !text[start_close..].starts_with(RAW_BLOCK_CLOSE) {
        return Ok(None);
    }
    let body = start_close + RAW_BLOCK_CLOSE.len();
    let Some(end_open) = find_unescaped(text, body, RAW_BLOCK_OPEN) else {
        return Ok(None);
    };
    let Some(end_close) =
        tag_close(text, end_open + RAW_BLOCK_OPEN.len(), depth)?
    else {
        return Ok(None);
    };
    Ok(text[end_close..]
        .starts_with(RAW_BLOCK_CLOSE)
        .then_some(end_close + RAW_BLOCK_CLOSE.len()))
}

/// Position just past the comment that opens at `open`. A `{{!--` comment
/// runs to the first `--}}`, and falls back to the first `}}` when there is
/// none, which `long_close_missing` remembers for the comments that follow.
fn comment_end(
    text: &[u8],
    open: usize,
    long_close_missing: &mut bool,
) -> Option<usize> {
    let body = open + COMMENT_OPEN.len();
    let dashes = skip_while(text, body, is_grammar_whitespace);
    if !*long_close_missing && text[dashes..].starts_with(LONG_COMMENT_DASHES) {
        match find_bytes(
            text,
            dashes + LONG_COMMENT_DASHES.len(),
            LONG_COMMENT_CLOSE,
        ) {
            Some(close) => return Some(close + LONG_COMMENT_CLOSE.len()),
            None => *long_close_missing = true,
        }
    }
    find_bytes(text, body, TAG_CLOSE).map(|close| close + TAG_CLOSE.len())
}

/// Whether the tag whose content starts at `marker` is an `else` clause with
/// an expression of its own, such as `else if`. A plain `else` does not nest
/// deeper than the block it belongs to.
fn is_else_chain(text: &[u8], marker: usize) -> bool {
    let keyword = if text.get(marker) == Some(&b'^') {
        1
    } else if text[marker..].starts_with(ELSE_KEYWORD) {
        ELSE_KEYWORD.len()
    } else {
        return false;
    };
    let after = skip_while(text, marker + keyword, is_tag_padding);
    !text[after..].starts_with(TAG_CLOSE)
}

/// Follows how the template grammar nests `template`, without parsing it, and
/// fails when blocks, `else` chains, subexpressions and literals nest deeper
/// than `MAX_TEMPLATE_DEPTH`. Each clause of an `else` chain counts as a level
/// because the parser nests it in the clause before. A closing tag, or an
/// `else` clause that has no block to continue (a helper may be named `else`),
/// leaves the depth as it is. The scan stops where the grammar cannot read on,
/// since parsing fails there.
pub(super) fn check_nesting(template: &str) -> Result<(), RenderError> {
    let text = template.as_bytes();
    let mut depth = 0;
    let mut clauses: Vec<usize> = Vec::new();
    let mut long_comment_close_missing = false;
    let mut pos = 0;
    while let Some(open) = find_unescaped(text, pos, TAG_OPEN) {
        let next = if text[open..].starts_with(RAW_BLOCK_OPEN) {
            raw_block_end(text, open, depth)?
        } else if text[open..].starts_with(COMMENT_OPEN) {
            comment_end(text, open, &mut long_comment_close_missing)
        } else {
            let marker =
                skip_while(text, open + TAG_OPEN.len(), is_tag_padding);
            let content = match text.get(marker) {
                Some(b'#') => {
                    depth += 1;
                    clauses.push(0);
                    check_depth(depth)?;
                    let kind =
                        skip_while(text, marker + 1, is_grammar_whitespace);
                    if text.get(kind) == Some(&b'>') {
                        partial_name_end(text, kind + 1)
                    } else {
                        Some(kind)
                    }
                }
                Some(b'/') => {
                    if let Some(block_clauses) = clauses.pop() {
                        depth = depth.saturating_sub(block_clauses + 1);
                    }
                    partial_name_end(text, marker + 1)
                }
                Some(b'>') => partial_name_end(text, marker + 1),
                Some(b'{') => Some(marker + 1),
                _ if is_else_chain(text, marker) => {
                    if let Some(block_clauses) = clauses.last_mut() {
                        *block_clauses += 1;
                        depth += 1;
                        check_depth(depth)?;
                    }
                    Some(marker)
                }
                _ => Some(marker),
            };
            match content {
                Some(content) => tag_close(text, content, depth)?
                    .map(|close| close + TAG_CLOSE.len()),
                None => None,
            }
        };
        let Some(next) = next else {
            return Ok(());
        };
        pos = next;
    }
    Ok(())
}

fn partial_name(partial: &DecoratorTemplate) -> Result<&str, RenderError> {
    partial.name.as_name().ok_or_else(|| {
        RenderErrorReason::Other(
            "template partial names must be literals".to_string(),
        )
        .into()
    })
}

/// The name an `inline` decorator defines a partial under, or `None` for
/// any other decorator.
fn inline_partial_name(
    decorator: &DecoratorTemplate,
) -> Result<Option<&str>, RenderError> {
    if decorator.name.as_name() != Some(INLINE_DECORATOR) {
        return Ok(None);
    }
    match decorator.params.first() {
        Some(Parameter::Literal(Value::String(name))) => Ok(Some(name)),
        _ => Err(RenderErrorReason::Other(
            "inline partial names must be string literals".to_string(),
        )
        .into()),
    }
}

/// Adds the body of every inline partial and partial block in `template`
/// to `partials`, under the name it renders as.
fn collect_partials<'a>(
    template: &'a Template,
    level: usize,
    partials: &mut HashMap<&'a str, Vec<&'a Template>>,
) -> Result<(), RenderError> {
    check_depth(level)?;
    for element in &template.elements {
        match element {
            TemplateElement::HelperBlock(helper) => {
                for block in helper.template.iter().chain(&helper.inverse) {
                    collect_partials(block, level + 1, partials)?;
                }
            }
            TemplateElement::PartialExpression(partial)
            | TemplateElement::PartialBlock(partial) => {
                if let Some(content) = &partial.template {
                    partials.entry(PARTIAL_BLOCK).or_default().push(content);
                    collect_partials(content, level + 1, partials)?;
                }
            }
            TemplateElement::DecoratorExpression(decorator)
            | TemplateElement::DecoratorBlock(decorator) => {
                if let (Some(body), Some(name)) =
                    (&decorator.template, inline_partial_name(decorator)?)
                {
                    partials.entry(name).or_default().push(body);
                    collect_partials(body, level + 1, partials)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// Follows the partials a template renders, failing on a partial that
/// includes itself, directly or through other partials, and on rendering
/// deeper than `MAX_TEMPLATE_DEPTH`. A partial name stands for every body it
/// could resolve to.
struct PartialDepths<'a, 'p> {
    partials: &'p HashMap<&'a str, Vec<&'a Template>>,
    heights: HashMap<&'a str, usize>,
    expanding: HashSet<&'a str>,
}

impl<'a, 'p> PartialDepths<'a, 'p> {
    fn new(partials: &'p HashMap<&'a str, Vec<&'a Template>>) -> Self {
        PartialDepths {
            partials,
            heights: HashMap::new(),
            expanding: HashSet::new(),
        }
    }

    /// How many levels of blocks and partials `template` renders below
    /// itself, when it is rendered `depth` levels deep.
    fn template_height(
        &mut self,
        template: &'a Template,
        depth: usize,
    ) -> Result<usize, RenderError> {
        check_depth(depth)?;
        let mut height = 0;
        for element in &template.elements {
            let element_height = match element {
                TemplateElement::HelperBlock(helper) => {
                    let mut block_height = 0;
                    for block in helper.template.iter().chain(&helper.inverse) {
                        block_height = block_height
                            .max(self.template_height(block, depth + 1)?);
                    }
                    block_height + 1
                }
                TemplateElement::PartialExpression(partial)
                | TemplateElement::PartialBlock(partial) => {
                    let mut partial_height =
                        self.partial_height(partial_name(partial)?, depth + 1)?;
                    if let Some(content) = &partial.template {
                        partial_height = partial_height
                            .max(self.template_height(content, depth + 1)?);
                    }
                    partial_height + 1
                }
                _ => 0,
            };
            height = height.max(element_height);
        }
        Ok(height)
    }

    /// The height of the partial `name`, which is the tallest of the bodies
    /// it could resolve to.
    fn partial_height(
        &mut self,
        name: &'a str,
        depth: usize,
    ) -> Result<usize, RenderError> {
        let height = match self.heights.get(name) {
            Some(height) => *height,
            None => {
                if !self.expanding.insert(name) {
                    return Err(RenderErrorReason::Other(format!(
                        "template partial {name} includes itself"
                    ))
                    .into());
                }
                let partials = self.partials;
                let mut height = 0;
                for body in partials.get(name).into_iter().flatten() {
                    height = height.max(self.template_height(body, depth)?);
                }
                self.expanding.remove(name);
                self.heights.insert(name, height);
                height
            }
        };
        check_depth(depth + height)?;
        Ok(height)
    }
}

/// Checks a template that is rendered on its own: it must not nest too deeply
/// and none of its inline partials may include themselves.
pub(super) fn check_template(template: &str) -> Result<(), RenderError> {
    check_nesting(template)?;
    let template = Template::compile(template)?;
    let mut partials = HashMap::new();
    collect_partials(&template, 0, &mut partials)?;
    PartialDepths::new(&partials).template_height(&template, 0)?;
    Ok(())
}

/// Checks the templates registered in `registry` that rendering the template
/// `entry` could reach: none of their partials may include themselves.
pub(super) fn check_registered(
    registry: &Handlebars<'_>,
    entry: &str,
) -> Result<(), RenderError> {
    let mut partials: HashMap<&str, Vec<&Template>> = HashMap::new();
    for (name, template) in registry.get_templates() {
        partials.entry(name.as_str()).or_default().push(template);
        collect_partials(template, 0, &mut partials)?;
    }
    PartialDepths::new(&partials).partial_height(entry, 0)?;
    Ok(())
}
