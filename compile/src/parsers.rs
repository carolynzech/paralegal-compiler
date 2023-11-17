use nom::{
    branch::alt,
    bytes::complete::{tag, take_till, take_until, take_while},
    error::{context, VerboseError},
    sequence::tuple,
    IResult,
};

use crate::{Influence, Quantifier, TwoVarObligation, Variable};

pub type Res<T, U> = IResult<T, U, VerboseError<T>>;

static FLOWS_TO_TAG: &str = "flowsto";
static CONTROL_FLOW_TAG: &str = "hascontrolflowinfluenceon";

fn some(s: &str) -> Res<&str, Quantifier> {
    context("some", tag("some"))(s).map(|(remainder, res)| (remainder, res.into()))
}

fn flows_to_phrase(s: &str) -> Res<&str, &str> {
    context("flows to", tag(FLOWS_TO_TAG))(s)
}

fn control_flow_phrase(s: &str) -> Res<&str, &str> {
    context("has control flow influence on", tag(CONTROL_FLOW_TAG))(s)
}

fn marker(s: &str) -> Res<&str, &str> {
    context(
        "marker",
        alt((
            alt((take_until(FLOWS_TO_TAG), take_until(CONTROL_FLOW_TAG))),
            take_while(char::is_alphabetic),
        )),
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

fn flows_to<'a>(s: &'a str) -> Res<&str, Influence<'a>> {
    let mut combinator = tuple((variable, flows_to_phrase, variable));
    let (remainder, res) = combinator(&s)?;

    Ok((
        remainder,
        Influence::FlowsTo(TwoVarObligation {
            src: res.0,
            dest: res.2,
        }),
    ))
}

fn control_flow<'a>(s: &'a str) -> Res<&str, Influence<'a>> {
    let mut combinator = tuple((variable, control_flow_phrase, variable));
    let (remainder, res) = combinator(&s)?;

    Ok((
        remainder,
        Influence::ControlFlow(TwoVarObligation {
            src: res.0,
            dest: res.2,
        }),
    ))
}

pub fn dispatch<'a>(s: &'a str) -> Res<&str, Influence<'a>> {
    alt((flows_to, control_flow))(s)
    // control_flow(s)
}
