// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/// Bound user-supplied pagination to the returned list without adding offset and limit.
pub(super) fn page<T>(
    items: &[T],
    offset: Option<usize>,
    limit: Option<usize>,
) -> &[T] {
    let remaining = &items[offset.unwrap_or(0).min(items.len())..];
    &remaining[..limit.unwrap_or(remaining.len()).min(remaining.len())]
}

#[cfg(test)]
mod tests {
    use super::page;

    #[test]
    fn ordinary_pages_preserve_order_and_unlimited_tail() {
        let items = [10, 20, 30, 40];
        assert_eq!(page(&items, None, None), &items);
        assert_eq!(page(&items, Some(1), Some(2)), &[20, 30]);
        assert_eq!(page(&items, Some(2), None), &[30, 40]);
        assert!(page(&items, None, Some(0)).is_empty());
    }

    #[test]
    fn offsets_at_or_beyond_the_end_return_an_empty_page() {
        let items = [10, 20];
        assert!(page(&items, Some(2), None).is_empty());
        assert!(page(&items, Some(3), Some(1)).is_empty());
        assert!(page(&items, Some(usize::MAX), Some(usize::MAX)).is_empty());
        assert!(page::<i32>(&[], Some(usize::MAX), Some(usize::MAX)).is_empty());
    }

    #[test]
    fn large_limits_return_the_remaining_items_without_overflow() {
        let items = [10, 20, 30];
        assert_eq!(page(&items, Some(1), Some(usize::MAX)), &[20, 30]);
        assert_eq!(page(&items, None, Some(usize::MAX)), &items);
    }
}
