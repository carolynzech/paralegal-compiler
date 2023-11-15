use handlebars::{no_escape, Handlebars};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while},
    error::{context, VerboseError},
    sequence::tuple,
    IResult,
};
use std::collections::HashMap;
use std::fs;
use std::io::Result;

// template names
const BASE_TEMPLATE: &str = "base";
const FLOWS_TO_TEMPLATE: &str = "flows-to";

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

pub type Res<T, U> = IResult<T, U, VerboseError<T>>;

#[derive(Debug)]
pub enum Quantifier {
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
pub struct Variable<'a> {
    quantifier: Quantifier,
    marker: &'a str,
}

#[derive(Debug)]
pub struct FlowsTo<'a> {
    src: Variable<'a>,
    dest: Variable<'a>,
}

fn func_call(q: &Quantifier) -> &str {
    match q {
        Quantifier::Some => "any",
        Quantifier::All => "all",
        Quantifier::No => todo!(),
    }
}

fn some(s: &str) -> Res<&str, Quantifier> {
    context("some", tag("some"))(s).map(|(remainder, res)| (remainder, res.into()))
}

fn flows_to(s: &str) -> Res<&str, &str> {
    context("flows to", tag("flowsto"))(s)
}

fn marker(s: &str) -> Res<&str, &str> {
    context(
        "marker",
        alt((take_until("flowsto"), take_while(char::is_alphabetic))),
    )(s)
}

fn variable<'a>(s: &'a str) -> Res<&str, Variable<'a>> {
    let (remainder, res) = context("variable", tuple((some, marker)))(s)?;
    Ok((
        remainder,
        Variable {
            quantifier: res.0,
            marker: res.1,
        },
    ))
}

fn construct_flows_to<'a>(s: &'a str) -> Res<&str, FlowsTo<'a>> {
    let mut combinator = tuple((variable, flows_to, variable));
    let (remainder, res) = combinator(&s)?;

    Ok((
        remainder,
        FlowsTo {
            src: res.0,
            dest: res.2,
        },
    ))
}

pub fn parse<'a>(s: &'a str) -> Res<&str, FlowsTo<'a>> {
    let res = construct_flows_to(&s)?;
    dbg!(&res);
    Ok(res)
}

fn compile_flows_to<'a>(
    handlebars: &mut Handlebars,
    flows_to: &FlowsTo<'a>,
    map: &mut HashMap<String, String>,
) -> Result<()> {
    handlebars
        .register_template_file(FLOWS_TO_TEMPLATE, "templates/flows-to.txt")
        .expect("Could not register flows to template with handlebars");

    map.insert("src_marker".to_string(), flows_to.src.marker.to_string());
    map.insert("dest_marker".to_string(), flows_to.dest.marker.to_string());
    map.insert(
        "src_func_call".to_string(),
        func_call(&flows_to.src.quantifier).to_string(),
    );
    map.insert(
        "dest_func_call".to_string(),
        func_call(&flows_to.dest.quantifier).to_string(),
    );

    let policy = handlebars
        .render(FLOWS_TO_TEMPLATE, &map)
        .expect("Could not render flows to handlebars template");

    map.insert("policy".to_string(), policy);

    let res = handlebars
        .render(BASE_TEMPLATE, &map)
        .expect("Could not render flows to handlebars template");

    fs::write("compiled-policy.rs", &res)?;
    Ok(())
}

pub fn compile<'a>(flows_to: &FlowsTo<'a>, map: &mut HashMap<String, String>) -> Result<()> {
    let mut handlebars = Handlebars::new();

    handlebars.register_escape_fn(no_escape);

    handlebars
        .register_template_file(BASE_TEMPLATE, "templates/base.txt")
        .expect("Could not register base template with handlebars");

    compile_flows_to(&mut handlebars, flows_to, map)
}
