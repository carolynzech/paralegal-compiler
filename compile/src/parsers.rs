use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    error::{context, VerboseError},
    multi::many0,
    sequence::{terminated, tuple},
    IResult,
};

use crate::{Influence, TwoVarObligation, Variable};

pub type Res<T, U> = IResult<T, U, VerboseError<T>>;

static FLOWS_TO_TAG: &str = "flows to";
static CONTROL_FLOW_TAG: &str = "has control flow influence on";

fn flows_to_phrase(s: &str) -> Res<&str, &str> {
    context(
        "flows to",
        terminated(tag(FLOWS_TO_TAG), many0(is_nonalphabetic)),
    )(s)
}

fn control_flow_phrase(s: &str) -> Res<&str, &str> {
    context(
        "control flow influence",
        terminated(tag(CONTROL_FLOW_TAG), many0(is_nonalphabetic)),
    )(s)
}

fn is_nonalphabetic(s: &str) -> Res<&str, &str> {
    alt((tag("."), tag(","), tag(" ")))(s)
}

// take while it's alphabetic, and also get rid of any spaces/commas/periods following it
fn variable<'a>(s: &'a str) -> Res<&str, Variable<'a>> {
    let (remainder, res) = context(
        "variable",
        terminated(take_while1(char::is_alphabetic), many0(is_nonalphabetic)),
    )(s)?;
    Ok((remainder, Variable { name: res }))
}

fn flows_to<'a>(s: &'a str) -> Res<&str, Influence<'a>> {
    let mut combinator = tuple((variable, flows_to_phrase, variable));
    // let mut combinator = variable;
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

pub fn parse_expr<'a>(s: &'a str) -> Res<&str, Influence<'a>> {
    alt((flows_to, control_flow))(s)
}

pub fn parse<'a>(s: &'a str) -> Res<&str, Influence<'a>> {
    let res = parse_expr(s);
    dbg!(&res);
    res
}

// and/or expr parser = alt((and parser, or parser)), expr parser
// then <expr> and <logic-expr> parser is tuple(expr parser, many0(and/or expr parser))
// I guess can gather into a vector for now using many0, then iterate over the vector and construct the AST
// every entry in the vector would be the "dest" in a new "And/Or" node, with the previous result as the "src"

/*
    <program> := <logic-expr>
    <logic-expr> := <expr> and <logic-expr> | <expr> or <logic-expr> | <expr>
    <expr> := <var> flows to <var> | <var> has control flow influence on <var>
*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variables() {
        let var1 = "a";
        let var2 = "sensitive";
        let wrong = "123hello";
        let partially_keyword = "a flows to b";

        assert!(variable(var1).is_ok());
        assert!(variable(var2).is_ok());
        assert!(variable(wrong).is_err());
        assert!(variable(partially_keyword) == Ok(("flows to b", Variable { name: "a" })));
    }
}
