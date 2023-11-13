use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while},
    character::{is_alphabetic, is_alphanumeric},
    sequence::tuple,
    IResult,
};

enum Quantifier {
    Some,
    All,
    No,
}

pub struct Variable<'a> {
    quantifier: Quantifier,
    marker: &'a str,
}

pub struct FlowsTo<'a> {
    left: Variable<'a>,
    right: Variable<'a>,
}

// todo: structs for AST

// todo: policy should use variables instead of quantifiers everywhere

pub fn some(s: &str) -> IResult<&str, &str> {
    tag("some")(s)
}

pub fn flows_to(s: &str) -> IResult<&str, &str> {
    tag("flowsto")(s)
}

pub fn marker(s: &str) -> IResult<&str, &str> {
    alt((take_until("flowsto"), take_while(char::is_alphabetic)))(s)
}

pub fn parse(s: &str) {
    let trimmed = s.replace(" ", "");
    let mut combinator = tuple((some, marker, flows_to, some, marker));
    let res = combinator(&trimmed);
    assert_eq!(res, Ok(("", ("some", "a", "flowsto", "some", "b"))));

    // construct AST
    // i think you change each parser to do the mapping into the data structure directly
    // so there's a variable parser and a flows_to one
    // and then a top-level one that calls tuple(variable, flows to, variable)
    // and maps the result into a FlowsTo struct
}
