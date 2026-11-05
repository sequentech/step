// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The copies of a report as one PDF: each copy is rendered on its own, and
//! its pages follow those of the copy before it.

use anyhow::{anyhow, Context, Result};
use lopdf::{dictionary, Dictionary, Document, Object, ObjectId};

/// What a page takes from the page tree nodes above it when it has none of
/// its own.
const INHERITED_PAGE_ATTRIBUTES: [&[u8]; 4] = [b"Resources", b"MediaBox", b"CropBox", b"Rotate"];

/// A page tree deeper than this is a loop.
const MAX_PAGE_TREE_DEPTH: usize = 64;

const TYPE_CATALOG: &[u8] = b"Catalog";
const TYPE_PAGES: &[u8] = b"Pages";

/// One PDF with the pages of each of `documents`, in their order. A single
/// document is returned as it is.
pub fn merge_pdfs(documents: &[Vec<u8>]) -> Result<Vec<u8>> {
    match documents {
        [] => Err(anyhow!("There is no PDF to merge")),
        [only] => Ok(only.clone()),
        several => merge(several),
    }
}

fn inherited_attributes(
    document: &Document,
    page_id: ObjectId,
) -> Result<Vec<(&'static [u8], Object)>> {
    let mut found: Vec<(&'static [u8], Object)> = Vec::new();
    let mut node = document.get_dictionary(page_id)?;
    let mut depth = 0;
    while let Ok(parent) = node.get(b"Parent").and_then(Object::as_reference) {
        depth += 1;
        if depth > MAX_PAGE_TREE_DEPTH {
            return Err(anyhow!("The PDF's page tree does not end"));
        }
        node = document.get_dictionary(parent)?;
        for name in INHERITED_PAGE_ATTRIBUTES {
            if found.iter().all(|(known, _)| *known != name) {
                if let Ok(value) = node.get(name) {
                    found.push((name, value.clone()));
                }
            }
        }
    }
    Ok(found)
}

fn merge(documents: &[Vec<u8>]) -> Result<Vec<u8>> {
    let mut merged = Document::with_version("1.5");
    let mut next_id = 1;
    let mut pages: Vec<ObjectId> = Vec::new();
    let mut catalog: Option<Dictionary> = None;

    for (index, bytes) in documents.iter().enumerate() {
        let mut document = Document::load_mem(bytes)
            .with_context(|| format!("Copy {} is not a PDF that can be read", index + 1))?;
        document.renumber_objects_with(next_id);
        next_id = document.max_id + 1;

        let page_ids: Vec<ObjectId> = document.get_pages().into_values().collect();
        for page_id in &page_ids {
            let inherited = inherited_attributes(&document, *page_id)?;
            let page = document.get_object_mut(*page_id)?.as_dict_mut()?;
            for (name, value) in inherited {
                if !page.has(name) {
                    page.set(name, value);
                }
            }
            // The structure tree is not merged: a page's place in it is
            // dropped with it.
            page.remove(b"StructParents");
        }
        pages.extend(page_ids);

        if catalog.is_none() {
            catalog = Some(document.catalog()?.clone());
        }
        for (id, object) in std::mem::take(&mut document.objects) {
            let kind = object.type_name().unwrap_or_default();
            if kind != TYPE_CATALOG && kind != TYPE_PAGES {
                merged.objects.insert(id, object);
            }
        }
    }

    let root_id: ObjectId = (next_id, 0);
    for page_id in &pages {
        merged
            .get_object_mut(*page_id)?
            .as_dict_mut()?
            .set("Parent", Object::Reference(root_id));
    }
    merged.objects.insert(
        root_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Count" => i64::try_from(pages.len())?,
            "Kids" => pages.iter().map(|id| Object::Reference(*id)).collect::<Vec<Object>>(),
        }),
    );

    let mut catalog = catalog.ok_or_else(|| anyhow!("The copies have no catalog"))?;
    catalog.set("Pages", Object::Reference(root_id));
    for dropped in [&b"Outlines"[..], b"StructTreeRoot", b"MarkInfo"] {
        catalog.remove(dropped);
    }
    let catalog_id: ObjectId = (next_id + 1, 0);
    merged
        .objects
        .insert(catalog_id, Object::Dictionary(catalog));
    merged.trailer.set("Root", Object::Reference(catalog_id));
    merged.max_id = next_id + 1;
    merged.prune_objects();
    merged.renumber_objects();

    let mut bytes = Vec::new();
    merged
        .save_to(&mut bytes)
        .context("Error writing the copies as one PDF")?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    use lopdf::content::{Content, Operation};
    use lopdf::Stream;

    /// A PDF whose pages each say one of `texts`, with the page size set on
    /// the page tree's node, as a renderer may leave it.
    fn pdf(texts: &[&str]) -> Vec<u8> {
        let mut document = Document::with_version("1.5");
        let pages_id = document.new_object_id();
        let font_id = document.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });
        let kids: Vec<Object> = texts
            .iter()
            .map(|text| {
                let content = Content {
                    operations: vec![
                        Operation::new("BT", vec![]),
                        Operation::new("Tf", vec!["F1".into(), 12.into()]),
                        Operation::new("Td", vec![72.into(), 720.into()]),
                        Operation::new("Tj", vec![Object::string_literal(*text)]),
                        Operation::new("ET", vec![]),
                    ],
                };
                let content_id = document.add_object(Stream::new(
                    dictionary! {},
                    content.encode().expect("the page's content"),
                ));
                Object::Reference(document.add_object(dictionary! {
                    "Type" => "Page",
                    "Parent" => Object::Reference(pages_id),
                    "Contents" => Object::Reference(content_id),
                    "StructParents" => 0,
                }))
            })
            .collect();
        document.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Count" => texts.len() as i64,
                "Kids" => kids,
                "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
                "Resources" => dictionary! {
                    "Font" => dictionary! { "F1" => Object::Reference(font_id) },
                },
            }),
        );
        let catalog_id = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => Object::Reference(pages_id),
            "MarkInfo" => dictionary! { "Marked" => true },
        });
        document.trailer.set("Root", Object::Reference(catalog_id));
        let mut bytes = Vec::new();
        document.save_to(&mut bytes).expect("a PDF");
        bytes
    }

    fn page_texts(bytes: &[u8]) -> Vec<String> {
        let document = Document::load_mem(bytes).expect("the merged PDF");
        document
            .get_pages()
            .keys()
            .map(|number| {
                document
                    .extract_text(&[*number])
                    .expect("the page's text")
                    .trim()
                    .to_string()
            })
            .collect()
    }

    #[test]
    fn the_copies_follow_one_another_in_one_pdf() {
        let merged = merge_pdfs(&[
            pdf(&["Copy 1 of 3, page 1", "Copy 1 of 3, page 2"]),
            pdf(&["Copy 2 of 3, page 1", "Copy 2 of 3, page 2"]),
            pdf(&["Copy 3 of 3, page 1", "Copy 3 of 3, page 2"]),
        ])
        .unwrap();
        assert_eq!(
            page_texts(&merged),
            vec![
                "Copy 1 of 3, page 1",
                "Copy 1 of 3, page 2",
                "Copy 2 of 3, page 1",
                "Copy 2 of 3, page 2",
                "Copy 3 of 3, page 1",
                "Copy 3 of 3, page 2",
            ]
        );
    }

    #[test]
    fn each_page_keeps_what_it_took_from_its_page_tree() {
        let merged = merge_pdfs(&[pdf(&["first"]), pdf(&["second"])]).unwrap();
        let document = Document::load_mem(&merged).unwrap();
        let catalog = document.catalog().unwrap();
        assert!(!catalog.has(b"MarkInfo"));
        for page_id in document.get_pages().into_values() {
            let page = document.get_dictionary(page_id).unwrap();
            assert!(page.has(b"MediaBox"));
            assert!(page.has(b"Resources"));
            assert!(!page.has(b"StructParents"));
        }
    }

    #[test]
    fn one_copy_is_the_report_as_rendered() {
        let only = pdf(&["the report"]);
        assert_eq!(merge_pdfs(&[only.clone()]).unwrap(), only);
    }

    #[test]
    fn nothing_and_what_is_not_a_pdf_are_refused() {
        assert!(merge_pdfs(&[]).is_err());
        let refused = merge_pdfs(&[pdf(&["a"]), b"not a pdf".to_vec()]).unwrap_err();
        assert!(refused.to_string().contains("Copy 2"), "{refused}");
    }

    #[test]
    fn a_page_tree_that_loops_is_refused() {
        let mut document = Document::load_mem(&pdf(&["a"])).unwrap();
        let pages_id = document
            .catalog()
            .unwrap()
            .get(b"Pages")
            .and_then(Object::as_reference)
            .unwrap();
        document
            .get_object_mut(pages_id)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Parent", Object::Reference(pages_id));
        let mut looping = Vec::new();
        document.save_to(&mut looping).unwrap();
        let refused = merge_pdfs(&[looping, pdf(&["b"])]).unwrap_err();
        assert!(refused.to_string().contains("does not end"), "{refused}");
    }

    #[test]
    fn the_renderers_own_pdfs_merge() {
        let rendered = BASE64
            .decode(include_str!("../fixtures/chromium-generated-without-id.pdf.b64").trim())
            .unwrap();
        let pages = Document::load_mem(&rendered).unwrap().get_pages().len();
        let merged = merge_pdfs(&[rendered.clone(), rendered.clone(), rendered]).unwrap();
        let document = Document::load_mem(&merged).unwrap();
        assert!(pages > 0);
        assert_eq!(document.get_pages().len(), pages * 3);
        for page_id in document.get_pages().into_values() {
            assert!(!document.get_page_content(page_id).unwrap().is_empty());
        }
    }
}
