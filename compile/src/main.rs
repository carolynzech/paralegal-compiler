use std::env;
use std::fs;

use compile::compile;
use compile::parse;
use std::io::Result;

fn run(args: &Vec<String>) -> Result<()> {
    if args.len() < 1 {
        panic!("Need to pass path to policy file");
    }
    let policy_file = &args[1];
    println!("Policy file is {}", policy_file);
    let policy = fs::read_to_string(policy_file)
        .expect("Could not read policy file")
        .replace(" ", "");
    println!("Policy is {}", policy);

    let res = parse(&policy);
    match res {
        Ok((remainder, parsed)) => {
            if !remainder.is_empty() {
                panic!("failed to parse entire policy");
            } else {
                compile(&parsed)?;
            }
        }
        Err(e) => panic!("{}", e),
    };
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    run(&args)?;
    Ok(())
}
