// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const assert = require("node:assert/strict")
const {randomUUID} = require("node:crypto")
const fs = require("node:fs")
const path = require("node:path")
const {setTimeout: sleep} = require("node:timers/promises")

const periodMs = 30000
const minimumFreshMs = 2500
const stages = [
    ["before", 0],
    ["first-setup", 0],
    ["first-approved", 1],
    ["second-setup", 1],
    ["second-approved", 2],
    ["after-refresh", 2],
]

function freshOtpStep(now, lastStep = -1) {
    assert.ok(Number.isSafeInteger(now) && now >= 0)
    assert.ok(Number.isSafeInteger(lastStep) && lastStep >= -1)
    const current = Math.floor(now / periodMs)
    if (current > lastStep && (current + 1) * periodMs - now >= minimumFreshMs)
        return {step: current, waitMs: 0}
    const step = Math.max(current + 1, lastStep + 1)
    const waitMs = step * periodMs - now + 100
    assert.ok(waitMs <= periodMs + 100, "Fixture clock moved behind a previously used OTP step")
    return {step, waitMs}
}

function matchesWitnessPanel(panel, fixture) {
    const request = panel?.request
    return (
        request?.id === fixture.requestId &&
        request.tenant_id === fixture.tenantId &&
        request.election_event_id === fixture.eventId &&
        request.election_id === fixture.electionId &&
        request.action === "initialize-voting" &&
        request.payload_sha256 === fixture.payloadSha256 &&
        request.required === 2 &&
        panel.operational_signing_mode === "hsm-required"
    )
}

function readAcknowledgement(file) {
    let fd
    try {
        fd = fs.openSync(
            file,
            fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW | fs.constants.O_NONBLOCK
        )
    } catch (error) {
        if (error.code === "ENOENT") return undefined
        throw error
    }
    try {
        const stat = fs.fstatSync(fd)
        assert.ok(
            stat.isFile() &&
                stat.uid === process.getuid() &&
                (stat.mode & 0o777) === 0o600 &&
                stat.nlink === 1 &&
                stat.size <= 512,
            "Unsafe fixture observer acknowledgement"
        )
        return fs.readFileSync(fd, "utf8")
    } finally {
        fs.closeSync(fd)
    }
}

function makeStageGate(directory, {timeoutMs = 60000, pollMs = 100} = {}) {
    assert.ok(path.isAbsolute(directory), "The fixture observer directory must be absolute")
    const dir = fs.lstatSync(directory)
    assert.ok(
        dir.isDirectory() &&
            dir.uid === process.getuid() &&
            (dir.mode & 0o777) === 0o700 &&
            fs.realpathSync(directory) === directory,
        "The fixture observer directory must be owned, private and symlink-free"
    )
    assert.equal(
        fs.readdirSync(directory).length,
        0,
        "Use a fresh, empty fixture observer directory"
    )
    let next = 0
    let waiting = false
    return async (name) => {
        const index = stages.findIndex(([stage]) => stage === name)
        // The two setup observations are skipped only for already enrolled officers.
        let allowed = next
        while (allowed < index && [1, 3].includes(allowed)) allowed++
        assert.ok(!waiting && index === allowed && index >= 0, "Unexpected fixture observer stage")
        next = allowed
        waiting = true
        const [, expected] = stages[index]
        const nonce = randomUUID()
        const request = path.join(directory, name + ".json")
        const ack = path.join(directory, name + ".verified")
        const provisional = request + ".tmp"
        fs.writeFileSync(provisional, JSON.stringify({name, expected, nonce}) + "\n", {
            flag: "wx",
            mode: 0o600,
        })
        fs.renameSync(provisional, request)
        const deadline = performance.now() + timeoutMs
        while (performance.now() <= deadline) {
            const confirmation = readAcknowledgement(ack)
            if (confirmation !== undefined) {
                const effects = expected === 2 ? 1 : 0
                const proof = {
                    name,
                    nonce,
                    approvals: expected,
                    distinct_officials: expected,
                    applied_device_receipts: expected,
                    post_effects: effects,
                    linked_effects: effects,
                }
                let observed
                try {
                    observed = JSON.parse(confirmation)
                } catch {
                    throw new Error("Invalid fixture observer acknowledgement")
                }
                assert.ok(
                    observed &&
                        Object.keys(observed).length === Object.keys(proof).length &&
                        Object.entries(proof).every(([key, value]) => observed[key] === value),
                    "Fixture observer acknowledgement mismatch"
                )
                next++
                waiting = false
                return
            }
            await sleep(pollMs)
        }
        throw new Error("Fixture observer did not verify stage " + name)
    }
}

module.exports = {freshOtpStep, makeStageGate, matchesWitnessPanel}
