// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

/** Exports use the same server-scoped, bounded pages as forensic review. */
export async function collectForensicExport<T extends {id: string}>(
    fetchPage: (offset: number, limit: number) => Promise<{items: T[]; total: number}>,
    selectedIds?: Array<string | number>
): Promise<T[]> {
    const selected = selectedIds ? new Set(selectedIds.map(String)) : undefined
    const rows: T[] = []
    let offset = 0
    while (true) {
        const page = await fetchPage(offset, 100)
        rows.push(...page.items.filter((row) => !selected || selected.has(row.id)))
        offset += page.items.length
        if (!page.items.length || offset >= page.total) return rows
    }
}
