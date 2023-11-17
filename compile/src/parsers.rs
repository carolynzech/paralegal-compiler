use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while},
    error::{context, VerboseError},
    sequence::tuple,
    IResult,
};

use crate::{ASTVariable, Influence, TwoVarObligation};

pub type Res<T, U> = IResult<T, U, VerboseError<T>>;

static FLOWS_TO_TAG: &str = "flowsto";
static CONTROL_FLOW_TAG: &str = "hascontrolflowinfluenceon";

fn flows_to_phrase(s: &str) -> Res<&str, &str> {
    context("flows to", tag(FLOWS_TO_TAG))(s)
}

fn control_flow_phrase(s: &str) -> Res<&str, &str> {
    context("has control flow influence on", tag(CONTROL_FLOW_TAG))(s)
}

fn variable<'a>(s: &'a str) -> Res<&str, ASTVariable<'a>> {
    let (remainder, res) = context(
        "marker",
        alt((
            alt((take_until(FLOWS_TO_TAG), take_until(CONTROL_FLOW_TAG))),
            take_while(char::is_alphabetic),
        )),
    )(s)?;
    Ok((remainder, ASTVariable { name: res }))
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
}
