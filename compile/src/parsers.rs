use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    combinator::{all_consuming, map},
    error::{context, VerboseError},
    multi::many0,
    sequence::{pair, terminated, tuple},
    IResult,
};

use crate::{ASTNode, ConjunctionData, TwoVarObligation, Variable};

pub type Res<T, U> = IResult<T, U, VerboseError<T>>;

static FLOWS_TO_TAG: &str = "flows to";
static CONTROL_FLOW_TAG: &str = "has control flow influence on";

fn is_nonalphabetic(s: &str) -> Res<&str, Vec<&str>> {
    many0(alt((tag("."), tag(","), tag(" "))))(s)
}

fn flows_to_phrase(s: &str) -> Res<&str, &str> {
    context("flows to", terminated(tag(FLOWS_TO_TAG), is_nonalphabetic))(s)
}

fn control_flow_phrase(s: &str) -> Res<&str, &str> {
    context(
        "control flow influence",
        terminated(tag(CONTROL_FLOW_TAG), is_nonalphabetic),
    )(s)
}

fn and(s: &str) -> Res<&str, &str> {
    context("and", terminated(tag("and"), is_nonalphabetic))(s)
}
fn or(s: &str) -> Res<&str, &str> {
    context("or", terminated(tag("or"), is_nonalphabetic))(s)
}

// take while it's alphabetical, and also get rid of any spaces/commas/periods following it
fn variable<'a>(s: &'a str) -> Res<&str, Variable<'a>> {
    let (remainder, res) = context(
        "variable",
        terminated(take_while1(char::is_alphabetic), is_nonalphabetic),
    )(s)?;
    Ok((remainder, Variable { name: res }))
}

fn flows_to<'a>(s: &'a str) -> Res<&str, ASTNode<'a>> {
    let mut combinator = tuple((variable, flows_to_phrase, variable));
    let (remainder, (var1, _, var2)) = combinator(&s)?;

    Ok((
        remainder,
        ASTNode::FlowsTo(TwoVarObligation {
            src: var1,
            dest: var2,
        }),
    ))
}

fn control_flow<'a>(s: &'a str) -> Res<&str, ASTNode<'a>> {
    let mut combinator = tuple((variable, control_flow_phrase, variable));
    let (remainder, (var1, _, var2)) = combinator(&s)?;

    Ok((
        remainder,
        ASTNode::ControlFlow(TwoVarObligation {
            src: var1,
            dest: var2,
        }),
    ))
}

fn parse_expr<'a>(s: &'a str) -> Res<&str, ASTNode<'a>> {
    alt((flows_to, control_flow))(s)
}

// parse "and/or <expr>"
fn parse_conjunction_expr<'a>(s: &'a str) -> Res<&str, (&'a str, ASTNode<'a>)> {
    let mut combinator = tuple((alt((and, or)), parse_expr));
    let (remainder, (conjunction, expr)) = combinator(&s)?;
    Ok((remainder, (conjunction, expr)))
}

// parse "<expr> and/or <expr> and/or <expr> ..."
fn parse_policy_body<'a>(s: &'a str) -> Res<&str, ASTNode<'a>> {
    all_consuming(map(
        pair(parse_expr, many0(parse_conjunction_expr)),
        |(expr, conj_expr_vec)| {
            conj_expr_vec.into_iter().fold(expr, |acc, (conj, expr)| {
                ASTNode::Conjunction(Box::new(ConjunctionData {
                    typ: conj.into(),
                    src: acc,
                    dest: expr,
                }))
            })
        },
    ))(s)
}

pub fn parse<'a>(s: &'a str) -> Res<&str, ASTNode<'a>> {
    // TODO parse policy let bindings
    let final_parse_res = all_consuming(parse_policy_body)(s);
    dbg!(&final_parse_res);
    final_parse_res
}

#[cfg(test)]
mod tests {
    use crate::Conjunction;

    use super::*;

    // TODO: test other parsers

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

    #[test]
    fn joined_exprs() {
        let policy1 = "a flows to b";
        let policy1_ans = ASTNode::FlowsTo(TwoVarObligation {
            src: Variable { name: "a" },
            dest: Variable { name: "b" },
        });
        let policy2 = "a flows to b and a flows to c";
        let policy2_ans = ASTNode::Conjunction(Box::new(ConjunctionData {
            typ: Conjunction::And,
            src: ASTNode::FlowsTo(TwoVarObligation {
                src: Variable { name: "a" },
                dest: Variable { name: "b" },
            }),
            dest: ASTNode::FlowsTo(TwoVarObligation {
                src: Variable { name: "a" },
                dest: Variable { name: "c" },
            }),
        }));
        let policy3 = "a has control flow influence on b or a flows to c and b flows to c";
        let policy3_ans = ASTNode::Conjunction(Box::new(ConjunctionData {
            typ: Conjunction::And,
            src: ASTNode::Conjunction(Box::new(ConjunctionData {
                typ: Conjunction::Or,
                src: ASTNode::ControlFlow(TwoVarObligation {
                    src: Variable { name: "a" },
                    dest: Variable { name: "b" },
                }),
                dest: ASTNode::FlowsTo(TwoVarObligation {
                    src: Variable { name: "a" },
                    dest: Variable { name: "c" },
                }),
            })),
            dest: ASTNode::FlowsTo(TwoVarObligation {
                src: Variable { name: "b" },
                dest: Variable { name: "c" },
            }),
        }));

        let err1 = "a flows to b or";
        let err2 = "a flows to b or b flows to";

        assert!(parse_policy_body(policy1) == Ok(("", policy1_ans)));
        assert!(parse_policy_body(policy2) == Ok(("", policy2_ans)));
        assert!(parse_policy_body(policy3) == Ok(("", policy3_ans)));
        assert!(parse_policy_body(err1).is_err());
        assert!(parse_policy_body(err2).is_err());
    }
}
