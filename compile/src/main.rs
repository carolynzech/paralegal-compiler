use std::env;
use std::fs;

use compile::parse;

fn run(args: &Vec<String>) {
    if args.len() < 1 {
        panic!("Need to pass path to policy file");
    }
    let policy_file = &args[1];
    println!("Policy file is {}", policy_file);
    let policy = fs::read_to_string(policy_file).expect("Could not read policy file");
    println!("Policy is {}", policy);

    // business logic
    parse(&policy);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    run(&args);
}

/*
Road map:
-- read in policy text from file
-- check for proper formatting (optional I guess)
-- parse
-- generate boilerplate
-- then specific policy from parser output
 */
