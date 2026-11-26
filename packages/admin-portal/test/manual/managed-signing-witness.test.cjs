// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const assert = require("node:assert/strict")
const fs = require("node:fs")
const os = require("node:os")
const path = require("node:path")
const test = require("node:test")
const {freshOtpStep, makeStageGate, matchesWitnessPanel} = require("./managed-signing-witness.cjs")

function scratch(t) {
    const d = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), "hsm-witness-test-")))
    t.after(() => fs.rmSync(d, {recursive: true, force: true}))
    return d
}

function acknowledge(directory, event, overrides = {}) {
    const {name, nonce, expected} = event
    const effects = expected === 2 ? 1 : 0
    fs.writeFileSync(
        path.join(directory, name + ".verified"),
        JSON.stringify({
            name,
            nonce,
            approvals: expected,
            distinct_officials: expected,
            applied_device_receipts: expected,
            post_effects: effects,
            linked_effects: effects,
            ...overrides,
        }) + "\n",
        {flag: "wx", mode: 0o600}
    )
}

test("a second Gold prompt waits for a distinct, fresh TOTP period", () => {
    assert.deepEqual(freshOtpStep(61000), {step: 2, waitMs: 0})
    assert.deepEqual(freshOtpStep(61000, 2), {step: 3, waitMs: 29100})
    assert.deepEqual(freshOtpStep(90100, 2), {step: 3, waitMs: 0})
    assert.deepEqual(freshOtpStep(89999, 1), {step: 3, waitMs: 101})
    assert.throws(() => freshOtpStep(40000, 3), /moved behind/)
})

test("witness gates preserve separate 0/0/1/1/2/2 observer stages and ownership", async (t) => {
    const d = scratch(t)
    const step = makeStageGate(d, {timeoutMs: 500, pollMs: 1})
    const order = [
        ["before", 0],
        ["first-setup", 0],
        ["first-approved", 1],
        ["second-setup", 1],
        ["second-approved", 2],
        ["after-refresh", 2],
    ]
    await assert.rejects(step("first-approved"), /Unexpected/)
    for (const [name, expected] of order) {
        const waiting = step(name)
        const f = path.join(d, name + ".json")
        const info = JSON.parse(fs.readFileSync(f, "utf8"))
        assert.equal(info.name, name)
        assert.equal(info.expected, expected)
        assert.match(info.nonce, /^[0-9a-f-]{36}$/)
        assert.equal(fs.statSync(f).mode & 0o777, 0o600)
        acknowledge(d, info)
        await waiting
    }
    await assert.rejects(step("after-refresh"), /Unexpected/)
})

test("an already enrolled officer may skip setup but not the first receipt", async (t) => {
    const d = scratch(t)
    const step = makeStageGate(d, {timeoutMs: 500, pollMs: 1})
    async function approve(name, expected) {
        const waiting = step(name)
        const event = JSON.parse(fs.readFileSync(path.join(d, name + ".json"), "utf8"))
        assert.equal(event.expected, expected)
        acknowledge(d, event)
        await waiting
    }
    await approve("before", 0)
    await assert.rejects(step("second-setup"), /Unexpected/)
    await approve("first-approved", 1)
    await approve("second-setup", 1)
})

test("stale, missing or unsafe observer confirmation cannot advance the browser", async (t) => {
    const a = scratch(t)
    const stepA = makeStageGate(a, {timeoutMs: 500, pollMs: 1})
    const waitingA = stepA("before")
    const eventA = JSON.parse(fs.readFileSync(path.join(a, "before.json"), "utf8"))
    acknowledge(a, eventA, {nonce: "previous-run"})
    await assert.rejects(waitingA, /acknowledgement mismatch/)

    const b = scratch(t)
    const stepB = makeStageGate(b, {timeoutMs: 500, pollMs: 1})
    const waitingB = stepB("before")
    fs.symlinkSync(path.join(a, "before.verified"), path.join(b, "before.verified"))
    await assert.rejects(waitingB, (error) => error.code === "ELOOP")

    const c = scratch(t)
    const stepC = makeStageGate(c, {timeoutMs: 6, pollMs: 1})
    await assert.rejects(stepC("before"), /did not verify stage before/)
    assert.throws(() => makeStageGate(c), /fresh, empty/)

    const d = scratch(t)
    fs.chmodSync(d, 0o755)
    assert.throws(() => makeStageGate(d), /owned, private and symlink-free/)
})

test("a nonce echo alone or incorrect observed effects cannot pass a stage", async (t) => {
    const direct = scratch(t)
    const stepA = makeStageGate(direct, {timeoutMs: 500, pollMs: 1})
    const waitingA = stepA("before")
    const eventA = JSON.parse(fs.readFileSync(path.join(direct, "before.json"), "utf8"))
    fs.writeFileSync(path.join(direct, "before.verified"), JSON.stringify({nonce: eventA.nonce}), {
        mode: 0o600,
    })
    await assert.rejects(waitingA, /acknowledgement mismatch/)

    const wrongCount = scratch(t)
    const stepB = makeStageGate(wrongCount, {timeoutMs: 500, pollMs: 1})
    const waitingB = stepB("before")
    const eventB = JSON.parse(fs.readFileSync(path.join(wrongCount, "before.json"), "utf8"))
    acknowledge(wrongCount, eventB, {post_effects: 1})
    await assert.rejects(waitingB, /acknowledgement mismatch/)
})

test("a real review panel must match the exact operator-pinned request", () => {
    const fixture = {
        requestId: "one-request",
        tenantId: "one-tenant",
        eventId: "one-event",
        electionId: "one-post",
        payloadSha256: "a".repeat(64),
    }
    const panel = {
        request: {
            id: fixture.requestId,
            tenant_id: fixture.tenantId,
            election_event_id: fixture.eventId,
            election_id: fixture.electionId,
            payload_sha256: fixture.payloadSha256,
            action: "initialize-voting",
            required: 2,
        },
        operational_signing_mode: "hsm-required",
    }
    assert.equal(matchesWitnessPanel(panel, fixture), true)
    for (const [field, value] of [
        ["payload_sha256", "b".repeat(64)],
        ["id", "another-request"],
        ["tenant_id", "another-tenant"],
        ["election_event_id", "another-event"],
        ["election_id", "another-post"],
        ["required", 1],
        ["action", "open-voting"],
    ]) {
        assert.equal(
            matchesWitnessPanel({...panel, request: {...panel.request, [field]: value}}, fixture),
            false
        )
    }
    assert.equal(matchesWitnessPanel(undefined, fixture), false)
})
