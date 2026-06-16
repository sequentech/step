---
id: key_ceremony_bbt_requirements
title: BBT Key Ceremony — Requirements
sidebar_label: BBT Key Ceremony Requirements
---

# Keys Ceremony Requirements

See [Keys Ceremony State Machine](./key_ceremony_state_machine.md) for the authoritative
`KeysCeremonyExecutionStatus` transition table and implementation, and
[BBT Signing Keypair Proposal](./key_ceremony_bbt_propossal_no_protoccol_change.md) for the
full design these requirements describe the UI/UX side of.

## 1. Core Rule

The key ceremony does **not** require synchronous 100% trustee presence. Trustees register
their keys independently, in any order and at any time, and the ceremony automatically
advances once every selected trustee has a registered key (see §4). A trustee going offline
never blocks or cancels the ceremony by itself (see §3, §6) — the only effect is local to
that trustee's own wizard (§6, §7.3).

The admin may cancel a ceremony at any point before it reaches a terminal state, with one
exception: cancelling a `SUCCESS` ceremony is only allowed if no election in the event has
started its voting period. Cancellation is always an explicit admin command (chevron to open
status → cancel button); it is never inferred from trustee connectivity.

---

## 2. State Machine

### 2.1 Ceremony execution states

```
ceremony.execution_status ∈ { AWAITING_TRUSTEE_KEYS, IN_PROGRESS, SUCCESS, CANCELLED }
```

| State | Meaning | Set by |
|---|---|---|
| `AWAITING_TRUSTEE_KEYS` | Ceremony created; waiting for every selected trustee (BBT and server-based alike) to have a registered per-ceremony `public_key`. The `Configuration` message has not been posted to the board yet. | `create_keys_ceremony` (initial); persists until the key-registration gate in §4 is satisfied |
| `IN_PROGRESS` | Every trustee's key is registered; `Configuration` has been posted to the board; trustees are running DKG and completing the Download/Check steps. | `create_keys_impl` beat task, atomically with posting `Configuration` |
| `SUCCESS` | Every trustee has reached `KEY_CHECKED`. | `set_public_key_impl` beat task |
| `CANCELLED` | Terminated by an explicit admin command. Terminal — no further transitions are permitted. | `/cancel-keys-ceremony` |

`PENDING` and `STARTED` do not exist in the implemented model — `AWAITING_TRUSTEE_KEYS`
absorbs both the "just created" and "waiting to post Configuration" roles. There is no
admin "start" action and no formula that derives `execution_status` from step completion;
it is an explicit, persisted value written only at the call sites in §6.

Transition graph (see [state machine doc](./key_ceremony_state_machine.md) for the enforced
`try_transition` table):

```
AWAITING_TRUSTEE_KEYS ──▶ IN_PROGRESS ──▶ SUCCESS

AWAITING_TRUSTEE_KEYS ─┐
      IN_PROGRESS       ├──▶ CANCELLED   (SUCCESS → CANCELLED only if no election
          SUCCESS       ─┘                in the event has started its voting period)
```

### 2.2 Trustee key-ceremony status

```
trustee.status ∈ { WAITING, KEY_GENERATED, KEY_RETRIEVED, KEY_CHECKED }
```

| Status | Meaning |
|---|---|
| `WAITING` | Initial state for every selected trustee. Also re-applied whenever the trustee has no usable key or no matching board message yet. |
| `KEY_GENERATED` | A `PublicKey`/`PublicKeySigned` board message matching the trustee's registered key was found. |
| `KEY_RETRIEVED` | Trustee self-reported their identity-key backup as downloaded (`confirm_key_backup` for BBT; `/get-private-key` for server-based). |
| `KEY_CHECKED` | Trustee self-reported their backup as locally verified (`confirm_key_check` for BBT; `/check-private-key` for server-based). |

Renamed from an earlier `GENERATED → DOWNLOADED → CHECKED` naming to match the implemented
`TrusteeStatus` enum; `WAITING` is an explicit member, not an implicit default.

### 2.3 Trustee online indicator

```
trustee.heartbeat ∈ { ACTIVE, NOT_ACTIVE }
```

| Value | UI indicator |
|---|---|
| `ACTIVE` | Green check |
| `NOT_ACTIVE` | Red indicator |
| Step pending | Gray indicator |

Driven by the B4 heartbeat: each BBT/native session POSTs a heartbeat every
`BRAID_B4_HEARTBEAT` seconds; two missed beats flips the session to `NOT_ACTIVE`. This is
purely an **observability** signal. It never gates a ceremony-execution-status transition, a
trustee-status transition, or any Harvest endpoint — its only behavioral effect is the
per-trustee wizard-navigation rule in §6/§7.3.

---

## 3. Cancellation

3.1 `ceremony.execution_status` transitions to `CANCELLED` only via the explicit
`/cancel-keys-ceremony` admin command (`cancel_keys_ceremony` Hasura action). A trustee's
online indicator never changes `execution_status` by itself, and is neither necessary nor
sufficient for cancellation.

3.2 When a trustee goes `NOT_ACTIVE` during `{ AWAITING_TRUSTEE_KEYS, IN_PROGRESS }`:
- Show a red/offline indicator in that trustee's row.
- Do **not** block the key-registration gate (§4), reject any Harvest endpoint call, or
  change `execution_status` or any other trustee's status.
- The only behavioral effect is local: that trustee's own wizard disables its "Next" action
  (§6, §7.3). No other trustee or the ceremony as a whole is affected.

3.3 When admin cancels:
- Valid source states: `AWAITING_TRUSTEE_KEYS`, `IN_PROGRESS`, `SUCCESS`. `SUCCESS →
  CANCELLED` additionally requires that no election in the event has started its voting
  period.
- `try_transition` writes `CANCELLED`.
- In the same transaction, `election.keys_ceremony_id` is cleared on every election in the
  event.
- Preserve completed trustee statuses for audit visibility.
- Ceremony becomes immutable (terminal).

---

## 4. Key Registration Gating

### 4.1 Ceremony creation

`create_keys_ceremony` directly inserts the ceremony at `AWAITING_TRUSTEE_KEYS`. There is no
separate admin "start" command and no connectivity precondition on creation.

### 4.2 Advancement gate

`AWAITING_TRUSTEE_KEYS → IN_PROGRESS` fires automatically — checked on every beat cycle —
once every selected trustee (BBT and server-based alike) has a `public_key` row scoped to
`(election_event_id, keys_ceremony_id)`. Server-based rows are populated idempotently by
Windmill in the same beat arm; BBT rows arrive independently via `/register-trustee-key`.
**Trustee online/offline state plays no role in this gate.** If any key is still missing,
the ceremony stays in `AWAITING_TRUSTEE_KEYS` and is retried on the next beat — it is never
rejected.

### 4.3 Step self-report endpoints

`register_trustee_key`, `confirm_key_backup`, and `confirm_key_check` are accepted
regardless of any trustee's connection state, including the caller's own. Their only guards
are the `TrusteeStatus` dependency rules in §5 and JWT identity (e.g.
`NO_PUBLIC_KEY_REGISTERED`, `INVALID_STATE`).

---

## 5. Step Progression

Steps per trustee, in order: `WAITING → KEY_GENERATED → KEY_RETRIEVED → KEY_CHECKED`
(renamed from `GENERATED → DOWNLOADED → CHECKED`).

Dependency rules:
- `KEY_RETRIEVED` requires `KEY_GENERATED` to be done
- `KEY_CHECKED` requires `KEY_RETRIEVED` to be done
- Out-of-order events are rejected (`INVALID_STATE`), not silently corrected

These rules are enforced independently of connection state (see §4.3) — a trustee can
complete every step while another trustee is offline.

---

## 6. Wizard Navigation Gating

`execution_status` and `trustee.status` are never derived client-side or from a formula —
they are explicit, persisted values written only at these call sites (see
[state machine doc](./key_ceremony_state_machine.md) for the authoritative `try_transition`
guard):

| Call site | Transition |
|---|---|
| `create_keys_ceremony` | (insert) → `AWAITING_TRUSTEE_KEYS` |
| `create_keys_impl` (beat task) | `AWAITING_TRUSTEE_KEYS` → `IN_PROGRESS` |
| `set_public_key_impl` (beat task) | `IN_PROGRESS` → `SUCCESS` |
| `/cancel-keys-ceremony` | `AWAITING_TRUSTEE_KEYS` / `IN_PROGRESS` / `SUCCESS` → `CANCELLED` |

The only client-local rule is the **wizard navigation gate**: while the logged-in trustee's
own session is `NOT_ACTIVE` (`HeadlessTrusteeContext.isConnected === false`), the
`TrusteeWizard` disables its "Next" action for that trustee. This is a per-trustee UX guard
only:
- It does not depend on any other trustee's connection state.
- It never writes to `execution_status` or `trustee.status`.
- It never blocks the ceremony-level gates in §4.

---

## 7. UI Rendering

### 7.1 Online indicator column

- `ACTIVE` → green check
- `NOT_ACTIVE` → red indicator (informational only; does not imply cancellation and does not
  block any other trustee's progress)

### 7.2 Step columns (Generated / Retrieved / Checked)

- Step `DONE` (`KEY_GENERATED` / `KEY_RETRIEVED` / `KEY_CHECKED`) → green check
- Otherwise → gray indicator / hourglass

Step rendering is not conditioned on `execution_status` except when `CANCELLED`.

### 7.3 Wizard navigation gating (own trustee offline)

- When the logged-in trustee's own session is `NOT_ACTIVE`, disable "Next" in their wizard
  and show an inline "reconnecting…" notice.
- No cross-trustee effect: other trustees' wizards, their step progress, and the
  ceremony-level `execution_status` are all unaffected.
- No cancellation banner.

### 7.4 `CANCELLED` state

- Show cancellation banner.
- Online indicator column reflects actual states (some rows may be red).
- Step cells stay at their last known state for audit; ceremony is immutable, no further
  advancement.
- There is **no in-place restart**. Recovery is creating a **new** ceremony — fresh create
  or duplicate-from-previous — under a new `keys_ceremony_id`. Trustees regenerate and
  re-register keys against that new id (see proposal §6, §9).

### 7.5 Prohibited logic

Remove any code that:
- Cancels the ceremony on trustee disconnection.
- Infers cancellation from online/connection state.
- Couples the online indicator's rendering with `execution_status` changes.
- Rejects a step self-report call, or blocks the key-registration gate, because some
  trustee — other than the caller — is offline.
- Computes `execution_status` or `trustee.status` from a client-side formula instead of
  reading the persisted value written at the call sites in §6.
- Resets a cancelled ceremony's steps "in place" instead of routing recovery through
  cancel + create-new (§7.4).
