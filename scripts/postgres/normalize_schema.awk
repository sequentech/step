# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

# pg_dump writes ordinary column definitions on separate indented lines.
# A DROP/ADD rollback changes their physical order. Compare definitions in
# name order, retaining their types, defaults, nullability and all constraints.
# This is a comparison format, not SQL to restore.
function columns(    i, j, value) {
    for (i = 2; i <= count; i++) {
        value = column[i]
        j = i - 1
        while (j > 0 && column[j] > value) {
            column[j + 1] = column[j]
            j--
        }
        column[j + 1] = value
    }
    for (i = 1; i <= count; i++) print column[i]
    count = 0
}
/^\\(un)?restrict / { next }
/^CREATE (UNLOGGED )?TABLE / { in_table = 1; print; next }
in_table && /^    [^ ]/ && !/^    CONSTRAINT / {
    sub(/,$/, "")
    column[++count] = $0
    next
}
{
    columns()
    if (in_table && (/^    CONSTRAINT / || /^\);/)) in_table = 0
    print
}
END { columns() }
