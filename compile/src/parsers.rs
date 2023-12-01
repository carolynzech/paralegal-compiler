use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::char,
    combinator::{all_consuming, eof, map, opt, recognize},
    error::{context, VerboseError},
    multi::{many0, many1},
    sequence::{delimited, pair, preceded, terminated, tuple},
    IResult,
};

use crate::{
    ASTNode, ConditionalData, ConjunctionData, Quantifier, TwoVarObligation, Variable,
    VariableBinding,
};

pub type Res<T, U> = IResult<T, U, VerboseError<T>>;

static FLOWS_TO_TAG: &str = "flows to";
static CONTROL_FLOW_TAG: &str = "has control flow influence on";

// TODO: may want to make this more specific -- kind of weird to allow two commas, random newlines, etc.,
// also certain punctuation should only be allowed in certain places (e.g., periods)
fn is_nonalphabetic(s: &str) -> Res<&str, &str> {
    let (remainder, res) = context(
        "is nonalphabetic",
        alt((
            eof,
            recognize(many1(alt((tag("."), tag(","), tag(" "), tag("\n"))))),
        )),
    )(s)?;
    Ok((remainder, res))
}

fn flows_to_phrase(s: &str) -> Res<&str, &str> {
    context(
        "flows to phrase",
        terminated(tag(FLOWS_TO_TAG), is_nonalphabetic),
    )(s)
}

fn control_flow_phrase(s: &str) -> Res<&str, &str> {
    context(
        "control flow phrase",
        terminated(tag(CONTROL_FLOW_TAG), is_nonalphabetic),
    )(s)
}

fn _if(s: &str) -> Res<&str, &str> {
    context("if", terminated(tag("if"), is_nonalphabetic))(s)
}

fn then(s: &str) -> Res<&str, &str> {
    context("then", terminated(tag("then"), is_nonalphabetic))(s)
}

fn and(s: &str) -> Res<&str, &str> {
    context("and", terminated(tag("and"), is_nonalphabetic))(s)
}

fn or(s: &str) -> Res<&str, &str> {
    context("or", terminated(tag("or"), is_nonalphabetic))(s)
}

fn _let(s: &str) -> Res<&str, &str> {
    context("let", terminated(tag("let"), is_nonalphabetic))(s)
}

fn equal(s: &str) -> Res<&str, &str> {
    context("equal", terminated(tag("="), is_nonalphabetic))(s)
}

fn some(s: &str) -> Res<&str, Quantifier> {
    let mut combinator = context("some", terminated(tag("some"), is_nonalphabetic));
    let (remainder, _) = combinator(s)?;

    Ok((remainder, Quantifier::Some))
}

fn all(s: &str) -> Res<&str, Quantifier> {
    let mut combinator = context("all", terminated(tag("all"), is_nonalphabetic));
    let (remainder, _) = combinator(s)?;

    Ok((remainder, Quantifier::All))
}

fn quantifier(s: &str) -> Res<&str, Quantifier> {
    context("quantifier", alt((some, all)))(s)
}

fn alphabetic_w_underscores(s: &str) -> Res<&str, &str> {
    let mut combinator = context(
        "alphabetic w/ underscores",
        recognize(tuple((
            opt(char('_')),
            take_while1(char::is_alphabetic),
            opt(char('_')),
            opt(take_while1(char::is_alphabetic)),
        ))),
    );
    let (remainder, res) = combinator(s)?;
    Ok((remainder, res))
}

fn marker<'a>(s: &'a str) -> Res<&str, &'a str> {
    let (remainder, res) = context(
        "marker",
        terminated(
            delimited(tag("\""), alphabetic_w_underscores, tag("\"")),
            is_nonalphabetic,
        ),
    )(s)?;
    Ok((remainder, res))
}

fn variable<'a>(s: &'a str) -> Res<&str, Variable<'a>> {
    let (remainder, res) = context(
        "variable",
        terminated(alphabetic_w_underscores, is_nonalphabetic),
    )(s)?;
    Ok((remainder, Variable { name: res }))
}

fn flows_to<'a>(s: &'a str) -> Res<&str, ASTNode<'a>> {
    let mut combinator = context(
        "flows to expr",
        tuple((variable, flows_to_phrase, variable)),
    );
    let (remainder, (var1, _, var2)) = combinator(s)?;

    Ok((
        remainder,
        ASTNode::FlowsTo(TwoVarObligation {
            src: var1,
            dest: var2,
        }),
    ))
}

fn control_flow<'a>(s: &'a str) -> Res<&str, ASTNode<'a>> {
    let mut combinator = context(
        "control flow expr",
        tuple((variable, control_flow_phrase, variable)),
    );
    let (remainder, (var1, _, var2)) = combinator(s)?;

    Ok((
        remainder,
        ASTNode::ControlFlow(TwoVarObligation {
            src: var1,
            dest: var2,
        }),
    ))
}

fn expr<'a>(s: &'a str) -> Res<&str, ASTNode<'a>> {
    context("parse expr", alt((flows_to, control_flow)))(s)
}

// parse "and/or <expr>"
fn conjunction_expr<'a>(s: &'a str) -> Res<&str, (&'a str, ASTNode<'a>)> {
    let mut combinator = context("parse conjunction expr", tuple((alt((and, or)), expr)));
    let (remainder, (conjunction, expr)) = combinator(s)?;
    Ok((remainder, (conjunction, expr)))
}

// parse "<expr> and/or <expr> and/or <expr> ..."
fn chained_exprs<'a>(s: &'a str) -> Res<&str, ASTNode<'a>> {
    context(
        "parse chained expressions",
        map(
            pair(expr, many0(conjunction_expr)),
            |(first_expr, conj_expr_vec)| {
                conj_expr_vec
                    .into_iter()
                    .fold(first_expr, |acc, (conj, next_expr)| {
                        ASTNode::Conjunction(Box::new(ConjunctionData {
                            typ: conj.into(),
                            src: acc,
                            dest: next_expr,
                        }))
                    })
            },
        ),
    )(s)
}

fn conditional<'a>(s: &'a str) -> Res<&str, ASTNode<'a>> {
    let mut combinator = context(
        "parse conditionals",
        tuple((preceded(_if, chained_exprs), preceded(then, chained_exprs))),
    );
    let (remainder, (premise, obligation)) = combinator(s)?;
    Ok((
        remainder,
        ASTNode::Conditional(Box::new(ConditionalData {
            premise,
            obligation,
        })),
    ))
}

fn do_parse<'a>(s: &'a str) -> Res<&str, ASTNode<'a>> {
    context("parse body helper", alt((conditional, chained_exprs)))(s)
}

pub fn parse_body<'a>(s: &'a str) -> Res<&str, ASTNode<'a>> {
    context("parse body", all_consuming(do_parse))(s)
}

fn single_binding<'a>(s: &'a str) -> Res<&str, VariableBinding<'a>> {
    let mut combinator = context(
        "parse single binding",
        tuple((
            preceded(_let, variable),
            preceded(equal, quantifier),
            marker,
        )),
    );
    let (remainder, (variable, quantifier, marker)) = combinator(s)?;

    Ok((
        remainder,
        VariableBinding {
            variable,
            quantifier,
            marker,
        },
    ))
}

// parse let bindings
pub fn parse_bindings<'a>(s: &'a str) -> Res<&str, Vec<VariableBinding<'a>>> {
    context("parse bindings", many1(single_binding))(s)
}

#[cfg(test)]
mod tests {
    use crate::Conjunction;

    use super::*;

    // TODO: test other parsers

    #[test]
    fn test_marker() {
        let a = "\"a\"";
        let b = "\"sensitive\"";
        let err1 = "sensitive";
        let err2 = "\"sensitive";

        assert!(marker(a) == Ok(("", "a")));
        assert!(marker(b) == Ok(("", "sensitive")));
        assert!(marker(err1).is_err());
        assert!(marker(err2).is_err());
    }

    #[test]
    fn test_variable() {
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
    fn test_chained_exprs() {
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

        let err1 = "a flows to";

        assert!(chained_exprs(policy1) == Ok(("", policy1_ans)));
        assert!(chained_exprs(policy2) == Ok(("", policy2_ans)));
        assert!(chained_exprs(policy3) == Ok(("", policy3_ans)));
        assert!(chained_exprs(err1).is_err());
    }

    #[test]
    fn test_conditional() {
        let policy1 = "if a flows to b, then c flows to d";
        let policy1_ans = ASTNode::Conditional(Box::new(ConditionalData {
            premise: ASTNode::FlowsTo(TwoVarObligation {
                src: Variable { name: "a" },
                dest: Variable { name: "b" },
            }),
            obligation: ASTNode::FlowsTo(TwoVarObligation {
                src: Variable { name: "c" },
                dest: Variable { name: "d" },
            }),
        }));
        let policy2 = "if a flows to b and b flows to c, then c has control flow influence on d";
        let policy2_ans = ASTNode::Conditional(Box::new(ConditionalData {
            premise: ASTNode::Conjunction(Box::new(ConjunctionData {
                typ: Conjunction::And,
                src: ASTNode::FlowsTo(TwoVarObligation {
                    src: Variable { name: "a" },
                    dest: Variable { name: "b" },
                }),
                dest: ASTNode::FlowsTo(TwoVarObligation {
                    src: Variable { name: "b" },
                    dest: Variable { name: "c" },
                }),
            })),
            obligation: ASTNode::ControlFlow(TwoVarObligation {
                src: Variable { name: "c" },
                dest: Variable { name: "d" },
            }),
        }));
        let err1 = "a flows to b";
        let err2 = "if a flows to b";
        let err3 = "a flows to b then";

        assert!(conditional(policy1) == Ok(("", policy1_ans)));
        assert!(conditional(policy2) == Ok(("", policy2_ans)));
        assert!(conditional(err1).is_err());
        assert!(conditional(err2).is_err());
        assert!(conditional(err3).is_err());
    }

    #[test]
    fn test_body() {
        // TODO add more robust tests
        // at some point the paper policy tests should make their way in here
        // or at least ones approximating their functionality

        let err1 = "a flows to b or b flows to";
        // can only have one, top-level conditionals as of now; this test may change in the future
        let err2 = "if a flows to b and if b flows to c, then d flows to e";
        let err3 = "a flows to b and a flows to";

        assert!(parse_body(err1).is_err());
        assert!(parse_body(err2).is_err());
        assert!(parse_body(err3).is_err());
    }

    #[test]
    fn test_single_binding() {
        let binding1 = "let a = some \"a\"";
        let binding1_ans = VariableBinding {
            variable: Variable { name: "a" },
            quantifier: Quantifier::Some,
            marker: "a",
        };
        let binding2 = "let sens = all \"sensitive\"";
        let binding2_ans = VariableBinding {
            variable: Variable { name: "sens" },
            quantifier: Quantifier::All,
            marker: "sensitive",
        };

        let var_in_quotes = "let \"a\" = some \"a\"";
        let wrong_quantifier = "let a = any \"a\"";

        assert!(single_binding(binding1) == Ok(("", binding1_ans)));
        assert!(single_binding(binding2) == Ok(("", binding2_ans)));
        assert!(single_binding(var_in_quotes).is_err());
        assert!(single_binding(wrong_quantifier).is_err());
    }

    #[test]
    fn test_bindings() {
        let single_w_spaces = "let sens    = all \"sensitive\"   \n";
        let single_ans = vec![VariableBinding {
            variable: Variable { name: "sens" },
            quantifier: Quantifier::All,
            marker: "sensitive",
        }];
        let multi_newline = "let commit = some \"commit\"\nlet store = some \"sink\"\nlet auth_check = all \"check_rights\"\n";
        let multi_comma = "let commit = some \"commit\", let store = some \"sink\", let auth_check = all \"check_rights\"\n";
        let multi_ans = vec![
            VariableBinding {
                variable: Variable { name: "commit" },
                quantifier: Quantifier::Some,
                marker: "commit",
            },
            VariableBinding {
                variable: Variable { name: "store" },
                quantifier: Quantifier::Some,
                marker: "sink",
            },
            VariableBinding {
                variable: Variable { name: "auth_check" },
                quantifier: Quantifier::All,
                marker: "check_rights",
            },
        ];

        let not_separated = "let commit = some \"commit\"let store = some \"sink\"";

        assert!(parse_bindings(single_w_spaces) == Ok(("", single_ans)));
        assert!(parse_bindings(multi_newline) == Ok(("", multi_ans.clone())));
        assert!(parse_bindings(multi_comma) == Ok(("", multi_ans)));
        assert!(parse_bindings(not_separated).is_err());
    }
}
