use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{char, multispace0, multispace1},
    combinator::{all_consuming, not, opt, recognize},
    error::{context, VerboseError},
    multi::many1,
    sequence::{delimited, separated_pair, terminated, tuple},
    IResult,
};

use crate::{
    ASTNode, Marker, Operator, TwoNodeObligation, Policy, PolicyScope, Quantifier, ThreeVarObligation, TwoVarObligation, Variable, VariableBinding, VariableClause
};

pub type Res<T, U> = IResult<T, U, VerboseError<T>>;

static FLOWS_TO_TAG: &str = "flows to";
static CONTROL_FLOW_TAG: &str = "has control flow influence on";

fn colon(s: &str) -> Res<&str, &str> {
    context("colon", delimited(multispace0, tag(":"), multispace0))(s)
}

fn flows_to(s: &str) -> Res<&str, &str> {
    context("flows to", delimited(multispace1, tag(FLOWS_TO_TAG), multispace1))(s)
}

fn control_flow(s: &str) -> Res<&str, &str> {
    context(
        "control flow",
        delimited(multispace1, tag(CONTROL_FLOW_TAG), multispace1),
    )(s)
}

fn through(s: &str) -> Res<&str, &str> {
    context("through", delimited(multispace1, tag("through"), multispace1))(s)
}

fn always(s: &str) -> Res<&str, &str> {
    context(
        "always",
        delimited(multispace0, tag("always"), colon),
    )(s)
}

fn sometimes(s: &str) -> Res<&str, &str> {
    context(
        "sometimes",
        delimited(multispace0, tag("sometimes"), colon),
    )(s)
}

fn and(s: &str) -> Res<&str, &str> {
    context("and", delimited(multispace0, tag("and"), multispace1))(s)
}

fn or(s: &str) -> Res<&str, &str> {
    context("or", delimited(multispace0, tag("or"), multispace1))(s)
}

fn implies(s: &str) -> Res<&str, &str> {
    context("implies", delimited(multispace0, tag("implies"), multispace1))(s)
}

fn open_paren(s: &str) -> Res<&str, &str> {
    context("open paren", delimited(multispace0, tag("("), multispace0))(s)
}

fn close_paren(s: &str) -> Res<&str, &str> {
    context("close paren", delimited(multispace0, tag(")"), multispace0))(s)
}

fn some(s: &str) -> Res<&str, Quantifier> {
    let mut combinator = context("some", delimited(multispace0, tag("some"), multispace1));
    let (remainder, _) = combinator(s)?;

    Ok((remainder, Quantifier::Some))
}

fn all(s: &str) -> Res<&str, Quantifier> {
    let mut combinator = context("all", delimited(multispace0, tag("all"), multispace1));
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

fn marker<'a>(s: &'a str) -> Res<&str, Marker<'a>> {
    let (remainder, res) = context(
        "marker",
        delimited(tag("\""), alphabetic_w_underscores, tag("\""))
    )(s)?;
    Ok((remainder, res))
}

fn variable<'a>(s: &'a str) -> Res<&str, Variable<'a>> {
    let (remainder, res) = context(
        "variable",
        alphabetic_w_underscores,
    )(s)?;
    Ok((remainder, res))
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

// parse "and/or/implies <leaf expr>"
fn operator<'a>(s: &'a str) -> Res<&str, Operator> {
    let mut combinator = context("operator", alt((and, or, implies)));
    let (remainder, operator_str) = combinator(s)?;
    Ok((remainder, operator_str.into()))
}

fn scope(s: &str) -> Res<&str, PolicyScope> {
    let mut combinator = context("scope", alt((always, sometimes)));
    let (remainder, res) = combinator(s)?;

    Ok((remainder, res.into()))
}

fn joined_bodies<'a>(s: &'a str) -> Res<&str, ASTNode<'a>> {
    let mut combinator = context(
        "joined bodies",
        tuple((
            alt((flows_to_or_through_expr, control_flow_expr)), 
            operator, 
            body)),
    );
    let (remainder, (src, operator, dest)) = combinator(s)?;
    let body = Box::new(TwoNodeObligation {src, dest});

    let node = match operator {
        Operator::And => ASTNode::And(body),
        Operator::Or => ASTNode::Or(body),
        Operator::Implies => ASTNode::Implies(body),
    };

    Ok((remainder, node))
}

fn body<'a>(s: &'a str) -> Res<&str, ASTNode<'a>> {
    context(
        "body",
        alt((
            joined_bodies,
            flows_to_or_through_expr,
            control_flow_expr,
        ))
    )(s)
}

// parse joined expressions inside a variable clause
// needs to be called by variable_clause, i.e., this parses data *inside* a clause 
// so that bodies are allowed to be present alone
fn joined_clauses<'a>(s: &'a str) -> Res<&str, ASTNode<'a>> {
    let mut combinator = context(
        "joined clauses",
        tuple((
            alt((variable_clause, body)),
            operator, 
            alt((joined_clauses, variable_clause, body)),
        )));
    let (remainder, (src, operator, dest)) = combinator(s)?;
    let body = Box::new(TwoNodeObligation {src, dest});

    let node = match operator {
        Operator::And => ASTNode::And(body),
        Operator::Or => ASTNode::Or(body),
        Operator::Implies => ASTNode::Implies(body),
    };

    Ok((remainder, node))
}

fn variable_clause<'a>(s: &'a str) -> Res<&str, ASTNode<'a>> {
    let mut combinator = context(
        "variable clause",
        tuple((
            // first line; declare variable binding & open clause
            quantifier,
            terminated(variable, colon),
            terminated(marker, open_paren),
            // body of the clause & close clause
            terminated(
                    alt((joined_clauses, variable_clause, body)), 
                    terminated(close_paren, multispace0)
            ),
        ))
    );
    let (remainder, (quantifier, variable, marker, body)) = combinator(s)?;

    Ok((
        remainder,
        ASTNode::VarIntroduction(
            Box::new(VariableClause {
                binding : VariableBinding {
                    quantifier,
                    variable,
                    marker
                },
                body
            })
        )
    ))
}

// joined_clauses is capable of parsing everything that this does
// the difference is that joined_clauses lets *bodies* be joined together.
// That's fine as long as we're already inside a variable clause, which is always the case when we call that parser.
// But we don't want to allow bodies without variable bindings at the top level, hence this separate, more restrictive parser.
fn joined_variable_clauses<'a>(s: &'a str) -> Res<&str, ASTNode<'a>> {
    let mut combinator = context(
        "joined variable clauses",
        tuple((
            variable_clause, 
            operator, 
            exprs
        )),
    );
    let (remainder, (src, operator, dest)) = combinator(s)?;
    let body = Box::new(TwoNodeObligation {src, dest});

    let node = match operator {
        Operator::And => ASTNode::And(body),
        Operator::Or => ASTNode::Or(body),
        Operator::Implies => ASTNode::Implies(body),
    };

    Ok((remainder, node))
}

fn exprs<'a>(s: &'a str) -> Res<&str, ASTNode<'a>> {
    context(
        "exprs",
        alt((
            joined_variable_clauses,
            variable_clause,
        ))
    )(s)
}

pub fn parse<'a>(s: &'a str) -> Res<&str, Policy<'a>> {
    let mut combinator = context("parse policy", 
        all_consuming(
            tuple((
                scope, exprs,
            ))
        )
    );
    let (remainder, (scope, body)) = combinator(s)?;
    Ok((
        remainder,
        Policy {
            scope,
            body
        }
    ))
}


#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_scope() {
        let always = "always:";
        let sometimes = "sometimes:";
        let always_w_punc = "\nalways: \n";
        let sometimes_w_punc = "\nsometimes: \n";

        assert_eq!(scope(always), Ok(("", PolicyScope::Always)));
        assert_eq!(scope(always_w_punc), Ok(("", PolicyScope::Always)));
        assert_eq!(scope(sometimes), Ok(("", PolicyScope::Sometimes)));
        assert_eq!(scope(sometimes_w_punc), Ok(("", PolicyScope::Sometimes)));
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

        assert_eq!(variable(var1), Ok(("", "a")));
        assert_eq!(variable(var2), Ok(("", "sensitive")));
        assert_eq!(
            variable(partially_keyword),
            Ok((" flows to b", "a"))
        );
        assert!(variable(wrong).is_err());
    }

    #[test]
    fn test_flows_to_or_through_expr() {
        let policy1 = "a flows to b";
        let policy1_ans = ASTNode::FlowsTo(TwoVarObligation {src: "a", dest: "b"});
        let policy2 = "a flows to b through c";
        let policy2_ans = ASTNode::Through(ThreeVarObligation {src: "a", dest: "b", checkpoint: "c"});
        
        let err1 = "a has control flow influence on b";
        let err2 = "a flows to b through c through d";

        assert_eq!(flows_to_or_through_expr(policy1), Ok(("", policy1_ans)));
        assert_eq!(flows_to_or_through_expr(policy2), Ok(("", policy2_ans)));
        assert!(flows_to_or_through_expr(err1).is_err());
        assert_eq!(flows_to_or_through_expr(err2), Ok((" through d", ASTNode::Through(ThreeVarObligation { src: "a", dest: "b", checkpoint: "c" }))));
    }

    #[test]
    fn test_body() {
        let through = "a flows to b through c";
        let through_ans = ASTNode::Through(ThreeVarObligation {
            src: "a",
            dest: "b" ,
            checkpoint: "c" 
        });

        let flows_to = "a flows to b";
        let flows_to_ans = ASTNode::FlowsTo(TwoVarObligation {
            src: "a" ,
            dest: "b" 
        });
        let control_flow = "a has control flow influence on b";
        let control_flow_ans = ASTNode::ControlFlow(TwoVarObligation {
            src: "a",
            dest: "b" 
        });

        let joined1 = "a flows to b and a flows to b through c";
        let joined1_ans = ASTNode::And(
            Box::new(
                TwoNodeObligation {
                    src: ASTNode::FlowsTo(TwoVarObligation {
                        src: "a", 
                        dest: "b" 
                    }),
                    dest: ASTNode::Through(ThreeVarObligation {
                        src: "a", 
                        dest: "b",
                        checkpoint: "c" 
                    }),
                }
            )
        );

        let joined2 = "a flows to b and a flows to b through c or a has control flow influence on b";
        let joined2_ans = ASTNode::And(
            Box::new(
                TwoNodeObligation {
                    src: ASTNode::FlowsTo(TwoVarObligation {
                        src: "a", 
                        dest: "b" 
                    }),
                    dest: ASTNode::Or(
                        Box::new(
                            TwoNodeObligation {
                                src: ASTNode::Through(
                                    ThreeVarObligation {
                                        src: "a", 
                                        dest: "b",
                                        checkpoint: "c"
                                    }),
                                dest: ASTNode::ControlFlow(
                                    TwoVarObligation {
                                        src: "a", 
                                        dest: "b" 
                                    }),
                            }
                        )),
                }
            )
        );

        let joined3 = "a has control flow influence on b implies a flows to c and b flows to c";
        let joined3_ans = ASTNode::Implies(Box::new(TwoNodeObligation {
            src: ASTNode::ControlFlow(TwoVarObligation{
                src: "a",
                dest: "b",
            }),
            dest: ASTNode::And(
                Box::new(
                    TwoNodeObligation {
                        src: ASTNode::FlowsTo(TwoVarObligation {
                            src: "a", 
                            dest: "c" 
                        }),
                        dest: ASTNode::FlowsTo(TwoVarObligation {
                            src: "b", 
                            dest: "c" 
                        }),
                    }
                ))
        }));

        let err1 = "a flows to";
        let err2 = "a flows to b through";
        let err3 = "a has control flow influence on";

        assert_eq!(body(through), Ok(("", through_ans)));
        assert_eq!(body(flows_to), Ok(("", flows_to_ans)));
        assert_eq!(body(control_flow), Ok(("", control_flow_ans)));
        assert_eq!(body(joined1), Ok(("", joined1_ans)));
        assert_eq!(body(joined2), Ok(("", joined2_ans)));
        assert_eq!(body(joined3), Ok(("", joined3_ans)));
        assert!(body(err1).is_err());
        assert_eq!(body(err2), Ok((" through", ASTNode::FlowsTo(TwoVarObligation {src: "a", dest: "b"}))));
        assert!(body(err3).is_err());
    }


    #[test]
    fn test_variable_clause() {
        let simple_body = 
            "all dc : \"delete_check\" ( 
                dc flows to sink
            )";
        
        let simple_body_ans =
            ASTNode::VarIntroduction(Box::new(VariableClause {
                binding: VariableBinding {quantifier: Quantifier::All, variable: "dc", marker: "delete_check"},
                body: ASTNode::FlowsTo(TwoVarObligation{src: "dc", dest: "sink"})
            }));
        
        let joined_body =
            "all dc : \"delete_check\" ( 
                dc flows to sink or dc flows to encrypts and dc flows to source
            )";
        let joined_body_ans =
            ASTNode::VarIntroduction(Box::new (VariableClause {
                binding: VariableBinding {quantifier: Quantifier::All, variable: "dc", marker: "delete_check"},
                body: ASTNode::Or(
                    Box::new(TwoNodeObligation {
                        src: ASTNode::FlowsTo(TwoVarObligation{src: "dc", dest: "sink"}),
                        dest: ASTNode::And(Box::new(TwoNodeObligation {
                            src: ASTNode::FlowsTo(TwoVarObligation{src: "dc", dest: "encrypts"}),
                            dest: ASTNode::FlowsTo(TwoVarObligation{src: "dc", dest: "source"}),
                        }))
                    })
                ) 
            }));

        let triple_nested = 
            "some a : \"a\" (
                some b : \"b\" (
                    some c : \"c\" (
                        a flows to c
                    )
                )
            )";
        let triple_nested_ans =
        ASTNode::VarIntroduction(Box::new(VariableClause {
            binding: VariableBinding {quantifier: Quantifier::Some, variable: "a", marker: "a"},
            body: ASTNode::VarIntroduction(Box::new(VariableClause {
                binding: VariableBinding {quantifier: Quantifier::Some, variable: "b", marker: "b"},
                body: ASTNode::VarIntroduction(Box::new(VariableClause {
                    binding: VariableBinding {quantifier: Quantifier::Some, variable: "c", marker: "c"},
                    body: ASTNode::FlowsTo(TwoVarObligation{src: "a", dest: "c"})
                }))
            }))
        }));

        let lemmy_comm = "
        some comm_data : \"community_data\" (
            all write : \"db_write\" (
                comm_data flows to write
                implies
                some comm_dc : \"community_delete_check\" (
                    comm_data flows to comm_dc
                    and
                    comm_dc has control flow influence on write
                ) and
                some comm_bc : \"community_ban_check\" (
                    comm_data flows to comm_bc
                    and
                    comm_bc has control flow influence on write
                )
            )
        )";
        let lemmy_comm_ans = ASTNode::VarIntroduction(Box::new(VariableClause {
            binding: VariableBinding { quantifier: Quantifier::Some, variable: "comm_data", marker: "community_data" },
            body: ASTNode::VarIntroduction(Box::new(VariableClause { 
                binding: VariableBinding { quantifier: Quantifier::All, variable: "write", marker: "db_write" }, 
                body: ASTNode::Implies(Box::new(TwoNodeObligation { 
                    src: ASTNode::FlowsTo(TwoVarObligation { src: "comm_data", dest: "write" }), 
                    dest: ASTNode::And(Box::new(TwoNodeObligation{
                        src: ASTNode::VarIntroduction(Box::new(VariableClause { 
                            binding: VariableBinding { quantifier: Quantifier::Some, variable: "comm_dc", marker: "community_delete_check" }, 
                            body: ASTNode::And(Box::new(TwoNodeObligation { 
                                src: ASTNode::FlowsTo(TwoVarObligation { src: "comm_data", dest: "comm_dc" }), 
                                dest: ASTNode::ControlFlow(TwoVarObligation { src: "comm_dc", dest: "write" })
                            })) 
                        })),
                        dest: ASTNode::VarIntroduction(Box::new(VariableClause { 
                            binding: VariableBinding { quantifier: Quantifier::Some, variable: "comm_bc", marker: "community_ban_check" }, 
                            body: ASTNode::And(Box::new(TwoNodeObligation { 
                                src: ASTNode::FlowsTo(TwoVarObligation { src: "comm_data", dest: "comm_bc" }), 
                                dest: ASTNode::ControlFlow(TwoVarObligation { src: "comm_bc", dest: "write" })
                            })) 
                        })),
                    }))
                }))
            }))
        }));

        // should only parse the first *top level* variable clause
        let lemmy_inst = 
        "some dc: \"instance_delete_check\" (
            all write : \"db_write\" (
                dc has control flow influence on write
            )
            and
            all read: \"db_read\" (
                dc has control flow influence on read
            )
        ) and 
        some bc : \"instance_ban_check\" (
            all write : \"db_write\" (
                bc has control flow influence on write
            )
            and
            all read: \"db_read\" (
                bc has control flow influence on read
            )
        )";
        let lemmy_inst_partial = ASTNode::VarIntroduction(Box::new(VariableClause {
                binding: VariableBinding { quantifier: Quantifier::Some, variable: "dc", marker: "instance_delete_check" },
                body: ASTNode::And(Box::new(TwoNodeObligation {
                    src: ASTNode::VarIntroduction(Box::new(VariableClause {
                        binding: VariableBinding {quantifier: Quantifier::All, variable: "write", marker: "db_write"},
                        body: ASTNode::ControlFlow(TwoVarObligation { src: "dc", dest: "write"})
                        })),
                    dest: ASTNode::VarIntroduction(Box::new(VariableClause {
                        binding: VariableBinding {quantifier: Quantifier::All, variable: "read", marker: "db_read"},
                        body: ASTNode::ControlFlow(TwoVarObligation { src: "dc", dest: "read"})
                        })),
                    }))
            }));
        let lemmy_inst_leftover = "and 
        some bc : \"instance_ban_check\" (
            all write : \"db_write\" (
                bc has control flow influence on write
            )
            and
            all read: \"db_read\" (
                bc has control flow influence on read
            )
        )";

        // should be able to parse anything that joined_clauses can
        // as long as it's wrapped in a variable binding
        let wrapped =
            "some dc : \"delete_check\" (
                dc flows to sink or dc flows to encrypts through bc and dc has control flow influence on source
                implies
                all dc : \"delete_check\" ( 
                    dc flows to sink or dc flows to encrypts through bc and dc has control flow influence on source
                )
            )";
        
        let clause_with_joined_body_ans = 
            ASTNode::Implies(
                Box::new(TwoNodeObligation {
                    src: ASTNode::Or(Box::new(TwoNodeObligation {
                        src: ASTNode::FlowsTo(TwoVarObligation {src: "dc", dest: "sink"}),
                        dest: ASTNode::And(
                            Box::new(TwoNodeObligation {
                                src: ASTNode::Through(ThreeVarObligation {src: "dc", dest: "encrypts", checkpoint: "bc"}),
                                dest: ASTNode::ControlFlow(TwoVarObligation {src: "dc", dest: "source"})
                            }))
                        })),
                    dest: ASTNode::VarIntroduction(
                        Box::new(VariableClause {
                            binding : VariableBinding {quantifier: Quantifier::All, variable: "dc", marker: "delete_check"},
                            body: ASTNode::Or(Box::new(TwoNodeObligation {
                                src: ASTNode::FlowsTo(TwoVarObligation {src: "dc", dest: "sink"}),
                                dest: ASTNode::And(
                                    Box::new(TwoNodeObligation {
                                        src: ASTNode::Through(ThreeVarObligation {src: "dc", dest: "encrypts", checkpoint: "bc"}),
                                        dest: ASTNode::ControlFlow(TwoVarObligation {src: "dc", dest: "source"})
                                    }))
                            }))
                        }))
             }));

             let wrapped_ans = ASTNode::VarIntroduction(Box::new(VariableClause {
                binding: VariableBinding {quantifier: Quantifier::Some, variable: "dc", marker: "delete_check"},
                body: clause_with_joined_body_ans,
             }));

        assert_eq!(variable_clause(simple_body), Ok(("", simple_body_ans)));
        assert_eq!(variable_clause(joined_body), Ok(("", joined_body_ans)));
        assert_eq!(variable_clause(triple_nested), Ok(("", triple_nested_ans)));
        assert_eq!(variable_clause(lemmy_comm), Ok(("", lemmy_comm_ans)));
        assert_eq!(variable_clause(lemmy_inst), Ok((lemmy_inst_leftover, lemmy_inst_partial)));
        assert_eq!(variable_clause(wrapped), Ok(("", wrapped_ans)));
    }

    #[test]
    fn test_joined_clauses() {
        let two_bodies = "a flows to b and b flows to c";
        let three_bodies = "a flows to b and b flows to c and a flows to c";

        let clause_with_simple_body_w_joined_variable_clauses = 
            "all dc : \"delete_check\" ( 
                dc flows to sink or dc flows to encrypts through bc and dc has control flow influence on source
            ) or
            all dc : \"delete_check\" ( 
                dc flows to sink
            ) and
            all dc : \"delete_check\" ( 
                all dc : \"delete_check\" ( 
                    dc flows to sink
                )
            )";
        let clause_with_simple_body_w_joined_variable_clauses_ans = 
            ASTNode::Or(
                Box::new(TwoNodeObligation {
                    src: ASTNode::VarIntroduction(
                        Box::new(VariableClause {
                            binding : VariableBinding {quantifier: Quantifier::All, variable: "dc", marker: "delete_check"},
                            body: ASTNode::Or(Box::new(TwoNodeObligation {
                                src: ASTNode::FlowsTo(TwoVarObligation {src: "dc", dest: "sink"}),
                                dest: ASTNode::And(
                                    Box::new(TwoNodeObligation {
                                        src: ASTNode::Through(ThreeVarObligation {src: "dc", dest: "encrypts", checkpoint: "bc"}),
                                        dest: ASTNode::ControlFlow(TwoVarObligation {src: "dc", dest: "source"})
                                    }))
                            }))
                    })),
                    dest: ASTNode::And(Box::new(TwoNodeObligation {
                        src: ASTNode::VarIntroduction(
                            Box::new(VariableClause {
                                binding : VariableBinding {quantifier: Quantifier::All, variable: "dc", marker: "delete_check"},
                                body: ASTNode::FlowsTo(TwoVarObligation {src: "dc", dest: "sink"}),
                            })),
                        dest: ASTNode::VarIntroduction(
                            Box::new(VariableClause {
                                binding : VariableBinding {quantifier: Quantifier::All, variable: "dc", marker: "delete_check"},
                                body: ASTNode::VarIntroduction(
                                    Box::new(VariableClause {
                                        binding : VariableBinding {quantifier: Quantifier::All, variable: "dc", marker: "delete_check"},
                                        body: ASTNode::FlowsTo(TwoVarObligation {src: "dc", dest: "sink"}),
                                    }))
                            }))
                    })) 
            }));
        
        let clause_with_simple_body_w_variable_clause = 
            "all dc : \"delete_check\" ( 
                dc flows to sink
            ) or
            all bc : \"ban_check\" ( 
                bc flows to sink
            )";
        
        let clause_with_simple_body_w_variable_clause_ans = 
            ASTNode::Or(
                Box::new(TwoNodeObligation {
                    src: ASTNode::VarIntroduction(
                        Box::new(VariableClause {
                            binding : VariableBinding {quantifier: Quantifier::All, variable: "dc", marker: "delete_check"},
                            body: ASTNode::FlowsTo(TwoVarObligation {src: "dc", dest: "sink"}),
                        })),
                    dest: ASTNode::VarIntroduction(
                        Box::new(VariableClause {
                            binding : VariableBinding {quantifier: Quantifier::All, variable: "bc", marker: "ban_check"},
                            body: ASTNode::FlowsTo(TwoVarObligation {src: "bc", dest: "sink"}),
                        })),
             }));
        
        let clause_with_joined_body =
            "dc flows to sink or dc flows to encrypts through bc and dc has control flow influence on source
            implies
            all dc : \"delete_check\" ( 
                dc flows to sink or dc flows to encrypts through bc and dc has control flow influence on source
            )";
        
        let clause_with_joined_body_ans = 
            ASTNode::Implies(
                Box::new(TwoNodeObligation {
                    src: ASTNode::Or(Box::new(TwoNodeObligation {
                        src: ASTNode::FlowsTo(TwoVarObligation {src: "dc", dest: "sink"}),
                        dest: ASTNode::And(
                            Box::new(TwoNodeObligation {
                                src: ASTNode::Through(ThreeVarObligation {src: "dc", dest: "encrypts", checkpoint: "bc"}),
                                dest: ASTNode::ControlFlow(TwoVarObligation {src: "dc", dest: "source"})
                            }))
                        })),
                    dest: ASTNode::VarIntroduction(
                        Box::new(VariableClause {
                            binding : VariableBinding {quantifier: Quantifier::All, variable: "dc", marker: "delete_check"},
                            body: ASTNode::Or(Box::new(TwoNodeObligation {
                                src: ASTNode::FlowsTo(TwoVarObligation {src: "dc", dest: "sink"}),
                                dest: ASTNode::And(
                                    Box::new(TwoNodeObligation {
                                        src: ASTNode::Through(ThreeVarObligation {src: "dc", dest: "encrypts", checkpoint: "bc"}),
                                        dest: ASTNode::ControlFlow(TwoVarObligation {src: "dc", dest: "source"})
                                    }))
                            }))
                        }))
             }));
        
        let multiple_bodies = 
            "dc flows to sink or dc flows to encrypts through bc and dc has control flow influence on source
            and
            bc flows to encrypts
            implies
            all dc : \"delete_check\" ( 
                dc flows to sink or dc flows to encrypts through bc and dc has control flow influence on source
            ) or
            dc flows to encrypts";

        let multiple_bodies_ans = ASTNode::Implies(
            Box::new(TwoNodeObligation { 
            // the four statements in the body
            src: ASTNode::Or(Box::new(TwoNodeObligation { 
                src: ASTNode::FlowsTo(TwoVarObligation { src: "dc", dest: "sink" }), 
                dest: ASTNode::And(Box::new(TwoNodeObligation { 
                    src: ASTNode::Through(ThreeVarObligation { src: "dc", dest: "encrypts", checkpoint: "bc" }), 
                    dest: ASTNode::And(Box::new(TwoNodeObligation { 
                        src: ASTNode::ControlFlow(TwoVarObligation { src: "dc", dest: "source" }), 
                        dest: ASTNode::FlowsTo(TwoVarObligation { src: "bc", dest: "encrypts" })}))}))})), 
            // "implies" the rest
            dest: ASTNode::Or(Box::new(TwoNodeObligation { 
                src: ASTNode::VarIntroduction(Box::new(VariableClause { 
                    binding: VariableBinding { quantifier: Quantifier::All, variable: "dc", marker: "delete_check" }, 
                    body: ASTNode::Or(Box::new(TwoNodeObligation { 
                        src: ASTNode::FlowsTo(TwoVarObligation { src: "dc", dest: "sink" }), 
                        dest: ASTNode::And(Box::new(TwoNodeObligation { 
                            src: ASTNode::Through(ThreeVarObligation { src: "dc", dest: "encrypts", checkpoint: "bc" }), 
                            dest: ASTNode::ControlFlow(TwoVarObligation { src: "dc", dest: "source" }) }))}))})), 
                dest: ASTNode::FlowsTo(TwoVarObligation { src: "dc", dest: "encrypts" }) })) }));
        
        
        assert_eq!(joined_clauses(clause_with_simple_body_w_joined_variable_clauses), Ok(("", clause_with_simple_body_w_joined_variable_clauses_ans)));
        assert_eq!(joined_clauses(clause_with_simple_body_w_variable_clause), Ok(("", clause_with_simple_body_w_variable_clause_ans)));
        assert_eq!(joined_clauses(clause_with_joined_body), Ok(("", clause_with_joined_body_ans)));
        assert_eq!(joined_clauses(multiple_bodies), Ok(("", multiple_bodies_ans)));
        // errors b/c body already covers multiple conjoined bodies
        // this parser gets >1 body joined *with* variable clauses
        assert!(joined_clauses(two_bodies).is_err());
        assert!(joined_clauses(three_bodies).is_err());
    }

    #[test]
    fn test_joined_variable_clauses() {
        let lemmy_inst = 
        "some dc: \"instance_delete_check\" (
            all write : \"db_write\" (
                dc has control flow influence on write
            )
            and
            all read: \"db_read\" (
                dc has control flow influence on read
            )
        ) and 
        some bc : \"instance_ban_check\" (
            all write : \"db_write\" (
                bc has control flow influence on write
            )
            and
            all read: \"db_read\" (
                bc has control flow influence on read
            )
        )";
        let lemmy_inst_ans = ASTNode::And(Box::new(TwoNodeObligation {
            src: ASTNode::VarIntroduction(Box::new(VariableClause {
                binding: VariableBinding { quantifier: Quantifier::Some, variable: "dc", marker: "instance_delete_check" },
                body: ASTNode::And(Box::new(TwoNodeObligation {
                    src: ASTNode::VarIntroduction(Box::new(VariableClause {
                        binding: VariableBinding {quantifier: Quantifier::All, variable: "write", marker: "db_write"},
                        body: ASTNode::ControlFlow(TwoVarObligation { src: "dc", dest: "write"})
                        })),
                    dest: ASTNode::VarIntroduction(Box::new(VariableClause {
                        binding: VariableBinding {quantifier: Quantifier::All, variable: "read", marker: "db_read"},
                        body: ASTNode::ControlFlow(TwoVarObligation { src: "dc", dest: "read"})
                        })),
                    }))
                })),
            dest: ASTNode::VarIntroduction(Box::new(VariableClause {
                binding: VariableBinding { quantifier: Quantifier::Some, variable: "bc", marker: "instance_ban_check" },
                body: ASTNode::And(Box::new(TwoNodeObligation {
                    src: ASTNode::VarIntroduction(Box::new(VariableClause {
                        binding: VariableBinding {quantifier: Quantifier::All, variable: "write", marker: "db_write"},
                        body: ASTNode::ControlFlow(TwoVarObligation { src: "bc", dest: "write"})
                        })),
                    dest: ASTNode::VarIntroduction(Box::new(VariableClause {
                        binding: VariableBinding {quantifier: Quantifier::All, variable: "read", marker: "db_read"},
                        body: ASTNode::ControlFlow(TwoVarObligation { src: "bc", dest: "read"})
                        })),
                    }))
                })),
        }));

        let triple_clauses = 
        "all dc : \"delete_check\" ( 
            dc flows to sink
        ) or
        all bc : \"ban_check\" ( 
            bc flows to sink
        ) and 
        all ec : \"encrypts_check\" ( 
            ec flows to sink
        )";
        
        let triple_clauses_ans =
            ASTNode::Or(
                Box::new(TwoNodeObligation {
                    src: ASTNode::VarIntroduction(
                        Box::new(VariableClause {
                            binding : VariableBinding {quantifier: Quantifier::All, variable: "dc", marker: "delete_check"},
                            body: ASTNode::FlowsTo(TwoVarObligation {src: "dc", dest: "sink"}),
                        })),
                    dest: ASTNode::And(Box::new(TwoNodeObligation { 
                        src: ASTNode::VarIntroduction(
                            Box::new(VariableClause {
                                binding : VariableBinding {quantifier: Quantifier::All, variable: "bc", marker: "ban_check"},
                                body: ASTNode::FlowsTo(TwoVarObligation {src: "bc", dest: "sink"}),
                            })),
                        dest: ASTNode::VarIntroduction(
                            Box::new(VariableClause {
                                binding : VariableBinding {quantifier: Quantifier::All, variable: "ec", marker: "encrypts_check"},
                                body: ASTNode::FlowsTo(TwoVarObligation {src: "ec", dest: "sink"}),
                            })),
                    }))
             }));
        
        // can't have bodies w/o bindings
        let multiple_bodies = 
            "dc flows to sink or dc flows to encrypts through bc and dc has control flow influence on source
            and
            bc flows to encrypts
            implies
            all dc : \"delete_check\" ( 
                dc flows to sink or dc flows to encrypts through bc and dc has control flow influence on source
            ) or
            dc flows to encrypts";
        
        let clause_with_joined_body =
            "dc flows to sink or dc flows to encrypts through bc and dc has control flow influence on source
            implies
            all dc : \"delete_check\" ( 
                dc flows to sink or dc flows to encrypts through bc and dc has control flow influence on source
            )";

        assert_eq!(joined_variable_clauses(lemmy_inst), Ok(("", lemmy_inst_ans)));
        assert_eq!(joined_variable_clauses(triple_clauses), Ok(("", triple_clauses_ans)));
        assert!(joined_variable_clauses(multiple_bodies).is_err());
        assert!(joined_variable_clauses(clause_with_joined_body).is_err());

    }

    #[test]
    fn test_parse() {
        let lemmy_inst = 
        "always:
        some dc: \"instance_delete_check\" (
            all write : \"db_write\" (
                dc has control flow influence on write
            )
            and
            all read: \"db_read\" (
                dc has control flow influence on read
            )
        ) and 
        some bc : \"instance_ban_check\" (
            all write : \"db_write\" (
                bc has control flow influence on write
            )
            and
            all read: \"db_read\" (
                bc has control flow influence on read
            )
        )";
        let lemmy_inst_ans = Policy {
            scope : PolicyScope::Always, 
            body: ASTNode::And(Box::new(TwoNodeObligation {
                src: ASTNode::VarIntroduction(Box::new(VariableClause {
                    binding: VariableBinding { quantifier: Quantifier::Some, variable: "dc", marker: "instance_delete_check" },
                    body: ASTNode::And(Box::new(TwoNodeObligation {
                        src: ASTNode::VarIntroduction(Box::new(VariableClause {
                            binding: VariableBinding {quantifier: Quantifier::All, variable: "write", marker: "db_write"},
                            body: ASTNode::ControlFlow(TwoVarObligation { src: "dc", dest: "write"})
                            })),
                        dest: ASTNode::VarIntroduction(Box::new(VariableClause {
                            binding: VariableBinding {quantifier: Quantifier::All, variable: "read", marker: "db_read"},
                            body: ASTNode::ControlFlow(TwoVarObligation { src: "dc", dest: "read"})
                            })),
                        }))
                    })),
                dest: ASTNode::VarIntroduction(Box::new(VariableClause {
                    binding: VariableBinding { quantifier: Quantifier::Some, variable: "bc", marker: "instance_ban_check" },
                    body: ASTNode::And(Box::new(TwoNodeObligation {
                        src: ASTNode::VarIntroduction(Box::new(VariableClause {
                            binding: VariableBinding {quantifier: Quantifier::All, variable: "write", marker: "db_write"},
                            body: ASTNode::ControlFlow(TwoVarObligation { src: "bc", dest: "write"})
                            })),
                        dest: ASTNode::VarIntroduction(Box::new(VariableClause {
                            binding: VariableBinding {quantifier: Quantifier::All, variable: "read", marker: "db_read"},
                            body: ASTNode::ControlFlow(TwoVarObligation { src: "bc", dest: "read"})
                            })),
                        }))
                    })),
                }))
            };

        let lemmy_comm = "
            always:
            some comm_data : \"community_data\" (
            all write : \"db_write\" (
                comm_data flows to write
                implies
                some comm_dc : \"community_delete_check\" (
                    comm_data flows to comm_dc
                    and
                    comm_dc has control flow influence on write
                ) and
                some comm_bc : \"community_ban_check\" (
                    comm_data flows to comm_bc
                    and
                    comm_bc has control flow influence on write
                )
            )
        )";
        let lemmy_comm_ans = Policy {
            scope: PolicyScope::Always,
            body: ASTNode::VarIntroduction(Box::new(VariableClause {
                binding: VariableBinding { quantifier: Quantifier::Some, variable: "comm_data", marker: "community_data" },
                body: ASTNode::VarIntroduction(Box::new(VariableClause { 
                    binding: VariableBinding { quantifier: Quantifier::All, variable: "write", marker: "db_write" }, 
                    body: ASTNode::Implies(Box::new(TwoNodeObligation { 
                        src: ASTNode::FlowsTo(TwoVarObligation { src: "comm_data", dest: "write" }), 
                        dest: ASTNode::And(Box::new(TwoNodeObligation{
                            src: ASTNode::VarIntroduction(Box::new(VariableClause { 
                                binding: VariableBinding { quantifier: Quantifier::Some, variable: "comm_dc", marker: "community_delete_check" }, 
                                body: ASTNode::And(Box::new(TwoNodeObligation { 
                                    src: ASTNode::FlowsTo(TwoVarObligation { src: "comm_data", dest: "comm_dc" }), 
                                    dest: ASTNode::ControlFlow(TwoVarObligation { src: "comm_dc", dest: "write" })
                                })) 
                            })),
                            dest: ASTNode::VarIntroduction(Box::new(VariableClause { 
                                binding: VariableBinding { quantifier: Quantifier::Some, variable: "comm_bc", marker: "community_ban_check" }, 
                                body: ASTNode::And(Box::new(TwoNodeObligation { 
                                    src: ASTNode::FlowsTo(TwoVarObligation { src: "comm_data", dest: "comm_bc" }), 
                                    dest: ASTNode::ControlFlow(TwoVarObligation { src: "comm_bc", dest: "write" })
                                })) 
                            })),
                        }))
                    }))
                }))
            }))
        };
        assert_eq!(parse(lemmy_comm), Ok(("", lemmy_comm_ans)));
        assert_eq!(parse(lemmy_inst), Ok(("", lemmy_inst_ans)));
    }
}
