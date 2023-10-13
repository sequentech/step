pub mod decode_ballots;
pub mod error;
pub mod pipe_name;

use self::error::Result;

use self::{
    decode_ballots::{ballot_codec::BallotCodec, DecodeBallots},
    pipe_name::PipeName,
};

trait Pipe {
    // pipe execution
    fn exec(&self) -> Result<()> {
        // dbg!(&self.config);
        // dbg!(&self.input_dir);
        // dbg!(&self.output_dir);
        //
        // // file handle to log execution process into
        // dbg!(&self.output_log_file);

        Ok(())
    }

    // load input
    // fn input(&self) -> Result<()>;

    // produce output
    // fn output(&self) -> Result<()>;
}

// TODO: rework this better
// TODO: pointeur sur fonction
pub fn match_run(pipe: PipeName) -> Result<()> {
    match pipe {
        PipeName::DecodeBallots => {
            let pipe = DecodeBallots::new();
            pipe.exec()?;
        }
        _ => {}
    };

    Ok(())
}
