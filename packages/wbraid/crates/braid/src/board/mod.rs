// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The unified board client (§6 of `crates/braid/v0.6_spec.md`).
//!
//! One component with the three §6 responsibilities: (a) the in-memory
//! [`store`] ([`MessageStore`]), (b) predicate [`persistence`] (anti-rewrite),
//! and (c) b4 interaction via a [`transport`]; [`mod@verify`] is the trust
//! boundary that turns a fetched message into the `(Predicate, Body)` pair (a) admits.
//! The orchestration is written once here; the persistence and transport
//! backends are swappable (in-memory / SQLite / HTTP+S3 / IndexedDB) so the
//! same logic serves M1 → M3.
//!
//! The client enforces the **update-first / loop-back** rule (§6): a message a
//! trustee produces has no local effect until it is fetched back, verified,
//! digest-persisted, and only then admitted to the store. The trustee reads the
//! store through the [`MessageStore`] returned by [`BoardClient::view`].

pub mod persistence;
pub mod store;
pub mod transport;
pub mod verify;

use anyhow::{bail, Result};

use cryptography::context::Context;

use crate::messages::artifact::Configuration;
use crate::messages::wire::ProtocolMessage;

use crate::messages::predicate::Predicate;
use persistence::Persistence;
use store::MessageStore;
use transport::{StagedRef, Transport};
use verify::verify;

/// The unified board client: owns the in-memory store, a persistence backend, and
/// a transport to b4. A constructed client always holds a `Configuration` (§9.8).
pub struct BoardClient<C: Context, T: Transport<C>, P: Persistence> {
    store: MessageStore<C>,
    /// The child (writable) board: `post` targets it, and it is one of the two
    /// boards `update` reads. For a non-union client this is the only board.
    transport: T,
    /// The parent (read-only) board of a union (§8.2). `Some` for a tally client
    /// (its DKG board); `None` for a plain single-board client. `update` reads it
    /// but `post` never targets it.
    parent: Option<T>,
    persistence: P,
    /// The durable set of predicates b4 has committed to (§6.2). Loaded on
    /// [`connect`](Self::connect) and grown by [`update`](Self::update); its sole
    /// purpose is the boundary anti-rewrite check (§6.3). For a union it is
    /// additionally **seeded** with the parent's predicates (§8.2).
    committed: Vec<Predicate>,
    /// The **own-post record** (§6.4): the slots this trustee has already staged
    /// for b4, and the handle that publishes each. Durable, predicate-sized, and
    /// loaded on construction — the outbound analogue of [`committed`], and the
    /// reason a restart cannot lose track of what was already handed over.
    own_posts: Vec<(Predicate, StagedRef)>,
}

impl<C: Context, T: Transport<C>, P: Persistence> BoardClient<C, T, P> {
    /// Construct against a board: fetch + accept its `Configuration` (§9.8
    /// post-condition — no Configuration ⇒ construction fails).
    ///
    /// Restart (loading persisted anti-rewrite predicates and re-checking them on
    /// re-fetch) is an M2 concern; `NoOpPersistence::load` returns empty.
    pub async fn connect(transport: T, persistence: P) -> Result<Self> {
        let configuration_message = transport.fetch_configuration().await?;
        let store = MessageStore::from_configuration_message(&configuration_message)?;
        let committed = persistence.load().await?;
        let own_posts = persistence.load_own_posts().await?;
        Ok(Self {
            store,
            transport,
            parent: None,
            persistence,
            committed,
            own_posts,
        })
    }

    /// Construct a **union** client (§8.2): a tally over a prior DKG.
    ///
    /// `child_transport` is the tally board (writable); `parent_transport` is the
    /// DKG board (read-only). The `Configuration` — the per-execution domain reused
    /// by tallies (§9.5) — is taken from the **parent**.
    ///
    /// `parent_predicates` is the anti-rewrite **seed** (§8.2): it MUST be the
    /// trustee's own committed digests from its DKG session, supplied out-of-band
    /// (via [`committed`](Self::committed) on the DKG client), never a fresh b4
    /// re-fetch. Seeding from the trustee's own memory is what forbids the child
    /// board from rewriting its parent's DKG history: [`update`](Self::update)
    /// re-fetches the parent for its bodies, but every fetched parent message is
    /// checked against this seed, so a rewritten DKG predicate halts.
    pub async fn connect_union(
        child_transport: T,
        parent_transport: T,
        persistence: P,
        parent_predicates: Vec<Predicate>,
    ) -> Result<Self> {
        let configuration_message = parent_transport.fetch_configuration().await?;
        let store = MessageStore::from_configuration_message(&configuration_message)?;
        let committed = persistence.load().await?;
        let own_posts = persistence.load_own_posts().await?;
        let mut client = Self {
            store,
            transport: child_transport,
            parent: Some(parent_transport),
            persistence,
            committed,
            own_posts,
        };
        client.seed_parent_predicates(parent_predicates).await?;
        Ok(client)
    }

    /// Seed the committed set with the parent's predicates (§8.2), so the tally's
    /// completeness gate (below) extends across the union: the DKG-session
    /// predicates become part of what every later `update()` must still be able
    /// to reconstruct. Persists each so the baseline is durable across tally
    /// restarts too.
    async fn seed_parent_predicates(&mut self, parent_predicates: Vec<Predicate>) -> Result<()> {
        for predicate in parent_predicates {
            if !self.committed.contains(&predicate) {
                self.persistence.persist(&predicate).await?;
                self.committed.push(predicate);
            }
        }
        Ok(())
    }

    /// Update-first (§6): fetch from b4, verify + admit each message, then run the
    /// anti-rewrite **completeness gate** (§6.3). For a union (§8.2) this reads
    /// BOTH the parent (DKG) and child (tally) boards and merges them into the one
    /// store the trustee sees. Idempotent — the store is a set keyed by predicate —
    /// so a full re-fetch each call is safe (§12; fetching the static parent every
    /// cycle is a known, deferred optimization).
    pub async fn update(&mut self) -> Result<()> {
        let mut messages = Vec::new();
        if let Some(parent) = &self.parent {
            messages.extend(parent.fetch().await?);
        }
        messages.extend(self.transport.fetch().await?);
        for message in &messages {
            self.admit(message).await?;
        }
        self.check_complete()
    }

    /// Verify a fetched message, then persist its digest and admit it to the store
    /// (§6.2). Collision detection is not this method's job (§5.3): a predicate
    /// that collides with an existing store entry is simply inserted alongside it
    /// under its own distinct key — the datalog's own `collides()` rule is what
    /// catches two such facts coexisting in one view.
    async fn admit(&mut self, message: &ProtocolMessage<C>) -> Result<()> {
        let (predicate, body) = verify(message, self.store.configuration())?;
        self.persistence.persist(&predicate).await?;
        if !self.committed.contains(&predicate) {
            self.committed.push(predicate.clone());
        }
        self.store.insert(predicate, body)?;
        Ok(())
    }

    /// Anti-rewrite completeness gate (§6.3): every predicate this client has ever
    /// committed to — durably persisted, and, for a union, seeded from the parent
    /// — must still be reconstructible from the current fetch. Plain equality, not
    /// `collides()`: within a slot the two are complements (a live entry is either
    /// equal to the committed one, or colliding with it — never neither), so a
    /// missing committed predicate is exactly the signature of b4 now serving
    /// something different (or nothing) for a slot it once committed to. This
    /// gates whether `step` runs at all this cycle; it does not itself detect
    /// equivocation between two live, simultaneously-fetched messages — that is
    /// the datalog's `collides()` rule, over whatever this gate lets through.
    fn check_complete(&self) -> Result<()> {
        let live = self.store.get_predicates();
        if let Some(missing) = self.committed.iter().find(|p| !live.contains(p)) {
            bail!(
                "anti-rewrite violation: committed predicate {:?} is no longer reconstructible from b4",
                missing
            );
        }
        Ok(())
    }

    /// Post messages to b4 — **compute-once, send-until-acked** (§6.4). Per the
    /// loop-back rule they take no local effect here; they become visible only
    /// after a subsequent [`update`](Self::update).
    ///
    /// For each outgoing message, one of two paths:
    ///
    /// - **The slot is already in the own-post record** ⇒ publish the *recorded*
    ///   message (`commit` its handle) and discard the one just computed. The
    ///   body was uploaded when it was staged, so committing sends only the
    ///   handle — no body crosses the wire again, which is what lets a recorded
    ///   post be published long after its bytes have left memory. Whatever the
    ///   failure was, the only message that can ever appear for that slot is the
    ///   recorded one.
    /// - **The slot is unrecorded** ⇒ `stage` the bytes, **record** the
    ///   `(predicate, handle)` pair durably — this write is the commit point —
    ///   then `commit`. A failure before the record means nothing was published
    ///   and no slot was claimed, so recomputing next cycle is safe.
    ///
    /// Why this exists: a successful post is a durable handoff (the body is
    /// stored and the row committed before b4 answers), but the author cannot
    /// tell "my post failed" from "my post landed and b4 is not serving it back
    /// to me". In the second case the datalog re-enables the action, since the
    /// slot still reads unfilled in the trustee's own view. Recomputing there
    /// would put two artifacts in one slot — exactly what `collides()` halts on
    /// (§5.2) — so a transport hiccup, a lost ack, or a crash could turn into a
    /// halt. Re-sending the recorded message instead makes the outcome the same
    /// as if the ack had arrived.
    ///
    /// The record is matched with the same `collides()` the datalog uses (§5.1 —
    /// one slot definition, reused), so "same slot" means exactly what it means
    /// everywhere else. Note that the recomputation itself is *not* avoided: the
    /// datalog has no knowledge of the record, so it keeps deriving the action
    /// and the action layer keeps producing an artifact, which this method then
    /// discards. Suppressing that work would require the record to reach into
    /// inference; it is left as a possible optimisation, not a requirement.
    pub async fn post(&mut self, messages: Vec<ProtocolMessage<C>>) -> Result<()> {
        for message in messages {
            let (predicate, _body) = verify(&message, self.store.configuration())?;
            if let Some((_, staged)) = self
                .own_posts
                .iter()
                .find(|(recorded, _)| *recorded == predicate || recorded.collides(&predicate))
            {
                let staged = staged.clone();
                self.transport.commit(&staged).await?;
                continue;
            }
            let staged = self.transport.stage(&message).await?;
            self.persistence
                .persist_own_post(&predicate, &staged)
                .await?;
            self.own_posts.push((predicate, staged.clone()));
            self.transport.commit(&staged).await?;
        }
        Ok(())
    }

    /// The slots this trustee has staged for b4, with their publish handles
    /// (§6.4). Exposed for diagnostics and tests.
    pub fn own_posts(&self) -> &[(Predicate, StagedRef)] {
        &self.own_posts
    }

    /// The accepted board `Configuration`.
    pub fn configuration(&self) -> &Configuration<C> {
        self.store.configuration()
    }

    /// The read view the trustee's `step` consumes.
    pub fn view(&self) -> &MessageStore<C> {
        &self.store
    }

    /// This client's committed predicate set (§6.2). A DKG session hands this to
    /// its tallies as the union anti-rewrite seed (§8.2) — the trustee's own
    /// memory of what it accepted during the DKG.
    pub fn committed(&self) -> &[Predicate] {
        &self.committed
    }
}

#[cfg(all(test, feature = "native"))]
mod tests {
    use std::marker::PhantomData;

    use anyhow::Result;

    use cryptography::context::{Context, RistrettoCtx};
    use cryptography::cryptosystem::elgamal::KeyPair;
    use cryptography::utils::signatures::SignatureScheme;

    use crate::messages::artifact::Configuration;
    use crate::messages::newtypes::{zero_hash, ConfigurationHash, PublicKeyHash};
    use crate::protocol_manager::ProtocolManager;
    use crate::messages::wire::ProtocolMessage;

    use cryptography::utils::serialization::VSerializable;

    use crate::board::persistence::NoOpPersistence;
    use crate::board::transport::{MemoryBoard, MemoryTransport};
    use crate::board::BoardClient;
    use crate::messages::predicate::Predicate;
    use crate::trustee::Trustee;

    const DATE: crate::messages::newtypes::Timestamp = 0;

    /// A minimal manager + `n`-trustee configuration for board-client tests.
    struct Setup<C: Context> {
        pm: ProtocolManager<C>,
        signing_keys: Vec<<C::SignatureScheme as SignatureScheme<C::Rng>>::Signer>,
        cfg: Configuration<C>,
        cfg_hash: ConfigurationHash,
        cfg_message: ProtocolMessage<C>,
    }

    fn setup<C: Context>(n: usize) -> Result<Setup<C>> {
        let mut key_rng = C::get_rng();
        let pm = ProtocolManager::<C>::new(C::SignatureScheme::gen_signing_key(&mut key_rng));
        let mut signing_keys = Vec::new();
        let mut trustee_vks = Vec::new();
        let mut share_enc_keys = Vec::new();
        for _ in 0..n {
            let sk = C::SignatureScheme::gen_signing_key(&mut key_rng);
            trustee_vks.push(C::SignatureScheme::verifying_key(&sk));
            signing_keys.push(sk);
            let keypair = KeyPair::<C>::generate();
            share_enc_keys.push(keypair.pkey.y.clone());
        }
        let cfg = Configuration::<C>::new(
            0,
            C::SignatureScheme::verifying_key(&pm.signing_key),
            trustee_vks,
            2,
            2,
            share_enc_keys,
            PhantomData,
        );
        let cfg_hash = ConfigurationHash::from_configuration(&cfg)?;
        let cfg_message = ProtocolMessage::<C>::configuration(&pm, DATE, &cfg);
        Ok(Setup {
            pm,
            signing_keys,
            cfg,
            cfg_hash,
            cfg_message,
        })
    }

    /// Compute-once (§6.4): once a slot is in the own-post record, a *recomputed*
    /// artifact for that slot is never published — the recorded message is
    /// re-sent instead. This is the state b4 can create by accepting a message
    /// and then not serving it back to its author: the author's own view still
    /// shows the slot unfilled, so the datalog re-enables the action and the
    /// action layer produces a fresh artifact. Publishing that would put two
    /// artifacts in one slot, which the datalog treats as equivocation (§5.2).
    #[tokio::test]
    async fn recorded_slot_publishes_the_recorded_message() -> Result<()> {
        run_recorded_slot::<RistrettoCtx>().await
    }

    async fn run_recorded_slot<C: Context>() -> Result<()> {
        let Setup {
            signing_keys,
            cfg,
            cfg_hash,
            cfg_message,
            ..
        } = setup::<C>(2)?;
        let sk1 = signing_keys.into_iter().next().unwrap();
        let trustee1 = Trustee::<C>::new("1".to_string(), sk1, KeyPair::<C>::generate(), &cfg)?;

        let board = MemoryBoard::<C>::new();
        board.push(cfg_message);
        let mut client =
            BoardClient::connect(MemoryTransport::new(board.clone()), NoOpPersistence).await?;

        let first = ProtocolMessage::<C>::shares(&trustee1, DATE, cfg_hash, &vec![1u8, 2, 3]);
        let first_bytes = first.ser();
        client.post(vec![first]).await?;
        assert_eq!(board.snapshot().len(), 2, "Configuration + the first Shares");
        assert_eq!(client.own_posts().len(), 1, "the slot is now recorded");

        // A recomputed sharing for the same slot: fresh randomness, so a different
        // body and a different predicate, but the same slot.
        let recomputed = ProtocolMessage::<C>::shares(&trustee1, DATE, cfg_hash, &vec![4u8, 5, 6]);
        client.post(vec![recomputed]).await?;

        let snapshot = board.snapshot();
        assert_eq!(
            snapshot.len(),
            2,
            "the recomputed artifact must not reach the board"
        );
        assert_eq!(
            snapshot[1].ser(),
            first_bytes,
            "the board still holds the originally recorded message"
        );
        assert_eq!(client.own_posts().len(), 1, "still one slot recorded");
        Ok(())
    }

    /// A union client (§8.2) reads BOTH boards into one store and writes only to
    /// the child: the parent's `Shares` and the child's `Ballots` are both visible
    /// to the trustee, while `post` lands only on the child (tally) board.
    #[tokio::test]
    async fn union_merges_parent_and_child() -> Result<()> {
        run_union_merges::<RistrettoCtx>().await
    }

    async fn run_union_merges<C: Context>() -> Result<()> {
        let Setup {
            pm,
            signing_keys,
            cfg,
            cfg_hash,
            cfg_message,
        } = setup::<C>(2)?;
        let sk1 = signing_keys.into_iter().next().unwrap();
        let trustee1 =
            Trustee::<C>::new("1".to_string(), sk1, KeyPair::<C>::generate(), &cfg)?;

        // Parent (DKG) board: Configuration + a Shares from trustee 1.
        let parent_board = MemoryBoard::<C>::new();
        parent_board.push(cfg_message);
        parent_board.push(ProtocolMessage::<C>::shares(
            &trustee1,
            DATE,
            cfg_hash,
            &vec![1u8, 2, 3],
        ));

        // Child (tally) board: a Ballots from the manager (dummy body — verify only
        // hashes it and checks the manager signature).
        let child_board = MemoryBoard::<C>::new();
        child_board.push(ProtocolMessage::<C>::ballots(
            &pm,
            DATE,
            cfg_hash,
            PublicKeyHash(zero_hash()),
            vec![1, 2],
            1,
            &vec![9u8, 9, 9],
        ));

        let mut client = BoardClient::connect_union(
            MemoryTransport::new(child_board.clone()),
            MemoryTransport::new(parent_board.clone()),
            NoOpPersistence,
            Vec::new(),
        )
        .await?;
        client.update().await?;

        let predicates = client.view().get_predicates();
        assert!(
            predicates.iter().any(|p| matches!(p, Predicate::Shares(_))),
            "parent Shares must be merged into the union view"
        );
        assert!(
            predicates
                .iter()
                .any(|p| matches!(p, Predicate::Ballots(_))),
            "child Ballots must be merged into the union view"
        );

        // A post targets the child board only.
        client
            .post(vec![ProtocolMessage::<C>::shares(
                &trustee1,
                DATE,
                cfg_hash,
                &vec![7u8, 7, 7],
            )])
            .await?;
        assert_eq!(
            child_board.snapshot().len(),
            2,
            "post lands on the child (Ballots + posted message)"
        );
        assert_eq!(
            parent_board.snapshot().len(),
            2,
            "parent is untouched by post (Configuration + Shares)"
        );
        Ok(())
    }

    /// Anti-rewrite completeness gate across the union (§6.3/§8.2): the tally is
    /// seeded with the trustee's own DKG-session committed digests. If b4 later
    /// serves a *different* board for that same union — same Configuration, but a
    /// colliding Shares in place of the one actually committed during the DKG —
    /// the original committed predicate is no longer reconstructible, so
    /// `update()`'s completeness gate blocks it before `step` ever runs over the
    /// incomplete view.
    #[tokio::test]
    async fn union_seed_blocks_parent_rewrite() -> Result<()> {
        run_union_anti_rewrite::<RistrettoCtx>().await
    }

    async fn run_union_anti_rewrite<C: Context>() -> Result<()> {
        let Setup {
            pm,
            signing_keys,
            cfg,
            cfg_hash,
            cfg_message,
        } = setup::<C>(2)?;
        let sk1 = signing_keys.into_iter().next().unwrap();
        let trustee1 =
            Trustee::<C>::new("1".to_string(), sk1, KeyPair::<C>::generate(), &cfg)?;

        let dkg_board = MemoryBoard::<C>::new();
        dkg_board.push(cfg_message);
        dkg_board.push(ProtocolMessage::<C>::shares(
            &trustee1,
            DATE,
            cfg_hash,
            &vec![1u8, 2, 3],
        ));

        // The DKG session's committed digests are the trustee's own memory of the
        // DKG — this is the anti-rewrite seed (never a fresh b4 re-fetch).
        let seed = {
            let mut dkg =
                BoardClient::connect(MemoryTransport::new(dkg_board.clone()), NoOpPersistence)
                    .await?;
            dkg.update().await?;
            dkg.committed().to_vec()
        };

        // A dishonest b4 now serves a DIFFERENT board for the same union: the same
        // Configuration, but a colliding Shares (different body) in place of the
        // one the DKG session actually saw — never the original alongside it, or
        // datalog's own collides() would catch it trivially with no seed needed.
        let rewritten_parent_board = MemoryBoard::<C>::new();
        rewritten_parent_board.push(ProtocolMessage::<C>::configuration(&pm, DATE, &cfg));
        rewritten_parent_board.push(ProtocolMessage::<C>::shares(
            &trustee1,
            DATE,
            cfg_hash,
            &vec![4u8, 5, 6],
        ));

        let child_board = MemoryBoard::<C>::new();
        let mut client = BoardClient::connect_union(
            MemoryTransport::new(child_board.clone()),
            MemoryTransport::new(rewritten_parent_board.clone()),
            NoOpPersistence,
            seed,
        )
        .await?;
        let result = client.update().await;
        assert!(
            result.is_err(),
            "seeded DKG digest must block the parent rewrite"
        );
        Ok(())
    }

    /// The complementary case (§5.3): when b4 serves BOTH the original and a
    /// colliding message together in one view — no restart, no union, nothing
    /// missing from history — the completeness gate has nothing to say (both are
    /// reconstructible). It is the datalog's own `collides()` rule, run over the
    /// resulting store, that catches this one.
    #[tokio::test]
    async fn live_collision_is_caught_by_datalog_not_completeness() -> Result<()> {
        run_live_collision::<RistrettoCtx>().await
    }

    async fn run_live_collision<C: Context>() -> Result<()> {
        let Setup {
            cfg,
            cfg_hash,
            cfg_message,
            signing_keys,
            ..
        } = setup::<C>(2)?;
        let sk1 = signing_keys.into_iter().next().unwrap();
        let trustee1 =
            Trustee::<C>::new("1".to_string(), sk1, KeyPair::<C>::generate(), &cfg)?;

        let board = MemoryBoard::<C>::new();
        board.push(cfg_message);
        board.push(ProtocolMessage::<C>::shares(
            &trustee1,
            DATE,
            cfg_hash,
            &vec![1u8, 2, 3],
        ));
        // Trustee 1 equivocates: a second, different Shares for the same slot,
        // served alongside the first (nothing withheld, nothing missing).
        board.push(ProtocolMessage::<C>::shares(
            &trustee1,
            DATE,
            cfg_hash,
            &vec![4u8, 5, 6],
        ));

        let mut client =
            BoardClient::connect(MemoryTransport::new(board.clone()), NoOpPersistence).await?;
        client.update().await?;

        let predicates = client.view().get_predicates();
        assert_eq!(
            predicates
                .iter()
                .filter(|p| matches!(p, Predicate::Shares(_)))
                .count(),
            2,
            "both colliding Shares must be admitted — the completeness gate does not reject either"
        );
        assert!(
            crate::datalog::composed::run(&predicates).is_err(),
            "datalog's own collides() rule must halt on two live Shares for the same sender"
        );
        Ok(())
    }
}
