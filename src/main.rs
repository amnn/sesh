use clap::Parser;
use clap::Subcommand;

use sesh::skim;
use sesh::tmux;

#[derive(Debug, Parser)]
#[command(name = "sesh", version, about)]
#[command(arg_required_else_help = true)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run `sesh` in a tmux pop-up.
    Popup {
        /// Width of the popup, passed to tmux as-is.
        #[arg(short = 'w', long = "width", default_value = "60%")]
        width: String,

        /// Height of the popup, passed to tmux as-is.
        #[arg(short = 'h', long = "height", default_value = "60%")]
        height: String,

        /// Title of the popup, passed to tmux as-is.
        #[arg(short = 'T', long = "title", default_value = "sesh")]
        title: String,

        /// Remaining arguments are forwarded to the underlying invocation of `sesh cli`.
        #[arg(last = true, trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Run `sesh` in the current terminal.
    Cli,
}

fn main() -> anyhow::Result<()> {
    match Args::parse().command {
        Command::Popup {
            width,
            height,
            title,
            args,
        } => tmux::popup(&width, &height, &title, &args),

        Command::Cli => {
            tmux::ensure()?;

            let sessions = tmux::sessions()?;
            skim::run(sessions);
            Ok(())
        }
    }
}
