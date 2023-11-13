use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while},
    error::{context, VerboseError},
    sequence::tuple,
    IResult,
};

use std::fs;
use std::io::Result;

/* TODOs
    - variable translation needs to be reversed: currently policies written with (quantifier + marker,
        which parse translates to variable, needs to go the other way.
    - better error handling
    - cargo new for the policy and write a boilerplate a Cargo.toml for it as well
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

fn compile_flows_to<'a>(f: &FlowsTo<'a>) -> Result<()> {
    let boilerplate =
        fs::read_to_string("src/boilerplate.txt").expect("Could not read boilerplate code");

    let src_marker = f.src.marker;
    let src_func_call = func_call(&f.src.quantifier);
    let dest_marker = f.dest.marker;
    let dest_func_call = func_call(&f.dest.quantifier);

    let dest_logic = format!("{dest_marker}_nodes.{dest_func_call}(|{dest_marker}| ctx.flows_to({src_marker}, {dest_marker}, EdgeType::Data))))");
    let src_logic = format!("{src_marker}_nodes.{src_func_call}(|{src_marker}| {dest_logic}");

    let policy = format!(
        "{boilerplate}\npolicy!(pol, ctx {{
            let mut {src_marker}_nodes = ctx.marked_nodes(marker!({src_marker}));
            let mut {dest_marker}_nodes = ctx.marked_nodes(marker!({dest_marker}));
            assert_error!(ctx, {src_logic};
            Ok(())
        }});",
    );

    fs::write("compiled-policy.rs", policy)?;
    Ok(())
}

pub fn compile<'a>(f: &FlowsTo<'a>) -> Result<()> {
    compile_flows_to(f)
}
