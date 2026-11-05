// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The render cache: one draw per distinct chart, however many viewers ask.

use super::{RenderCache, RenderKey, RenderKeyParts};
use indexmap::IndexMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn parts() -> RenderKeyParts<'static> {
    RenderKeyParts {
        tenant_id: "tenant",
        election_event_id: "event",
        dashboard: ("overview", "1".into()),
        widget: ("turnout", "3".into()),
        theme: ("default", "1".into()),
        settings_revision: "1".into(),
        snapshot_revision: 7,
        election_set_key: "0123456789abcdef",
        scope_key: "event".into(),
        selector_values: IndexMap::new(),
        renderer_version: "dbt-charts-0.8.0".into(),
        locale: "en-US",
        width_bucket: 640,
        color_scheme: "LIGHT",
    }
}

#[test]
fn every_part_of_the_key_tells_charts_apart() {
    let base = parts().key();
    assert_eq!(base, parts().key());
    let mut values = IndexMap::new();
    values.insert("measure".to_string(), "voted_pre".to_string());
    let variants: Vec<RenderKeyParts> = vec![
        RenderKeyParts {
            tenant_id: "other",
            ..parts()
        },
        RenderKeyParts {
            election_event_id: "other",
            ..parts()
        },
        RenderKeyParts {
            dashboard: ("overview", "2".into()),
            ..parts()
        },
        RenderKeyParts {
            widget: ("turnout", "4".into()),
            ..parts()
        },
        RenderKeyParts {
            theme: ("dark", "1".into()),
            ..parts()
        },
        RenderKeyParts {
            settings_revision: "2".into(),
            ..parts()
        },
        RenderKeyParts {
            snapshot_revision: 8,
            ..parts()
        },
        RenderKeyParts {
            election_set_key: "fedcba9876543210",
            ..parts()
        },
        RenderKeyParts {
            scope_key: "post=x".into(),
            ..parts()
        },
        RenderKeyParts {
            selector_values: values,
            ..parts()
        },
        RenderKeyParts {
            renderer_version: "other".into(),
            ..parts()
        },
        RenderKeyParts {
            locale: "fil-PH",
            ..parts()
        },
        RenderKeyParts {
            width_bucket: 680,
            ..parts()
        },
        RenderKeyParts {
            color_scheme: "DARK",
            ..parts()
        },
    ];
    for variant in variants {
        assert_ne!(variant.key(), base);
    }
}

#[test]
fn selector_values_key_the_same_in_any_order() {
    let mut one = IndexMap::new();
    one.insert("a".to_string(), "1".to_string());
    one.insert("b".to_string(), "2".to_string());
    let mut other = IndexMap::new();
    other.insert("b".to_string(), "2".to_string());
    other.insert("a".to_string(), "1".to_string());
    assert_eq!(
        RenderKeyParts {
            selector_values: one,
            ..parts()
        }
        .key(),
        RenderKeyParts {
            selector_values: other,
            ..parts()
        }
        .key()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_identical_renders_draw_once() {
    let cache = Arc::new(RenderCache::new(16));
    let draws = Arc::new(AtomicUsize::new(0));
    let key: RenderKey = parts().key();
    let mut viewers = vec![];
    for _ in 0..100 {
        let cache = cache.clone();
        let draws = draws.clone();
        viewers.push(tokio::spawn(async move {
            cache
                .get_or_render(key, || async move {
                    draws.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    Ok::<_, String>("<svg/>".to_string())
                })
                .await
        }));
    }
    for viewer in viewers {
        assert_eq!(viewer.await.unwrap().unwrap().as_str(), "<svg/>");
    }
    assert_eq!(draws.load(Ordering::SeqCst), 1);
    // Later viewers are served from the cache.
    let later = cache
        .get_or_render(key, || async {
            Err::<String, _>("drawn again".to_string())
        })
        .await;
    assert_eq!(later.unwrap().as_str(), "<svg/>");
}

#[tokio::test]
async fn a_failed_draw_is_not_kept() {
    let cache = RenderCache::new(16);
    let key = parts().key();
    let failed = cache
        .get_or_render(key, || async {
            Err::<String, _>("timeout".to_string())
        })
        .await;
    assert_eq!(failed, Err("timeout".to_string()));
    let retried = cache
        .get_or_render(key, || async { Ok::<_, String>("<svg/>".to_string()) })
        .await;
    assert_eq!(retried.unwrap().as_str(), "<svg/>");
}

#[tokio::test]
async fn the_least_recently_used_chart_is_dropped_first() {
    let cache = RenderCache::new(1);
    let first = parts().key();
    let second = RenderKeyParts {
        snapshot_revision: 8,
        ..parts()
    }
    .key();
    for key in [first, second] {
        cache
            .get_or_render(key, || async {
                Ok::<_, String>(format!("{key:?}"))
            })
            .await
            .unwrap();
    }
    let again = cache
        .get_or_render(first, || async {
            Ok::<_, String>("redrawn".to_string())
        })
        .await;
    assert_eq!(again.unwrap().as_str(), "redrawn");
}
