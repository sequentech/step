// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

/// Keeps the rows that convert and logs the ones that do not, so that one
/// unreadable row does not hide the others in a query that spans every tenant.
pub fn skip_unparseable_rows<T>(rows: impl IntoIterator<Item = anyhow::Result<T>>) -> Vec<T> {
    rows.into_iter()
        .filter_map(|row| row.map_err(|err| tracing::error!("{err:#}")).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::skip_unparseable_rows;
    use anyhow::anyhow;

    #[test]
    fn skip_unparseable_rows_keeps_the_rows_that_convert() {
        let rows = vec![
            Ok("first"),
            Err(anyhow!("Error mapping \"FOO\" into an EventProcessor")),
            Ok("third"),
            Err(anyhow!("error deserializing encryption_policy")),
            Ok("last"),
        ];
        assert_eq!(skip_unparseable_rows(rows), ["first", "third", "last"]);
    }

    #[test]
    fn skip_unparseable_rows_with_no_rows() {
        let rows: Vec<anyhow::Result<u8>> = vec![];
        assert!(skip_unparseable_rows(rows).is_empty());
    }
}
