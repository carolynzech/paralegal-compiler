use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while},
    error::{context, VerboseError},
    sequence::tuple,
    IResult,
};

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
    left: Variable<'a>,
    right: Variable<'a>,
}

// todo: policy should use variables instead of quantifiers everywhere

pub fn some(s: &str) -> Res<&str, Quantifier> {
    context("some", tag("some"))(s).map(|(remainder, res)| (remainder, res.into()))
}

pub fn flows_to(s: &str) -> Res<&str, &str> {
    context("flows to", tag("flowsto"))(s)
}

pub fn marker(s: &str) -> Res<&str, &str> {
    context(
        "marker",
        alt((take_until("flowsto"), take_while(char::is_alphabetic))),
    )(s)
}

pub fn variable<'a>(s: &'a str) -> Res<&str, Variable<'a>> {
    let (remainder, res) = context("variable", tuple((some, marker)))(s)?;
    Ok((
        remainder,
        Variable {
            quantifier: res.0,
            marker: res.1,
        },
    ))
}

pub fn construct_flows_to<'a>(s: &'a str) -> Res<&str, FlowsTo<'a>> {
    let mut combinator = tuple((variable, flows_to, variable));
    let (remainder, res) = combinator(&s)?;

    Ok((
        remainder,
        FlowsTo {
            left: res.0,
            right: res.2,
        },
    ))
}

pub fn parse<'a>(s: &'a str) -> Res<&str, FlowsTo<'a>> {
    let res = construct_flows_to(&s);
    dbg!(&res);
    res
}
