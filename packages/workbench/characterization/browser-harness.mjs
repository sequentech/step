// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Shared Playwright helpers for the workbench browser tools — the single
// source (mirroring `spec.mjs` on the headless side) for the idioms the
// `*-e2e-pipeline`, `reproduce-verify`, `dom-probe`, and the general
// DOM-validation runners all use: observing booth signals, client-side
// navigation, entering the booth, and the two ways to set contest config.
//
// A booth-flow change (a relabelled button, a new screen, a moved panel
// control) is then fixed once here rather than in every tool.
//
// Every function takes `page` explicitly (no shared closure). Waits are
// condition-based (`waitFor` / `waitForSelector`) rather than fixed sleeps
// where the target has a stable landmark — this is most of the reload-free
// speedup (~675ms/cell vs ~17s/cell; see VALIDATION_LOGIC_DISTILLATION.md
// §5.3). The rare remaining fixed waits are for the one-time snapshot load,
// which has no single settle signal.
//
// Two config methods, because they travel different code paths to the booth
// and each tool needs the one matching its purpose:
//   - `dispatchConfig` writes the PERSISTED `ballotStyles` slice directly
//     (`window.__store.dispatch`). What the `*-e2e-pipeline` runners use;
//     survives `page.goto` reloads.
//   - `setPanelConfig` drives the real Policy-overrides panel, writing the
//     EPHEMERAL overlay applied at booth-open (`applyEligibilitySwap`). The
//     reviewer's path (what `reproduce-verify` must confirm); being
//     ephemeral, it REQUIRES reload-free navigation or it is wiped.

/** Inline warning message keys currently rendered (each WarnBox carries
 *  `data-warn-id`; upstream #2832) — raw keys, no i18n ambiguity. */
export const warnIds = (page) =>
    page.evaluate(() =>
        Array.from(document.querySelectorAll("[data-warn-id]")).map((el) =>
            el.getAttribute("data-warn-id")
        )
    )

/** The Next-dialog kind: "none" | "dismissible" (has a Continue button) |
 *  "blocking" (must fix before proceeding). */
export const dialogKind = (page) =>
    page.evaluate(() => {
        const d = document.querySelector('[role="dialog"]')
        if (!d) return "none"
        const btns = Array.from(d.querySelectorAll("button")).map((b) => b.innerText)
        return btns.some((b) => /continue/i.test(b)) ? "dismissible" : "blocking"
    })

/** Client-side navigation via a react-router <Link>/<NavLink> — no document
 *  load, so ephemeral overrides survive. Present links: Shell nav
 *  (`/wb`, `/pipeline`, `/tally`), inspector rail (contest/voter/ballot-style),
 *  and the booth's own buttons. */
export const clickLink = async (page, href) => {
    await page.locator(`a[href="${href}"]`).first().click()
}

/** The ONE full document load per run: reset + load a bundled snapshot. */
export async function loadSnapshot(page, base, snapshotId) {
    await page.goto(base + "/wb", {waitUntil: "networkidle", timeout: 60000})
    await page.evaluate(() => window.__resetWorkbench && window.__resetWorkbench())
    await page.waitForTimeout(1500)
    await page.goto(base + "/wb/snapshot/" + encodeURIComponent(snapshotId), {
        waitUntil: "networkidle",
        timeout: 60000,
    })
    await page
        .getByRole("button", {name: /load this snapshot|^load$|reload/i})
        .first()
        .click()
        .catch(() => {})
    await page.waitForTimeout(2500)
}

/** Config method A — write the persisted ballot-style slice directly.
 *  `presentation` merges into `contest.presentation`; `bounds` (min_votes /
 *  max_votes) splice onto the contest. Survives reloads; bypasses the panel. */
export async function dispatchConfig(page, electionId, contestId, {presentation = {}, bounds = {}}) {
    await page.evaluate(
        ({electionId, contestId, presentation, bounds}) => {
            const row = window.__store.getState().ballotStyles[electionId]
            const next = JSON.parse(JSON.stringify(row))
            const c = next.ballot_eml.contests.find((x) => x.id === contestId)
            c.presentation = {...(c.presentation ?? {}), ...presentation}
            for (const [k, v] of Object.entries(bounds)) c[k] = v
            window.__store.dispatch({type: "ballotStyles/setBallotStyle", payload: next})
        },
        {electionId, contestId, presentation, bounds}
    )
}

/** Config method B — drive the Policy-overrides panel (the reviewer's path).
 *  `selects` maps a policy label (e.g. "Over-vote policy") to an option value;
 *  `bounds` maps a bound key (e.g. "min_votes") to a value. Navigates to the
 *  contest page client-side. Ephemeral: requires reload-free navigation. */
export async function setPanelConfig(page, contestId, {selects = {}, bounds = {}}) {
    await clickLink(page, `/wb/contest/${contestId}`)
    const firstLabel = Object.keys(selects)[0]
    if (firstLabel) {
        await page.waitForSelector(`select[aria-label="${firstLabel} override"]`, {timeout: 15000})
    }
    for (const [label, value] of Object.entries(selects)) {
        await page.locator(`select[aria-label="${label} override"]`).selectOption(value)
    }
    for (const [key, value] of Object.entries(bounds)) {
        await page.locator(`input[aria-label="${key} override"]`).fill(String(value))
    }
}

/** Enter the booth from the voter page, client-side, waiting on each screen's
 *  own landmark. Stops on the voting screen; the caller waits for its own
 *  candidate before selecting. */
export async function enterBooth(page, voterId) {
    await clickLink(page, `/wb/voter/${voterId}`)
    const castBtn = page.getByRole("button", {name: /cast a ballot in|recast in/i}).first()
    await castBtn.waitFor({timeout: 15000})
    await castBtn.click()
    const startBtn = page.locator(".start-voting-button").first()
    await startBtn.waitFor({timeout: 15000})
    await startBtn.click()
}

/** Clear any residual selection on the voting screen (idempotent). */
export async function clearSelections(page) {
    const clear = page.getByRole("button", {name: /clear/i}).first()
    if (await clear.count().catch(() => 0)) await clear.click().catch(() => {})
}

/** Dismiss any open dialog WITHOUT continuing to review — its MUI backdrop
 *  intercepts pointer events and would block subsequent navigation. */
export async function dismissDialog(page) {
    const dismiss = page.getByRole("button", {name: /cancel|back|review selection/i}).first()
    if (await dismiss.count().catch(() => 0)) await dismiss.click().catch(() => {})
    else await page.keyboard.press("Escape").catch(() => {})
    await page.waitForSelector('[role="dialog"]', {state: "detached", timeout: 5000}).catch(() => {})
}

/** Back to the inspector (Shell "Snapshots" link), client-side, ready for the
 *  next cell (waits for a contest rail link to confirm the rail rendered). */
export async function backToInspector(page) {
    await clickLink(page, "/wb")
    await page.waitForSelector('a[href^="/wb/contest/"]', {timeout: 15000}).catch(() => {})
}

/** Count marker-inclusive selections (`selected === 0`) live on the voting
 *  screen — the reachability signal (did the target state form?). */
export async function selectionCount(page, electionId, contestId) {
    return page.evaluate(
        ({electionId, contestId}) => {
            const sel = window.__store.getState().ballotSelections[electionId] ?? []
            const c = sel.find((x) => x.contest_id === contestId)
            return c ? c.choices.filter((ch) => ch.selected === 0).length : 0
        },
        {electionId, contestId}
    )
}
