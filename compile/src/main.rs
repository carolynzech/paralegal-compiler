use std::env;
use std::fs;

use compile::compile;
use compile::parsers::parse;
use std::collections::HashMap;
use std::io::Result;

fn run(args: &Vec<String>) -> Result<()> {
    if args.len() < 2 {
        panic!("Need to pass path to policy file");
    }
    let policy_file = &args[1];
    let policy = fs::read_to_string(policy_file).expect("Could not read policy file");

    let res = parse(&policy);
    // dbg!(&res);

    // let mut map: HashMap<String, String> = HashMap::new();

    // match res {
    //     Ok((remainder, parsed)) => {
    //         if !remainder.is_empty() {
    //             panic!("failed to parse entire policy");
    //         } else {
    //             compile(&parsed, &mut map)?;
    //         }
    //     }
    //     Err(e) => panic!("{}", e),
    // };
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    run(&args)?;
    Ok(())
}
