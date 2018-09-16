extern crate watchexec;
extern crate notify;
#[macro_use]
extern crate log;

use watchexec::{cli, error, run, pathop};

use pathop::PathOp;

fn main() -> error::Result<()> {
    run(cli::get_args(), |path: Vec<PathOp>| {
        debug!("{:?}", path)
    })
}
