use clap::Parser;
use anyhow::Result;
/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// file name
    #[arg(short, long)]
    name: Option<String>,
    /// Sets the output directory
    #[arg(short, long, default_value = "./output")]
    directory: String,
    /// Sets the concurrency level
    #[arg(short, long, default_value = "10")]
    concurrency: usize,
    /// Set the retry time
    #[arg(short, long, default_value = "10")]
    max_retry: usize,

    /// Sets the URL to download from
    url: String,
}



fn main() -> Result<()> {
    let args = Args::parse();
    let name = if let Some(name) = args.name {
        name
    } else {
        args.url.split('/').last().unwrap_or_default().to_string()
    };
    println!("name: {}", name);
    println!("Output directory: {}", args.directory);
    println!("Concurrency: {}", args.concurrency);
    println!("Retry: {}", args.max_retry);
    println!("url: {}", args.url);

    Ok(())
}
