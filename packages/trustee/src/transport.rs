// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Outages and verdicts.
//!
//! braid's errors are untyped. An error of the board service's transport is an
//! outage, which heals once the service is back; any other error of a session
//! is a verdict on what the board served, or a local fault, and halts the
//! session. The one place an error's origin is known is braid's `Transport`
//! seam, so every error that crosses it is tagged there.

use std::fmt;

use anyhow::Result;
use async_trait::async_trait;
use cryptography::context::Context;
use wbraid::board::transport::{StagedRef, Transport};
use wbraid::messages::wire::ProtocolMessage;

/// The tag on every error of the board service's transport.
#[derive(Debug)]
pub(crate) struct Outage;

impl fmt::Display for Outage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("the board service could not be used")
    }
}

/// A transport whose every error carries the [`Outage`] tag.
pub(crate) struct OutageTagged<T>(T);

impl<T> OutageTagged<T> {
    pub(crate) fn new(transport: T) -> OutageTagged<T> {
        OutageTagged(transport)
    }
}

#[async_trait(?Send)]
impl<C: Context, T: Transport<C>> Transport<C> for OutageTagged<T> {
    async fn fetch_configuration(&self) -> Result<ProtocolMessage<C>> {
        self.0.fetch_configuration().await.map_err(tag)
    }

    async fn fetch(&self) -> Result<Vec<ProtocolMessage<C>>> {
        self.0.fetch().await.map_err(tag)
    }

    async fn stage(&self, message: &ProtocolMessage<C>) -> Result<StagedRef> {
        self.0.stage(message).await.map_err(tag)
    }

    async fn commit(&self, staged: &StagedRef) -> Result<()> {
        self.0.commit(staged).await.map_err(tag)
    }
}

fn tag(err: anyhow::Error) -> anyhow::Error {
    err.context(Outage)
}

/// What an error of a session means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// The board service could not be used: tried again next cycle.
    Outage,
    /// Anything else: the session halts.
    Halt,
}

/// An error is an outage if its transport tagged it, under whatever context
/// braid or the trustee added on top.
pub(crate) fn classify(err: &anyhow::Error) -> Verdict {
    match err.downcast_ref::<Outage>() {
        Some(_) => Verdict::Outage,
        None => Verdict::Halt,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{anyhow, bail, Context as _};
    use protocol_board::Ctx;

    /// A board service that answers nothing.
    struct Down;

    #[async_trait(?Send)]
    impl Transport<Ctx> for Down {
        async fn fetch_configuration(&self) -> Result<ProtocolMessage<Ctx>> {
            bail!("connection refused")
        }

        async fn fetch(&self) -> Result<Vec<ProtocolMessage<Ctx>>> {
            bail!("connection refused")
        }

        async fn stage(
            &self,
            _message: &ProtocolMessage<Ctx>,
        ) -> Result<StagedRef> {
            bail!("connection refused")
        }

        async fn commit(&self, _staged: &StagedRef) -> Result<()> {
            bail!("connection refused")
        }
    }

    /// Any message will do for a transport that fails before looking at it.
    fn a_message() -> ProtocolMessage<Ctx> {
        ProtocolMessage::<Ctx>::configuration(
            &protocol_board::generate_manager(),
            0,
            &vec![1u8, 2, 3],
        )
    }

    /// With context on top, as the session set adds it.
    fn as_the_session_sees_it<T>(result: Result<T>) -> anyhow::Error {
        match result
            .context("posting to the board")
            .context("board dkg_1")
        {
            Ok(_) => panic!("the transport did not fail"),
            Err(err) => err,
        }
    }

    /// Each of the four transport calls braid makes is tagged, and the tag
    /// survives the context added on top of it.
    #[tokio::test]
    async fn every_error_of_the_tagged_transport_is_an_outage() {
        let tagged: &dyn Transport<Ctx> = &OutageTagged::new(Down);
        let errors = [
            as_the_session_sees_it(tagged.fetch_configuration().await),
            as_the_session_sees_it(tagged.fetch().await),
            as_the_session_sees_it(tagged.stage(&a_message()).await),
            as_the_session_sees_it(
                tagged.commit(&StagedRef("1".to_string())).await,
            ),
        ];
        for err in errors {
            assert_eq!(classify(&err), Verdict::Outage, "{err:#}");
            assert!(format!("{err:#}").contains("connection refused"));
        }
    }

    /// Only the tag makes an outage: an error retold with the same text is a
    /// verdict.
    #[tokio::test]
    async fn an_error_the_transport_did_not_tag_is_a_verdict() {
        let untagged = as_the_session_sees_it(Down.fetch().await);
        assert_eq!(classify(&untagged), Verdict::Halt, "{untagged:#}");

        let transport: &dyn Transport<Ctx> = &OutageTagged::new(Down);
        let tagged = as_the_session_sees_it(transport.fetch().await);
        let retold = anyhow!("{tagged:#}");
        assert_eq!(classify(&retold), Verdict::Halt, "{retold:#}");
    }
}
