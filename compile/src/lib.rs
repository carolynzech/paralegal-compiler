use handlebars::{no_escape, Handlebars};
use std::collections::HashMap;
use std::fs;
use std::io::Result;
use toml::Table;

pub mod parsers;

// template names
const BASE_TEMPLATE: &str = "base";
const FLOWS_TO_TEMPLATE: &str = "flows-to";
const CONTROL_FLOW_TEMPLATE: &str = "control-flow";

/* TODOs
    (Paralegal Functionality)
    -
    (Good Practice / User Experience / Nits)
    - deal with spaces properly
    - better error handling
    - pass template file paths as arguments instead of string literals
    - escaping {{}} in Rust code w/o overwriting no-escape for HTML characters
    - cargo new for the policy and write a template a Cargo.toml for it as well
*/

// #[derive(Debug)]
// enum Quantifier {
//     Some,
//     All,
//     No,
// }

// impl From<&str> for Quantifier {
//     fn from(s: &str) -> Self {
//         match s {
//             "some" => Quantifier::Some,
//             "all" => Quantifier::All,
//             "no" => Quantifier::No,
//             &_ => unimplemented!("no other quantifiers supported"),
//         }
//     }
// }
#[derive(Debug, PartialEq, Eq)]
struct Variable<'a> {
    name: &'a str,
}
// #[derive(Deserialize)]
// struct Variable<'a> {
//     name: &'a str, // todo: change to ASTVariable?
//     quantifier: &'a str,
//     marker: &'a str,
// }
#[derive(Debug)]
pub struct TwoVarObligation<'a> {
    src: Variable<'a>,
    dest: Variable<'a>,
}
#[derive(Debug)]
pub enum Influence<'a> {
    FlowsTo(TwoVarObligation<'a>),
    ControlFlow(TwoVarObligation<'a>),
}

// fn func_call(q: &Quantifier) -> &str {
//     match q {
//         Quantifier::Some => "any",
//         Quantifier::All => "all",
//         Quantifier::No => todo!(),
//     }
// }

fn func_call(q: &str) -> &str {
    match q {
        "some" => "any",
        "all" => "all",
        "no" => todo!(),
        &_ => unimplemented!(),
    }
}

fn get_variable_decs(p: &str) -> Table {
    let variables = fs::read_to_string(p).expect("Could not read variables file");
    let tab: Table = variables.parse::<Table>().unwrap();
    // dbg!(&tab);
    tab
}

fn extract_quantifier<'a>(variable_table: &'a Table, var_name: &'a str) -> &'a str {
    match variable_table.get(var_name) {
        Some(table) => match table.get("quantifier") {
            Some(ans) => ans.as_str().unwrap().into(),
            None => panic!("toml parsing inner table failed"),
        },
        None => panic!("toml parsing outer table failed"),
    }
}

fn extract_marker<'a>(variable_table: &'a Table, var_name: &'a str) -> &'a str {
    match variable_table.get(var_name) {
        Some(table) => match table.get("marker") {
            Some(ans) => ans.as_str().unwrap(),
            None => panic!("toml parsing inner table failed"),
        },
        None => panic!("toml parsing outer table failed"),
    }
}

fn fill_in_template<'a>(
    handlebars: &mut Handlebars,
    ob: &Influence<'a>,
    map: &mut HashMap<String, String>,
    variable_table: Table,
) -> Result<()> {
    let (obligation, template) = match ob {
        Influence::FlowsTo(o) => (o, FLOWS_TO_TEMPLATE),
        Influence::ControlFlow(o) => (o, CONTROL_FLOW_TEMPLATE),
    };

    let template_path = match template {
        FLOWS_TO_TEMPLATE => "templates/flows-to.txt",
        CONTROL_FLOW_TEMPLATE => "templates/control-flow.txt",
        &_ => panic!("should not reach this case"),
    };

    handlebars
        .register_template_file(template, template_path)
        .expect("Could not register flows to template with handlebars");

    let src_marker = extract_marker(&variable_table, obligation.src.name);
    let dest_marker = extract_marker(&variable_table, obligation.dest.name);
    let src_quantifier = extract_quantifier(&variable_table, obligation.src.name);
    let dest_quantifier = extract_quantifier(&variable_table, obligation.dest.name);

    map.insert("src_marker".to_string(), src_marker.to_string());
    map.insert("dest_marker".to_string(), dest_marker.to_string());

    map.insert(
        "src_func_call".to_string(),
        func_call(&src_quantifier).to_string(),
    );
    map.insert(
        "dest_func_call".to_string(),
        func_call(&dest_quantifier).to_string(),
    );

    let policy = handlebars
        .render(template, &map)
        .expect("Could not render flows to handlebars template");

    map.insert("policy".to_string(), policy);

    let res = handlebars
        .render(BASE_TEMPLATE, &map)
        .expect("Could not render flows to handlebars template");

    fs::write("compiled-policy.rs", &res)?;
    Ok(())
}

pub fn compile<'a>(obligation: &Influence<'a>, map: &mut HashMap<String, String>) -> Result<()> {
    let mut handlebars = Handlebars::new();

    handlebars.register_escape_fn(no_escape);

    handlebars
        .register_template_file(BASE_TEMPLATE, "templates/base.txt")
        .expect("Could not register base template with handlebars");

    let variable_table: Table = get_variable_decs("variables/flows-to.toml");
    fill_in_template(&mut handlebars, obligation, map, variable_table)
}
