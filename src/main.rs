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

    let result = ui::run(mode)?;

    println!(
        "wpm {:.1}  raw {:.1}  acc {:.1}%  consistency {:.1}%  time {:.1}s",
        result.wpm, result.raw_wpm, result.accuracy, result.consistency, result.test_duration
    );

    if args.no_sync {
        return Ok(());
    }
    match cfg.ape_key.as_deref() {
        Some(key) => match api::submit_result(key, &result).await {
            Ok(id) => println!("synced: {id}"),
            Err(e) => eprintln!("sync failed: {e}"),
        },
        None => eprintln!("no MONKEYTYPE_APE_KEY set — skipping sync"),
    }
    Ok(())
}
