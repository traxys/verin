use std::path::Path;

use clap::{CommandFactory, Parser};
use duct::cmd;

#[derive(Parser)]
struct Args {
    #[clap(subcommand)]
    task: Option<Task>,
}

#[derive(Parser)]
enum Task {
    Build,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace = manifest
        .ancestors()
        .nth(1)
        .expect("could not get workspace root");

    let args = Args::from_args();
    match args.task {
        None => Args::command().print_help()?,
        Some(task) => match task {
            Task::Build => {
                let posts = workspace.join("posts");
                let target = workspace.join("target/html");
                cmd!(env!("CARGO"), "run", "--", "build", posts, target)
                    .dir(workspace)
                    .run()?;
            }
        },
    }
    Ok(())
}
