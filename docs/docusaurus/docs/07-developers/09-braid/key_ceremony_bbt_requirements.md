---
id: key_ceremony_bbt_requirements
title: BBT Key Ceremony — Requirements
sidebar_label: BBT Key Ceremony Requirements
---

# Keys Ceremony Requirements

## 1. Core Rule

The key ceremony requires 100% trustee presence throughout. If any trustee disconnects after
the ceremony starts, the admin may cancel and restart from scratch. Cancellation action is
unchanged from the current UI (chevron to open status → cancel button). A ceremony can be
cancelled at any point before the tally.

---

## 2. State Machine

### 2.1 Ceremony states

    ceremony.status ∈ { PENDING, STARTED, IN_PROGRESS, SUCCESS, CANCELLED }

| State | Meaning |
|---|---|
| PENDING | Ceremony created, not yet started |
| STARTED | Ceremony started, no step completed yet |
| IN_PROGRESS | Ceremony started, at least one step completed |
| SUCCESS | All steps completed by all trustees |
| CANCELLED | Terminated by admin |

### 2.2 Trustee connection states

    trustee.connection ∈ { CONNECTED, DISCONNECTED }

| Value | UI indicator |
|---|---|
| CONNECTED | Green check |
| DISCONNECTED | Red indicator |
| Step pending | Gray indicator |

---

## 3. Cancellation

3.1 `ceremony.status` transitions to `CANCELLED` only via an explicit admin command.
Trustee disconnection alone does not change `ceremony.status`.

3.2 When any trustee becomes `DISCONNECTED` during `{ STARTED, IN_PROGRESS }`:
- Show red indicator in the trustee's connection column
- Keep `ceremony.status` unchanged
- Block further step progression until all trustees reconnect
- Do not mark any new step as DONE while any trustee is disconnected

3.3 When admin cancels:
- Set `ceremony.status = CANCELLED`
- Stop accepting further step events
- Show cancellation banner
- Preserve completed steps for audit visibility
- Ceremony becomes immutable

---

## 4. Presence Gating

### 4.1 Start gate

Ceremony can transition from `PENDING` to `STARTED` only if all trustees are `CONNECTED`.
If not, the ceremony remains `PENDING` and the admin cannot start (backend rejects the command).

### 4.2 Progress gate

Step events are accepted only when:
- `ceremony.status ∈ { STARTED, IN_PROGRESS }`
- All trustees are `CONNECTED` at event time

If a step event arrives while any trustee is disconnected: reject it (do not cancel the ceremony).
Error: `CEREMONY_BLOCKED_TRUSTEE_DISCONNECTED`.

---

## 5. Step Progression

Steps per trustee, in order: `GENERATED → DOWNLOADED → CHECKED`

Dependency rules:
- `DOWNLOADED` requires `GENERATED` to be done
- `CHECKED` requires `DOWNLOADED` to be done
- Out-of-order events are rejected (not silently corrected)

---

## 6. Ceremony Status Computation

    allConnected = trustees.every(t => t.connection == CONNECTED)
    anyStepDone  = trustees.some(t => any step is DONE)
    allStepsDone = trustees.every(t => all steps are DONE)

    if ceremony.cancelled  → CANCELLED
    else if allStepsDone   → SUCCESS
    else if anyStepDone    → IN_PROGRESS
    else if ceremony.started → STARTED
    else                   → PENDING

Derived flag:

    blocked = (ceremony.status ∈ { STARTED, IN_PROGRESS } && !allConnected)

The `blocked` flag drives UI warnings and prevents step updates without changing `ceremony.status`.

---

## 7. UI Rendering

### 7.1 Connection column

- `CONNECTED` → green check
- `DISCONNECTED` → red indicator (does not imply cancellation; only blocks progress)

### 7.2 Step columns (Generated / Downloaded / Checked)

- Step `DONE` → green check
- Otherwise → gray indicator / hourglass

Step rendering is not conditioned on `ceremony.status` except when `CANCELLED`.

### 7.3 Blocked state (any trustee disconnected mid-ceremony)

- Step icons remain at their current state; no new transitions occur
- Optionally display a "waiting for all trustees to reconnect" notice
- No cancellation banner

### 7.4 CANCELLED state

- Show cancellation banner
- Connection column reflects actual states (some rows may be red)
- Step cells stay at their last known state for audit; no further advancement
- Restart resets all step cells to hourglass and requires all trustees connected

### 7.5 Prohibited logic

Remove any code that:
- Cancels the ceremony on trustee disconnection
- Infers cancellation from connection state
- Couples red indicator rendering with `ceremony.status` changes
