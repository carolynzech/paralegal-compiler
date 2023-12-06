use std::env;
use std::fs;

use compile::compile;
use compile::parsers::{parse_bindings, parse_body};
use std::io::Result;

fn run(args: &Vec<String>) -> Result<()> {
    if args.len() < 2 {
        panic!("Need to pass path to policy file");
    }
    let policy_file = &args[1];
    let policy = fs::read_to_string(policy_file)
        .expect("Could not read policy file")
        .to_lowercase();

    let bindings_res = parse_bindings(&policy);
    match bindings_res {
        Ok((remainder, bindings)) => {
            // let mut env: HashMap<String, (Quantifier, String)> = HashMap::new();
            // construct_env(bindings, &mut env);
            let body_res = parse_body(remainder);
            // dbg!(&body_res)
            match body_res {
                Ok((_, policy_body)) => compile(policy_body, bindings)?,
                Err(e) => panic!("{}", e),
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
