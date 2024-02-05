use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::char,
    combinator::{all_consuming, eof, map, not, opt, recognize},
    error::{context, VerboseError},
    multi::{many0, many1},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    IResult,
};

use crate::{
    ASTNode, Conjunction, TwoNodeObligation, PolicyBody, PolicyScope, Quantifier, ThreeVarObligation, TwoVarObligation, Variable, VariableBinding
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

fn colon(s: &str) -> Res<&str, &str> {
    context("colon", tag(":"))(s)
}

fn flows_to(s: &str) -> Res<&str, &str> {
    context("flows to", terminated(tag(FLOWS_TO_TAG), is_nonalphabetic))(s)
}

fn control_flow(s: &str) -> Res<&str, &str> {
    context(
        "control flow",
        terminated(tag(CONTROL_FLOW_TAG), is_nonalphabetic),
    )(s)
}

fn through(s: &str) -> Res<&str, &str> {
    context("through", terminated(tag("through"), is_nonalphabetic))(s)
}

fn always(s: &str) -> Res<&str, &str> {
    context(
        "always",
        terminated(terminated(tag("always"), colon), is_nonalphabetic),
    )(s)
}

fn sometimes(s: &str) -> Res<&str, &str> {
    context(
        "sometimes",
        terminated(terminated(tag("sometimes"), colon), is_nonalphabetic),
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
        recognize(many1(tuple((
            take_while1(char::is_alphabetic),
            opt(char('_')),
        )))),
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

fn flows_to_expr<'a>(s: &'a str) -> Res<&str, ASTNode<'a>> {
    let mut combinator = context("flows to expr", tuple((variable, flows_to, variable)));
    let (remainder, (var1, _, var2)) = combinator(s)?;

    Ok((
        remainder,
        ASTNode::FlowsTo(TwoVarObligation {
            src: var1,
            dest: var2,
        }),
    ))
}

fn through_expr<'a>(s: &'a str) -> Res<&str, ASTNode<'a>> {
    let mut combinator = context(
        "through expr",
        separated_pair(flows_to_expr, through, variable),
    );
    let (remainder, (flows_to, checkpoint)) = combinator(s)?;

    match flows_to {
        ASTNode::FlowsTo(obligation) => {
            Ok((
                remainder,
                ASTNode::Through(ThreeVarObligation {
                    src: obligation.src,
                    dest: obligation.dest,
                    checkpoint,
                }),
            ))
        }
        _ => panic!("shouldn't reach this case; flows_to combinator should have failed")
    } 
}

// first tries to parse through expressions, then regular flows to if through fails
fn flows_to_or_through_expr<'a>(s: &'a str) -> Res<&str, ASTNode<'a>> {
    context(
        "flows to or through expr",
        alt((through_expr, terminated(flows_to_expr, not(through)))),
    )(s)
}

fn control_flow_expr<'a>(s: &'a str) -> Res<&str, ASTNode<'a>> {
    let mut combinator = context(
        "control flow expr",
        tuple((variable, control_flow, variable)),
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
    context(
        "parse expr",
        alt((control_flow_expr, flows_to_or_through_expr)),
    )(s)
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
                        let data : Box<TwoNodeObligation<'a>> = Box::new(
                            TwoNodeObligation {
                                src: acc,
                                dest: next_expr,
                            });
                        let conj_type : Conjunction = conj.into();
                        match conj_type {
                            And => ASTNode::And(data),
                            Or => ASTNode::Or(data),
                        }
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
        ASTNode::Conditional(Box::new(
            TwoNodeObligation {
                src: premise,
                dest: obligation,
            })),
    ))
}

fn scope(s: &str) -> Res<&str, PolicyScope> {
    let mut combinator = context("scope", alt((always, sometimes)));
    let (remainder, res) = combinator(s)?;

    Ok((remainder, res.into()))
}

fn body_helper<'a>(s: &'a str) -> Res<&str, PolicyBody<'a>> {
    let mut combinator = context(
        "parse body helper",
        tuple((scope, alt((conditional, chained_exprs)))),
    );
    let (remainder, (scope, body)) = combinator(s)?;

    Ok((remainder, PolicyBody { scope, body }))
}

pub fn parse_body<'a>(s: &'a str) -> Res<&str, PolicyBody<'a>> {
    context("parse body", all_consuming(body_helper))(s)
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

    #[test]
    fn test_is_nonalphabetic() {
        let spaces = "     ";
        let comma = ",";
        let period = ".";
        let newline = "\n";
        let punc = ",.\n";
        let err = "this is alphabetical";

        assert_eq!(is_nonalphabetic(spaces), Ok(("", spaces)));
        assert_eq!(is_nonalphabetic(comma), Ok(("", comma)));
        assert_eq!(is_nonalphabetic(period), Ok(("", period)));
        assert_eq!(is_nonalphabetic(newline), Ok(("", newline)));
        assert_eq!(is_nonalphabetic(punc), Ok(("", punc)));
        assert!(is_nonalphabetic(err).is_err());
    }

    #[test]
    fn test_marker() {
        let a = "\"a\"";
        let b = "\"sensitive\"";
        let err1 = "sensitive";
        let err2 = "\"sensitive";

        assert_eq!(marker(a), Ok(("", "a")));
        assert_eq!(marker(b), Ok(("", "sensitive")));
        assert!(marker(err1).is_err());
        assert!(marker(err2).is_err());
    }

    #[test]
    fn test_variable() {
        let var1 = "a";
        let var2 = "sensitive";
        let wrong = "123hello";
        let partially_keyword = "a flows to b";

        assert_eq!(variable(var1), Ok(("", Variable { name: "a" })));
        assert_eq!(variable(var2), Ok(("", Variable { name: "sensitive" })));
        assert_eq!(
            variable(partially_keyword),
            Ok(("flows to b", Variable { name: "a" }))
        );
        assert!(variable(wrong).is_err());
    }

    #[test]
    fn test_expr() {
        let through = "a flows to b through c";
        let through_ans = ASTNode::Through(ThreeVarObligation {
            src: Variable { name: "a" },
            dest: Variable { name: "b" },
            checkpoint: Variable { name: "c" },
        });

        let flows_to = "a flows to b";
        let flows_to_ans = ASTNode::FlowsTo(TwoVarObligation {
            src: Variable { name: "a" },
            dest: Variable { name: "b" },
        });
        let control_flow = "a has control flow influence on b";
        let control_flow_ans = ASTNode::ControlFlow(TwoVarObligation {
            src: Variable { name: "a" },
            dest: Variable { name: "b" },
        });

        let err1 = "a flows to";
        let err2 = "a flows to b through";
        let err3 = "a has control flow influence on";

        assert_eq!(expr(through), Ok(("", through_ans)));
        assert_eq!(expr(flows_to), Ok(("", flows_to_ans)));
        assert_eq!(expr(control_flow), Ok(("", control_flow_ans)));
        assert!(expr(err1).is_err());
        assert!(expr(err2).is_err());
        assert!(expr(err3).is_err());
    }

    #[test]
    fn test_chained_exprs() {
        let policy1 = "a flows to b";
        let policy1_ans = ASTNode::FlowsTo(TwoVarObligation {
            src: Variable { name: "a" },
            dest: Variable { name: "b" },
        });
        let policy2 = "a flows to b and a flows to c";
        let policy2_ans = ASTNode::And(Box::new(TwoNodeObligation {
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
        let policy3_ans = ASTNode::And(Box::new(TwoNodeObligation {
            src: ASTNode::Or(Box::new(TwoNodeObligation {
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

        assert_eq!(chained_exprs(policy1), Ok(("", policy1_ans)));
        assert_eq!(chained_exprs(policy2), Ok(("", policy2_ans)));
        assert_eq!(chained_exprs(policy3), Ok(("", policy3_ans)));
        assert!(chained_exprs(err1).is_err());
    }

    #[test]
    fn test_conditional() {
        let policy1 = "if a flows to b, then c flows to d";
        let policy1_ans = ASTNode::Conditional(Box::new(TwoNodeObligation {
            src: ASTNode::FlowsTo(TwoVarObligation {
                src: Variable { name: "a" },
                dest: Variable { name: "b" },
            }),
            dest: ASTNode::FlowsTo(TwoVarObligation {
                src: Variable { name: "c" },
                dest: Variable { name: "d" },
            }),
        }));
        let policy2 = "if a flows to b and b flows to c, then c has control flow influence on d";
        let policy2_ans = ASTNode::Conditional(Box::new(TwoNodeObligation {
            src: ASTNode::And(Box::new(TwoNodeObligation {
                src: ASTNode::FlowsTo(TwoVarObligation {
                    src: Variable { name: "a" },
                    dest: Variable { name: "b" },
                }),
                dest: ASTNode::FlowsTo(TwoVarObligation {
                    src: Variable { name: "b" },
                    dest: Variable { name: "c" },
                }),
            })),
            dest: ASTNode::ControlFlow(TwoVarObligation {
                src: Variable { name: "c" },
                dest: Variable { name: "d" },
            }),
        }));
        let err1 = "a flows to b";
        let err2 = "if a flows to b";
        let err3 = "a flows to b then";

        assert_eq!(conditional(policy1), Ok(("", policy1_ans)));
        assert_eq!(conditional(policy2), Ok(("", policy2_ans)));
        assert!(conditional(err1).is_err());
        assert!(conditional(err2).is_err());
        assert!(conditional(err3).is_err());
    }

    #[test]
    fn test_body() {
        // TODO add more robust tests
        // at some point the paper policy tests should make their way in here
        // or at least ones approximating their functionality
        let lemmy_comm = "always:
        if community_struct flows to write,
        then community_struct flows to delete_check and 
        delete_check has control flow influence on write and
        community_struct flows to ban_check and
        ban_check has control flow influence on write";

        let lemmy_comm_ans = PolicyBody {
            scope: PolicyScope::Always,
            body: ASTNode::Conditional(Box::new(TwoNodeObligation {
                src: ASTNode::FlowsTo(TwoVarObligation {
                    src: Variable {
                        name: "community_struct",
                    },
                    dest: Variable { name: "write" },
                }),
                dest: ASTNode::And(Box::new(TwoNodeObligation {
                    src: ASTNode::And(Box::new(TwoNodeObligation {
                        src: ASTNode::And(Box::new(TwoNodeObligation {
                            src: ASTNode::FlowsTo(TwoVarObligation {
                                src: Variable {
                                    name: "community_struct",
                                },
                                dest: Variable {
                                    name: "delete_check",
                                },
                            }),
                            dest: ASTNode::ControlFlow(TwoVarObligation {
                                src: Variable {
                                    name: "delete_check",
                                },
                                dest: Variable { name: "write" },
                            }),
                        })),
                        dest: ASTNode::FlowsTo(TwoVarObligation {
                            src: Variable {
                                name: "community_struct",
                            },
                            dest: Variable { name: "ban_check" },
                        }),
                    })),
                    dest: ASTNode::ControlFlow(TwoVarObligation {
                        src: Variable { name: "ban_check" },
                        dest: Variable { name: "write" },
                    }),
                })),
            })),
        };

        let err1 = "a flows to b or b flows to";
        // can only have one, top-level conditionals as of now; this test may change in the future
        let err2 = "if a flows to b and if b flows to c, then d flows to e";
        let err3 = "a flows to b and a flows to";

        assert_eq!(parse_body(lemmy_comm), Ok(("", lemmy_comm_ans)));

        assert!(parse_body(err1).is_err());
        assert!(parse_body(err2).is_err());
        assert!(parse_body(err3).is_err());
    }

    #[test]
    fn test_alphabetic_w_underscores() {
        let no_underscores = "var";
        let one_underscore = "hello_world";
        let two_underscores = "community_delete_check";
        let trailing_underscore = "hello_world_";
        let five_underscores = "this_is_a_long_variable";

        assert_eq!(
            alphabetic_w_underscores(no_underscores),
            Ok(("", no_underscores))
        );
        assert_eq!(
            alphabetic_w_underscores(one_underscore),
            Ok(("", one_underscore))
        );
        assert_eq!(
            alphabetic_w_underscores(two_underscores),
            Ok(("", two_underscores))
        );
        assert_eq!(
            alphabetic_w_underscores(trailing_underscore),
            Ok(("", trailing_underscore))
        );
        assert_eq!(
            alphabetic_w_underscores(five_underscores),
            Ok(("", five_underscores))
        );

        // these are errors for now, but don't need to be
        let leading_underscore = "_hello_world";
        let two_consec_underscores = "multiple__underscores";
        assert!(alphabetic_w_underscores(leading_underscore).is_err());
        assert!(all_consuming(alphabetic_w_underscores)(two_consec_underscores).is_err());
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
        let binding3 = "let delete_check = some \"community_delete_check\"\n        ";
        let binding3_ans = VariableBinding {
            variable: Variable {
                name: "delete_check",
            },
            quantifier: Quantifier::Some,
            marker: "community_delete_check",
        };

        let var_in_quotes = "let \"a\" = some \"a\"";
        let wrong_quantifier = "let a = any \"a\"";

        assert_eq!(single_binding(binding1), Ok(("", binding1_ans)));
        assert_eq!(single_binding(binding2), Ok(("", binding2_ans)));
        assert_eq!(single_binding(binding3), Ok(("", binding3_ans)));
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
        let lemmy_comm = "let community_struct = some \"community\"
        let delete_check = some \"community_delete_check\"
        let ban_check = some \"community_ban_check\"
        let write = some \"db_write\"";
        let lemmy_comm_ans = vec![
            VariableBinding {
                variable: Variable {
                    name: "community_struct",
                },
                quantifier: Quantifier::Some,
                marker: "community",
            },
            VariableBinding {
                variable: Variable {
                    name: "delete_check",
                },
                quantifier: Quantifier::Some,
                marker: "community_delete_check",
            },
            VariableBinding {
                variable: Variable { name: "ban_check" },
                quantifier: Quantifier::Some,
                marker: "community_ban_check",
            },
            VariableBinding {
                variable: Variable { name: "write" },
                quantifier: Quantifier::Some,
                marker: "db_write",
            },
        ];

        let not_separated = "let commit = some \"commit\"let store = some \"sink\"";

        assert_eq!(parse_bindings(single_w_spaces), Ok(("", single_ans)));
        assert_eq!(parse_bindings(multi_newline), Ok(("", multi_ans.clone())));
        assert_eq!(parse_bindings(multi_comma), Ok(("", multi_ans)));
        assert_eq!(parse_bindings(lemmy_comm), Ok(("", lemmy_comm_ans)));
        assert!(parse_bindings(not_separated).is_err());
    }
}
