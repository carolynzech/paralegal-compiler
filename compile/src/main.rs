use std::env;
use std::fs;
use std::process::Command;

use compile::compile;
use compile::parsers::parse;
use std::io::Result;

fn run(args: &Vec<String>) -> Result<()> {
    if args.len() < 2 {
        panic!("Need to pass path to policy file");
    }
    let policy_file = &args[1];
    let policy = fs::read_to_string(policy_file)
        .expect("Could not read policy file")
        .to_lowercase();

    let res = parse(&policy);
    match res {
        Ok((_, ast)) => compile(ast),
        Err(e) => panic!("{}", e),
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    run(&args)?;
    // Command::new("rustfmt compiled-policy.rs").output().expect("failed to run cargo fmt");
    Ok(())
}
