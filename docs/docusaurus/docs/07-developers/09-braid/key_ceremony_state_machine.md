---
id: key_ceremony_state_machine
title: Keys Ceremony — Runtime State Machine
sidebar_label: Keys Ceremony State Machine
---

# Keys Ceremony — Runtime State Machine

This document specifies the `KeysCeremonyExecutionStatus` transition table and the Rust
implementation that enforces it at runtime.  It is the authoritative design for how status
changes are validated across Harvest, Windmill beat tasks, and the admin portal.

See [BBT Signing Keypair Proposal](./key_ceremony_bbt_propossal_no_protoccol_change.md) for the
broader BBT key ceremony design that this state machine is part of.

---

## Transition graph

```
AWAITING_TRUSTEE_KEYS ──(all keys registered)──▶ STARTED
          STARTED      ──(config posted)────────▶ IN_PROGRESS
      IN_PROGRESS      ──(all KEY_CHECKED)──────▶ SUCCESS

AWAITING_TRUSTEE_KEYS ─┐
          STARTED       ├─(cancel)──────────────▶ CANCELLED
      IN_PROGRESS       ─┘
          SUCCESS       ──▶ (terminal — cancel gated on voting period, outside this enum)
```

`SUCCESS` and `CANCELLED` are terminal: no further transitions are permitted.

---

## Why runtime, not compile-time typestate

The typestate pattern (each status as a distinct generic parameter, transitions as methods
that consume `self`) only buys safety for values that live inside a single Rust process.
Here the state is persisted as a string in the DB and reconstructed on every Harvest request,
beat-service tick, and Windmill task.  The flow is:

1. Read `execution_status` string from DB.
2. Parse into `KeysCeremonyExecutionStatus`.
3. Validate the requested transition.
4. Write the new status string back to DB.

Step 2 is a `match` — and the real guard already happens there, before any generic type could
act.  Wrapping this in `Ceremony<AwaitingTrusteeKeys>` phantom types would add generic
blow-up and lose the single `try_transition` call for no added safety across the boundary
that actually matters (DB + multi-service dispatch).

---

## Implementation

### Where it lives

`sequent-core/src/types/ceremonies.rs` — added as methods on the existing
`KeysCeremonyExecutionStatus` enum so all crates that already import it get the guard for
free, with no new dependency.

### Code

```rust
/// One error type for every illegal move.  Carries enough context for a
/// descriptive Harvest response (e.g. mapped to a generic
/// `INVALID_CEREMONY_TRANSITION` error body).
#[derive(Debug, PartialEq, Eq)]
pub struct InvalidTransition {
    pub from: KeysCeremonyExecutionStatus,
    pub to:   KeysCeremonyExecutionStatus,
}

impl fmt::Display for InvalidTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid ceremony transition {:?} -> {:?}", self.from, self.to)
    }
}
impl std::error::Error for InvalidTransition {}

impl KeysCeremonyExecutionStatus {
    /// Validate a requested transition.  Returns the target status on success
    /// so callers can write it straight to the DB:
    ///
    /// ```ignore
    /// let next = current.try_transition(STARTED)?;
    /// update_execution_status(ceremony_id, next).await?;
    /// ```
    pub fn try_transition(
        self,
        to: KeysCeremonyExecutionStatus,
    ) -> Result<KeysCeremonyExecutionStatus, InvalidTransition> {
        let ok = matches!(
            (self, to),
            // forward progress
            (AWAITING_TRUSTEE_KEYS, STARTED)
                | (STARTED, IN_PROGRESS)
                | (IN_PROGRESS, SUCCESS)
            // cancellation (mirrors the tally ceremony table;
            // SUCCESS is terminal here — cancel-after-success is gated
            // on the voting period, which lives outside this enum)
                | (AWAITING_TRUSTEE_KEYS, CANCELLED)
                | (STARTED, CANCELLED)
                | (IN_PROGRESS, CANCELLED)
        );

        if ok { Ok(to) } else { Err(InvalidTransition { from: self, to }) }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, SUCCESS | CANCELLED)
    }
}
```

This mirrors the style of `update_tally_ceremony` in
`windmill/src/services/ceremonies/tally_ceremony.rs`, which already validates tally
status transitions with a hand-rolled `match` table.  The two ceremonies now read the same
way.

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::KeysCeremonyExecutionStatus::*;

    #[test]
    fn happy_path() {
        assert_eq!(AWAITING_TRUSTEE_KEYS.try_transition(STARTED),      Ok(STARTED));
        assert_eq!(STARTED.try_transition(IN_PROGRESS),                Ok(IN_PROGRESS));
        assert_eq!(IN_PROGRESS.try_transition(SUCCESS),                Ok(SUCCESS));
    }

    #[test]
    fn skipping_a_state_is_rejected() {
        assert!(AWAITING_TRUSTEE_KEYS.try_transition(IN_PROGRESS).is_err());
    }

    #[test]
    fn cancellation_arms() {
        assert!(AWAITING_TRUSTEE_KEYS.try_transition(CANCELLED).is_ok());
        assert!(STARTED.try_transition(CANCELLED).is_ok());
        assert!(IN_PROGRESS.try_transition(CANCELLED).is_ok());
        assert!(SUCCESS.try_transition(CANCELLED).is_err()); // gated elsewhere
    }

    #[test]
    fn terminals_go_nowhere() {
        assert!(SUCCESS.try_transition(STARTED).is_err());
        assert!(CANCELLED.try_transition(STARTED).is_err());
    }

    #[test]
    fn round_trips_through_serde_as_the_db_would() {
        let s    = serde_json::to_string(&AWAITING_TRUSTEE_KEYS).unwrap();
        let back: super::KeysCeremonyExecutionStatus = serde_json::from_str(&s).unwrap();
        assert_eq!(back.try_transition(STARTED), Ok(STARTED));
    }
}
```

---

## Call sites

Every place that currently writes `execution_status` directly to the DB should route through
`try_transition` instead:

| Call site | File | Transition to enforce |
|---|---|---|
| `process_board` (AWAITING → STARTED) | `windmill/src/tasks/process_board.rs` | `AWAITING_TRUSTEE_KEYS → STARTED` |
| `create_keys_impl` (STARTED → IN_PROGRESS) | `windmill/src/tasks/create_keys.rs` | `STARTED → IN_PROGRESS` |
| `set_public_key_impl` (IN_PROGRESS → SUCCESS via automated policy) | `windmill/src/tasks/set_public_key.rs` | `IN_PROGRESS → SUCCESS` |
| `/cancel-keys-ceremony` (future Harvest endpoint) | `harvest/src/routes/keys_ceremony.rs` | any non-terminal → `CANCELLED` |
