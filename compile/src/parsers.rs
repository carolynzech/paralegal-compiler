use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{char, multispace0},
    combinator::{all_consuming, map, not, opt, recognize},
    error::{context, VerboseError},
    multi::{many0, many1},
    sequence::{delimited, pair, separated_pair, terminated, tuple},
    IResult,
};

use crate::{
    ASTNode, TermLink, TwoNodeObligation, PolicyBody, PolicyScope, Quantifier, ThreeVarObligation, TwoVarObligation, Variable, VariableBinding, VariableClause
};

pub type Res<T, U> = IResult<T, U, VerboseError<T>>;

static FLOWS_TO_TAG: &str = "flows to";
static CONTROL_FLOW_TAG: &str = "has control flow influence on";

fn colon(s: &str) -> Res<&str, &str> {
    context("colon", tag(":"))(s)
}

fn flows_to(s: &str) -> Res<&str, &str> {
    context("flows to", terminated(tag(FLOWS_TO_TAG), multispace0))(s)
}

fn control_flow(s: &str) -> Res<&str, &str> {
    context(
        "control flow",
        terminated(tag(CONTROL_FLOW_TAG), multispace0),
    )(s)
}

fn through(s: &str) -> Res<&str, &str> {
    context("through", terminated(tag("through"), multispace0))(s)
}

fn always(s: &str) -> Res<&str, &str> {
    context(
        "always",
        terminated(terminated(tag("always"), colon), multispace0),
    )(s)
}

fn sometimes(s: &str) -> Res<&str, &str> {
    context(
        "sometimes",
        terminated(terminated(tag("sometimes"), colon), multispace0),
    )(s)
}

fn and(s: &str) -> Res<&str, &str> {
    context("and", terminated(tag("and"), multispace0))(s)
}

fn or(s: &str) -> Res<&str, &str> {
    context("or", terminated(tag("or"), multispace0))(s)
}

fn implies(s: &str) -> Res<&str, &str> {
    context("implies", terminated(tag("implies"), multispace0))(s)
}

fn open_paren(s: &str) -> Res<&str, &str> {
    context("open paren", terminated(tag("("), multispace0))(s)
}

fn close_paren(s: &str) -> Res<&str, &str> {
    context("close paren", terminated(tag(")"), multispace0))(s)
}

fn some(s: &str) -> Res<&str, Quantifier> {
    let mut combinator = context("some", terminated(tag("some"), multispace0));
    let (remainder, _) = combinator(s)?;

    Ok((remainder, Quantifier::Some))
}

fn all(s: &str) -> Res<&str, Quantifier> {
    let mut combinator = context("all", terminated(tag("all"), multispace0));
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
            multispace0,
        ),
    )(s)?;
    Ok((remainder, res))
}

fn variable<'a>(s: &'a str) -> Res<&str, Variable<'a>> {
    let (remainder, res) = context(
        "variable",
        terminated(alphabetic_w_underscores, multispace0),
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

fn leaf_expr<'a>(s: &'a str) -> Res<&str, ASTNode<'a>> {
    context(
        "parse expr",
        alt((control_flow_expr, flows_to_or_through_expr)),
    )(s)
}

// parse "and/or/implies <leaf expr>"
fn and_or_implies<'a>(s: &'a str) -> Res<&str, &'a str> {
    let mut combinator = context("parse conjunction expr", alt((and, or, implies)));
    let (remainder, term) = combinator(s)?;
    Ok((remainder, term))
}

/* parse <leaf> and/or/implies <leaf> and/or/implies...
e.g.:
"community_data" flows to "community_delete_check"
and
"community_delete_check" has control flow influence on "db_write"
*/
fn chained_leaf_exprs<'a>(s: &'a str) -> Res<&str, ASTNode<'a>> {
    context(
        "parse chained leaf expressions",
        map(
            pair(leaf_expr, many0(tuple((and_or_implies, leaf_expr)))),
            |(first_expr, term_then_expr_vec)| {
                term_then_expr_vec
                    .into_iter()
                    .fold(first_expr, |acc, (term, next_expr)| {
                        let data : Box<TwoNodeObligation<'a>> = Box::new(
                            TwoNodeObligation {
                                src: acc,
                                dest: next_expr,
                            });
                        let term_type : TermLink = term.into();
                        match term_type {
                            TermLink::And => ASTNode::And(data),
                            TermLink::Or => ASTNode::Or(data),
                            TermLink::Implies => ASTNode::Implies(data),
                        }
                    })
            },
        ),
    )(s)
}

fn scope(s: &str) -> Res<&str, PolicyScope> {
    let mut combinator = context("scope", alt((always, sometimes)));
    let (remainder, res) = combinator(s)?;

    Ok((remainder, res.into()))
}

fn variable_binding<'a>(s: &'a str) -> Res<&str, VariableBinding<'a>> { 
    let mut combinator = context(
        "variable binding",
        delimited(
            multispace0,
            tuple((
                quantifier,
                variable,
                colon,
                multispace0,
                marker,
                multispace0,
                open_paren
            )),
            multispace0
        )
    );
    let (remainder, (quantifier, variable, _, _, marker, _, _)) = combinator(s)?;
    Ok((
        remainder,
        VariableBinding {
            quantifier,
            variable,
            marker
        }
    ))
}

// parses 
fn chained_bodies<'a>(s: &'a str) -> Res<&str, ASTNode<'a>> {
    let mut combinator = context(
        "chained bodies",
        terminated(
            pair(
                variable_binding,
                alt((
                    leaf_expr,
                    chained_leaf_exprs,
                    chained_bodies // uhh infinite recursion? think it's maybe ok bc we try the leaf cases first but I'm suspicious
                )),
            ),
            // opt bc a recursive call may get to it before we get out here
            opt(close_paren)
        )
    );
    let (remainder, (binding, body)) = combinator(s)?;
    Ok((
        remainder,
        ASTNode::VarIntroduction(
            Box::new(
                VariableClause {
                    binding,
                    body
                }
            )
        )
    ))
}

/*
format of body is now:
    - variable intro (quantifier + marker + open paren)
        - alt(
            - leaf (flows to / control flow / through)
            - chained leaf expressions
            - body contained within (recursive case)
          )
    - close paren
    - ^^ present >=1 time, joined by chained exprs/implies
*/

fn parse_body<'a>(s: &'a str) -> Res<&str, ASTNode<'a>> {
    context(
        "parse body",
        map(
            // TODO I don't think many0(and_or_implies is right here?)
            // todo this is identical to the chained_leaf_expressions except "chained_bodies" 
            // in the pair instead -- write a function to return this closure to reduce code reuse
            pair(chained_bodies, many0(tuple((and_or_implies, chained_bodies)))),
            |(first_expr, term_then_expr_vec)| {
                term_then_expr_vec
                    .into_iter()
                    .fold(first_expr, |acc, (term, next_expr)| {
                        let data : Box<TwoNodeObligation<'a>> = Box::new(
                            TwoNodeObligation {
                                src: acc,
                                dest: next_expr,
                            });
                        let term_type : TermLink = term.into();
                        match term_type {
                            TermLink::And => ASTNode::And(data),
                            TermLink::Or => ASTNode::Or(data),
                            TermLink::Implies => ASTNode::Implies(data),
                        }
                    })
            },
        ),
    )(s)
}

pub fn parse<'a>(s: &'a str) -> Res<&str, PolicyBody<'a>> {
    let mut combinator = context("parse policy", 
        // all_consuming(
            tuple((
                scope, chained_bodies
            ))
        // )
    );
    let (remainder, (scope, body)) = combinator(s)?;
    Ok((
        remainder,
        PolicyBody {
            scope,
            body
        }
    ))
}

/*
#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_is_nonalphabetic() {
        let spaces = "     ";
        let comma = ",";
        let period = ".";
        let newline = "\n";
        let punc = ",.\n";
        let err = "this is alphabetical";

        assert_eq!(multispace0(spaces), Ok(("", spaces)));
        assert_eq!(multispace0(comma), Ok(("", comma)));
        assert_eq!(multispace0(period), Ok(("", period)));
        assert_eq!(multispace0(newline), Ok(("", newline)));
        assert_eq!(multispace0(punc), Ok(("", punc)));
        assert!(multispace0(err).is_err());
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

        assert_eq!(leaf_expr(through), Ok(("", through_ans)));
        assert_eq!(leaf_expr(flows_to), Ok(("", flows_to_ans)));
        assert_eq!(leaf_expr(control_flow), Ok(("", control_flow_ans)));
        assert!(leaf_expr(err1).is_err());
        assert!(leaf_expr(err2).is_err());
        assert!(leaf_expr(err3).is_err());
    }

    #[test]
    fn test_chained_leaf_expressions() {
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

        assert_eq!(chained_leaf_exprs(policy1), Ok(("", policy1_ans)));
        assert_eq!(chained_leaf_exprs(policy2), Ok(("", policy2_ans)));
        assert_eq!(chained_leaf_exprs(policy3), Ok(("", policy3_ans)));
        assert!(chained_leaf_exprs(err1).is_err());
    }

    #[test]
    fn test_conditional() {
        let policy1 = "if a flows to b, then c flows to d";
        let policy1_ans = ASTNode::Implies(Box::new(TwoNodeObligation {
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
        let policy2_ans = ASTNode::Implies(Box::new(TwoNodeObligation {
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

        assert_eq!(implies(policy1), Ok(("", policy1_ans)));
        assert_eq!(implies(policy2), Ok(("", policy2_ans)));
        assert!(implies(err1).is_err());
        assert!(implies(err2).is_err());
        assert!(implies(err3).is_err());
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
            body: ASTNode::Implies(Box::new(TwoNodeObligation {
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
        let binding1_ans = VariableClause {
            variable: Variable { name: "a" },
            quantifier: Quantifier::Some,
            marker: "a",
        };
        let binding2 = "let sens = all \"sensitive\"";
        let binding2_ans = VariableClause {
            variable: Variable { name: "sens" },
            quantifier: Quantifier::All,
            marker: "sensitive",
        };
        let binding3 = "let delete_check = some \"community_delete_check\"\n        ";
        let binding3_ans = VariableClause {
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
        let single_ans = vec![VariableClause {
            variable: Variable { name: "sens" },
            quantifier: Quantifier::All,
            marker: "sensitive",
        }];
        let multi_newline = "let commit = some \"commit\"\nlet store = some \"sink\"\nlet auth_check = all \"check_rights\"\n";
        let multi_comma = "let commit = some \"commit\", let store = some \"sink\", let auth_check = all \"check_rights\"\n";
        let multi_ans = vec![
            VariableClause {
                variable: Variable { name: "commit" },
                quantifier: Quantifier::Some,
                marker: "commit",
            },
            VariableClause {
                variable: Variable { name: "store" },
                quantifier: Quantifier::Some,
                marker: "sink",
            },
            VariableClause {
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
            VariableClause {
                variable: Variable {
                    name: "community_struct",
                },
                quantifier: Quantifier::Some,
                marker: "community",
            },
            VariableClause {
                variable: Variable {
                    name: "delete_check",
                },
                quantifier: Quantifier::Some,
                marker: "community_delete_check",
            },
            VariableClause {
                variable: Variable { name: "ban_check" },
                quantifier: Quantifier::Some,
                marker: "community_ban_check",
            },
            VariableClause {
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
*/