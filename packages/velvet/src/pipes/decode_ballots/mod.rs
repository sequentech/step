pub mod ballot_codec;

use self::ballot_codec::BallotCodec;
use super::{error::Result, Pipe};

pub struct DecodeBallots {}

impl DecodeBallots {
    pub fn new() -> Self {
        Self {}
    }
}

impl Pipe for DecodeBallots {
    fn exec(&self) -> Result<()> {
        let choices = vec![0, 0, 0, 1, 0, 0];

        let ballot_codec = BallotCodec::new(vec![2, 2, 2, 2, 2, 2]);
        let encoded_ballot = ballot_codec.encode_ballot(choices.clone());
        let _decoded_ballot = ballot_codec.decode_ballot(encoded_ballot);

        Ok(())
    }
}
