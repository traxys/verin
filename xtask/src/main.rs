use clap::{CommandFactory, Parser};

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

    let args = Args::from_args();
    match args.task {
        None => Args::command().print_help()?,
        Some(task) => match task {
            Task::Build => todo!(),
        },
    }
    Ok(())
}
