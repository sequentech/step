#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
#
# summarize.sh RESULTS_DIR -- render the key results of one benchmark session as
# Markdown on stdout: the machine header, the snapshot medians per (N, W) cell
# for the five targets, the global target's T and V per (N, W, Q) cell with
# each stage's share, and -- when a differential is present -- the before/after
# medians with speedup factors. The guidance benches are left out on purpose:
# they steer implementation, they are not the result.
#
# Reads snapshot rows (9 fields) from snapshot-*.csv and from bench.sh's
# bench-*.txt, tally rows (13 fields) from tally-*.csv and bench-*.txt, and
# differential rows from differential-<base>-vs-<tip>.csv. One session per
# directory. Portable bash + awk (no gawk extensions).
set -euo pipefail
DIR="${1:?usage: summarize.sh RESULTS_DIR}"
[ -d "$DIR" ] || { echo "summarize.sh: not a directory: $DIR" >&2; exit 2; }

snap_rows()  { cat "$DIR"/snapshot-*.csv "$DIR"/bench-*.txt 2>/dev/null | grep -E '^[0-9]+,[0-9]+,' | awk -F, 'NF == 9' || true; }
tally_rows() { cat "$DIR"/tally-*.csv    "$DIR"/bench-*.txt 2>/dev/null | grep -E '^[0-9]+,[0-9]+,' | awk -F, 'NF == 13' || true; }
diff_rows()  { cat "$DIR"/differential-*.csv 2>/dev/null | grep -E '^(base|curr),[0-9]+,' | awk -F, 'NF == 10' || true; }
tdiff_rows() { cat "$DIR"/tally-differential-*.csv 2>/dev/null | grep -E '^(base|curr),[0-9]+,' | awk -F, 'NF == 14' || true; }

# Shared awk: cell formatting and a median over the values collected for a key.
AWK_COMMON='
function fmtn(n) { if (n == 1000) return "10³"; if (n == 10000) return "10⁴"; if (n == 100000) return "10⁵"; if (n == 1000000) return "10⁶"; return n }
function median(k, m,   n, i, j, t, a) {
    n = cnt[k]
    for (i = 1; i <= n; i++) a[i] = v[k, m, i]
    for (i = 2; i <= n; i++) { t = a[i]; j = i - 1; while (j >= 1 && a[j] > t) { a[j + 1] = a[j]; j-- } a[j + 1] = t }
    if (n % 2) return a[(n + 1) / 2]
    return (a[n / 2] + a[n / 2 + 1]) / 2
}
function ms(x) { return sprintf("%d", x + 0.5) }
function secs(x) { return sprintf("%.2f s", x / 1000) }
'

echo "# Benchmark summary — $(basename "$DIR")"
echo
if [ -f "$DIR/machine.txt" ]; then
    echo '```'
    sed -n 's/^# //p' "$DIR/machine.txt"
    echo '```'
    echo
fi
echo "**Targets** (median of the reps in each cell, milliseconds): **prove** / **verify** — shuffle proof generation / verification, each including the generator derivation; **partial** — one trustee's partial decryption; **combine** — verify every trustee's partial and combine to plaintexts; **strip** — the first mix's Naor-Yung verify-and-strip. N = ciphertexts, W = ciphertext width; T = 3 of P = 5 trustees."
echo

# --- snapshot -----------------------------------------------------------------------
rows="$(snap_rows)"
if [ -n "$rows" ]; then
    echo "## Snapshot"
    echo
    printf '%s\n' "$rows" | awk -F, "$AWK_COMMON"'
    {
        k = $1 ":" $2
        if (!(k in seen)) { seen[k] = 1; order[++nk] = k; cnt[k] = 0 }
        n = ++cnt[k]
        for (m = 3; m <= 7; m++) v[k, m, n] = $m + 0
    }
    END {
        print "| N : W | reps | prove (ms) | verify (ms) | partial (ms) | combine (ms) | strip (ms) |"
        print "|---|---|---|---|---|---|---|"
        for (i = 1; i <= nk; i++) {
            k = order[i]; split(k, p, ":")
            printf "| %s : %s | %d |", fmtn(p[1]), p[2], cnt[k]
            for (m = 3; m <= 7; m++) printf " %s |", ms(median(k, m))
            printf "\n"
        }
    }'
    echo
fi

# --- the global target ------------------------------------------------------------------
trows="$(tally_rows)"
if [ -n "$trows" ]; then
    echo "## Tally — the global target"
    echo
    echo "One tally's critical path for a quorum of Q, replayed with real data flow (\`examples/tally.rs\`): **T** = 2·strip + Σ(prove + verify) + slowest partial + combine, each party's concurrent work counted once; **V** = strip + Σ verify + combine is the external verifier's path. Medians; each stage with its share of T."
    echo
    printf '%s\n' "$trows" | awk -F, "$AWK_COMMON"'
    {
        k = $1 ":" $2 ":" $3 ":" $4
        if (!(k in seen)) { seen[k] = 1; order[++nk] = k; cnt[k] = 0 }
        n = ++cnt[k]
        for (m = 5; m <= 13; m++) v[k, m, n] = $m + 0
    }
    END {
        print "| N : W : Q | reps | T | V | strip ×2 | prove ×Q | verify ×Q | partial | combine | ser/deser |"
        print "|---|---|---|---|---|---|---|---|---|---|"
        for (i = 1; i <= nk; i++) {
            k = order[i]; split(k, p, ":")
            t = median(k, 12); strip = median(k, 5) + median(k, 6)
            printf "| %s : %s : %s | %d | **%s** | %s |", fmtn(p[1]), p[2], p[3], cnt[k], secs(t), secs(median(k, 13))
            printf " %s (%d%%) |", secs(strip), 100 * strip / t + 0.5
            for (m = 7; m <= 10; m++) { x = median(k, m); printf " %s (%d%%) |", secs(x), 100 * x / t + 0.5 }
            if (p[4] == "1") { x = median(k, 11); printf " %s (%d%%) |\n", secs(x), 100 * x / t + 0.5 } else printf " off |\n"
        }
    }'
    echo
fi

# --- the global target, before/after ----------------------------------------------------
tdrows="$(tdiff_rows)"
if [ -n "$tdrows" ]; then
    tbase=""; ttip=""
    for f in "$DIR"/tally-differential-*.csv; do
        b="$(basename "$f" .csv)"; b="${b#tally-differential-}"
        tbase="${b%%-vs-*}"; ttip="${b##*-vs-}"
        break
    done
    echo "## Tally before / after — ${tbase:-baseline} → ${ttip:-tip}"
    echo
    echo "Both tally binaries ran interleaved, rep by rep, on the same machine; medians and the speedup factor."
    printf '%s\n' "$tdrows" | awk -F, "$AWK_COMMON"'
    {
        c = $2 ":" $3 ":" $4 ":" $5
        k = c ":" $1
        if (!(c in cseen)) { cseen[c] = 1; corder[++nc] = c }
        if (!(k in seen)) { seen[k] = 1; cnt[k] = 0 }
        n = ++cnt[k]
        for (m = 6; m <= 14; m++) v[k, m, n] = $m + 0
    }
    END {
        for (i = 1; i <= nc; i++) {
            c = corder[i]; split(c, p, ":")
            kb = c ":base"; kc = c ":curr"
            printf "\n### N = %s, W = %s, Q = %s%s (%d reps)\n\n", fmtn(p[1]), p[2], p[3], (p[4] == "1" ? ", with --ser" : ""), cnt[kb]
            print "| | before | after | speedup |"
            print "|---|---|---|---|"
            tb = median(kb, 13); tc = median(kc, 13)
            printf "| **T** | **%s** | **%s** | **%.2f×** |\n", secs(tb), secs(tc), (tc > 0 ? tb / tc : 0)
            vb = median(kb, 14); vc = median(kc, 14)
            printf "| V (external verifier) | %s | %s | %.2f× |\n", secs(vb), secs(vc), (vc > 0 ? vb / vc : 0)
            sb = median(kb, 6) + median(kb, 7); sc = median(kc, 6) + median(kc, 7)
            printf "| strip ×2 | %s | %s | %.2f× |\n", secs(sb), secs(sc), (sc > 0 ? sb / sc : 0)
            names[8] = "prove ×Q"; names[9] = "verify ×Q"; names[10] = "partial"; names[11] = "combine"; names[12] = "ser/deser"
            for (m = 8; m <= 12; m++) {
                if (m == 12 && p[4] != "1") continue
                b = median(kb, m); a = median(kc, m)
                printf "| %s | %s | %s | %.2f× |\n", names[m], secs(b), secs(a), (a > 0 ? b / a : 0)
            }
        }
    }'
    echo
fi

# --- the stage breakdown, from a profile build ----------------------------------------
if ls "$DIR"/profile-*.txt >/dev/null 2>&1; then
    echo "## Stage breakdown (profile build of targets)"
    echo
    echo "Wall-clock per cost category at each stage's outer call sites (\`vsc::utils::profile\`, \`--features profile\`), its share of the stage, and the unattributed remainder. One run per cell; the profile build is separate from the snapshot binaries."
    echo
    echo '```'
    cat "$DIR"/profile-*.txt | grep -E '^## cell|breakdown of|^    [a-z]'
    echo '```'
    echo
fi

# --- differential -------------------------------------------------------------------
drows="$(diff_rows)"
if [ -n "$drows" ]; then
    base=""; tip=""
    for f in "$DIR"/differential-*.csv; do
        b="$(basename "$f" .csv)"; b="${b#differential-}"
        base="${b%%-vs-*}"; tip="${b##*-vs-}"
        break
    done
    echo "## Before / after — ${base:-baseline} → ${tip:-tip}"
    echo
    echo "Both binaries ran interleaved, rep by rep, on the same machine; medians and the speedup factor."
    printf '%s\n' "$drows" | awk -F, "$AWK_COMMON"'
    {
        k = $2 ":" $3 ":" $1
        c = $2 ":" $3
        if (!(c in cseen)) { cseen[c] = 1; corder[++nc] = c }
        if (!(k in seen)) { seen[k] = 1; cnt[k] = 0 }
        n = ++cnt[k]
        for (m = 4; m <= 8; m++) v[k, m, n] = $m + 0
    }
    END {
        names[4] = "prove"; names[5] = "verify"; names[6] = "partial"; names[7] = "combine"; names[8] = "strip"
        for (i = 1; i <= nc; i++) {
            c = corder[i]; split(c, p, ":")
            kb = c ":base"; kc = c ":curr"
            printf "\n### N = %s, W = %s (%d reps)\n\n", fmtn(p[1]), p[2], cnt[kb]
            print "| target | before (ms) | after (ms) | speedup |"
            print "|---|---|---|---|"
            for (m = 4; m <= 8; m++) {
                b = median(kb, m); a = median(kc, m)
                printf "| %s | %s | %s | %.1f× |\n", names[m], ms(b), ms(a), (a > 0 ? b / a : 0)
            }
        }
    }'
    echo
fi

# --- the snapshot and the "after" column are independent runs of the same tip
# binary (standalone vs interleaved with the baseline); their disagreement is the
# machine's run-to-run spread -- the resolution of every number above.
if [ -n "$rows" ] && [ -n "$drows" ]; then
    { printf '%s\n' "$rows" | sed 's/^/snap,/'; printf '%s\n' "$drows"; } | awk -F, "$AWK_COMMON"'
    $1 == "snap" || $1 == "curr" {
        k = $2 ":" $3 ":" $1; c = $2 ":" $3
        if (!(c in cseen)) { cseen[c] = 1; corder[++nc] = c }
        if (!(k in seen)) { seen[k] = 1; cnt[k] = 0 }
        n = ++cnt[k]
        for (m = 4; m <= 8; m++) v[k, m, n] = $m + 0
    }
    END {
        worst = -1
        for (i = 1; i <= nc; i++) {
            c = corder[i]; ks = c ":snap"; kc = c ":curr"
            if (!(ks in seen) || !(kc in seen)) continue
            for (m = 4; m <= 8; m++) {
                s = median(ks, m); a = median(kc, m)
                if (a > 0) { d = (s > a ? s - a : a - s) / a; if (d > worst) worst = d }
            }
        }
        if (worst >= 0)
            printf "_The snapshot and the \"after\" column are independent runs of the same tip binary — standalone vs interleaved with the baseline. Across the cells present in both they agree within **%.1f%%**: the machine'"'"'s run-to-run spread, and the resolution of every number above._\n\n", worst * 100
    }'
fi

[ -n "$rows$trows$drows$tdrows" ] || echo "_No snapshot, tally or differential rows found in $DIR._"
