pub mod configure;

use crate::types::args::{Args, ArgsCommand};

pub fn run(args: Args) {
    match args.command {
        ArgsCommand::Configure { field1, field2 } => {
            configure::run(field1, field2);
        },
    }
}