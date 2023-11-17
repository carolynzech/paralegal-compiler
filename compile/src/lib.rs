use handlebars::{no_escape, Handlebars};
use parsers::{dispatch, Res};
use std::collections::HashMap;
use std::fs;
use std::io::Result;

pub mod parsers;

// template names
const BASE_TEMPLATE: &str = "base";
const FLOWS_TO_TEMPLATE: &str = "flows-to";
const CONTROL_FLOW_TEMPLATE: &str = "control-flow";

/* TODOs
    (Paralegal Functionality)
    - variable translation needs to be reversed: currently policies written with (quantifier + marker,
        which parse translates to variable, needs to go the other way.

    (Good Practice / User Experience / Nits)
    - better error handling
    - pass template file paths as arguments instead of string literals
    - escaping {{}} in Rust code w/o overwriting no-escape for HTML characters
    - cargo new for the policy and write a template a Cargo.toml for it as well
*/

#[derive(Debug)]
enum Quantifier {
    Some,
    All,
    No,
}

impl From<&str> for Quantifier {
    fn from(s: &str) -> Self {
        match s {
            "some" => Quantifier::Some,
            "all" => Quantifier::All,
            "no" => Quantifier::No,
            &_ => unimplemented!("no other quantifiers supported"),
        }
    }
}

#[derive(Debug)]
struct Variable<'a> {
    quantifier: Quantifier,
    marker: &'a str,
}

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

fn func_call(q: &Quantifier) -> &str {
    match q {
        Quantifier::Some => "any",
        Quantifier::All => "all",
        Quantifier::No => todo!(),
    }
}

pub fn parse<'a>(s: &'a str) -> Res<&str, Influence<'a>> {
    let res = dispatch(&s)?;
    dbg!(&res);
    Ok(res)
}

fn fill_in_template<'a>(
    handlebars: &mut Handlebars,
    ob: &Influence<'a>,
    map: &mut HashMap<String, String>,
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

    map.insert("src_marker".to_string(), obligation.src.marker.to_string());
    map.insert(
        "dest_marker".to_string(),
        obligation.dest.marker.to_string(),
    );
    map.insert(
        "src_func_call".to_string(),
        func_call(&obligation.src.quantifier).to_string(),
    );
    map.insert(
        "dest_func_call".to_string(),
        func_call(&obligation.dest.quantifier).to_string(),
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

    fill_in_template(&mut handlebars, obligation, map)
}
