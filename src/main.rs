mod api;
mod config;
mod test;
mod ui;
mod words;

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "mtt", about = "Terminal monkeytype client")]
struct Args {
    /// Test duration in seconds (time mode)
    #[arg(short, long, default_value_t = 30)]
    time: u32,

    /// Word count mode (overrides --time when set)
    #[arg(short, long)]
    words: Option<u32>,

    /// Skip uploading result to monkeytype
    #[arg(long)]
    no_sync: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let cfg = config::Config::load()?;

    let mode = match args.words {
        Some(n) => test::Mode::Words(n),
        None => test::Mode::Time(args.time),
    };

    let mut initial: Option<test::TestResult> = None;
    let mut sync_status: Option<String> = None;

    loop {
        let outcome = ui::run(mode, initial.take(), sync_status.as_deref())?;
        match outcome {
            ui::Outcome::Quit => break,
            ui::Outcome::Replay => {
                sync_status = None;
            }
            ui::Outcome::Sync(result) => {
                if args.no_sync {
                    sync_status = Some("sync disabled (--no-sync)".into());
                } else {
                    match cfg.ape_key.as_deref() {
                        Some(key) => match api::submit_result(key, &result).await {
                            Ok(id) => sync_status = Some(format!("synced: {id}")),
                            Err(e) => sync_status = Some(format!("sync failed: {e}")),
                        },
                        None => {
                            sync_status =
                                Some("no MONKEYTYPE_APE_KEY set — cannot sync".into());
                        }
                    }
                }
                initial = Some(result);
            }
        }
    }
    Ok(())
}
