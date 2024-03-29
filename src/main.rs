use std::env;
use std::process;

use scale_testing::Config;

fn main() {
    let args: Vec<String> = env::args().collect();

    let config = Config::build(&args).unwrap_or_else(|err| {
        println!("Problem parsing arguments: {err}");
        process::exit(1);
    });

    println!("Replication multiplier {}", config.replicator);
    println!("Source directory {}", config.src);
    println!("Destination directory {}", config.dst);

    if let Err(e) = scale_testing::run(config) {
        println!("Application error: {}", e);
        process::exit(1);
    }
}
