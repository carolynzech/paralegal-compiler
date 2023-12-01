use handlebars::{no_escape, Handlebars};
use std::collections::HashMap;
use std::fs;
use std::io::Result;

pub mod parsers;

// template names
const BASE_TEMPLATE: &str = "base";
const FLOWS_TO_TEMPLATE: &str = "flows-to";
const CONTROL_FLOW_TEMPLATE: &str = "control-flow";

/* TODOs
    (Functionality)
    - For "a flows to b", instead of getting every node marked b, then filtering
      for the ones that a flows to, call influencees to start from what a flows to
      and filter to the ones marked b.
    - conditionals: have multiples? perhaps only allowed after periods.
    - parentheses to change order that obligations are enforced (e.g., A and (B or C)))

    (Good Practice / User Experience / Nits)
    - better error handling
    - pass template file paths as arguments instead of string literals
    - escaping {{}} in Rust code w/o overwriting no-escape for HTML characters
    - cargo new for the policy and write a template a Cargo.toml for it as well
    - better separate concerns in this repository (break up parsers into multiple files, etc.)
*/

#[derive(Debug, PartialEq, Eq, Clone)]
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
            // "no" => Quantifier::No,
            &_ => unimplemented!("no other quantifiers supported"),
        }
    }
}
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
struct Variable<'a> {
    name: &'a str,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TwoVarObligation<'a> {
    src: Variable<'a>,
    dest: Variable<'a>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Conjunction {
    And,
    Or,
}

impl From<&str> for Conjunction {
    fn from(s: &str) -> Self {
        match s {
            "and" => Conjunction::And,
            "or" => Conjunction::Or,
            &_ => unimplemented!("no other conjunctions supported"),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ConjunctionData<'a> {
    typ: Conjunction,
    src: ASTNode<'a>,
    dest: ASTNode<'a>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ConditionalData<'a> {
    premise: ASTNode<'a>,
    obligation: ASTNode<'a>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ThroughData<'a> {
    flows_to: ASTNode<'a>,
    var: Variable<'a>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ASTNode<'a> {
    FlowsTo(TwoVarObligation<'a>),
    ControlFlow(TwoVarObligation<'a>),
    Through(Box<ThroughData<'a>>),
    Conjunction(Box<ConjunctionData<'a>>),
    Conditional(Box<ConditionalData<'a>>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct VariableBinding<'a> {
    variable: Variable<'a>,
    quantifier: Quantifier,
    marker: &'a str,
}

fn func_call(q: &Quantifier) -> &str {
    match q {
        Quantifier::Some => "any",
        Quantifier::All => "all",
        Quantifier::No => todo!(),
    }
}

// TODO: wonder if there's a better way to fix the Variable/VariableBinding structs lifetime issue than just making everything Strings
// it'd be nice to let the Variable itself be the key, but it doesn't really matter
pub fn construct_env(
    bindings: Vec<VariableBinding>,
    env: &mut HashMap<String, (Quantifier, String)>,
) {
    for binding in bindings {
        let key = String::from(binding.variable.name);
        let val = (binding.quantifier, String::from(binding.marker));
        if env.contains_key(&key) {
            panic!("Policy contains duplicate variable binding {}", key);
        }
        env.insert(key, val);
    }
}

/*
Traverse the tree in-order
When node is a FlowsTo or ControlFlow, base case: find template, fill it in
Otherwise,

*/

fn compile_policy<'a>(
    handlebars: &mut Handlebars,
    policy_body: ASTNode<'a>,
    env: &HashMap<String, (Quantifier, String)>,
) -> Result<()> {
    let (obligation, template) = match policy_body {
        ASTNode::FlowsTo(o) => (o, FLOWS_TO_TEMPLATE),
        ASTNode::ControlFlow(o) => (o, CONTROL_FLOW_TEMPLATE),
        _ => todo!(),
    };

    let template_path = match template {
        FLOWS_TO_TEMPLATE => "templates/flows-to.txt",
        CONTROL_FLOW_TEMPLATE => "templates/control-flow.txt",
        &_ => panic!("should not reach this case"),
    };

    handlebars
        .register_template_file(template, template_path)
        .expect("Could not register flows to template with handlebars");

    let src_var = obligation.src.name;
    let dest_var = obligation.dest.name;
    let (src_quantifier, src_marker) = env.get(src_var).unwrap();
    let (dest_quantifier, dest_marker) = env.get(dest_var).unwrap();

    /*
    TODO
    templates should be broken up: there should be flows-to logic that just has the flows-to call,
    then some kind of flows_to prefix that inserts var_nodes.func_call(|var|) before the logic for src and dest
    insert the prefix for src and dest iff it's the first reference to them in the policy body
    */

    let mut template_map: HashMap<&str, &str> = HashMap::new();
    template_map.insert("src_var", src_var);
    template_map.insert("dest_var", dest_var);
    template_map.insert("src_marker", src_marker);
    template_map.insert("dest_marker", dest_marker);
    template_map.insert("src_func_call", func_call(&src_quantifier));
    template_map.insert("dest_func_call", func_call(&dest_quantifier));

    let policy = handlebars
        .render(template, &template_map)
        .expect("Could not render flows to handlebars template");

    template_map.insert("policy", &policy);

    let res = handlebars
        .render(BASE_TEMPLATE, &template_map)
        .expect("Could not render flows to handlebars template");

    fs::write("compiled-policy.rs", &res)?;
    Ok(())
}

pub fn compile<'a>(
    policy_body: ASTNode<'a>,
    env: &HashMap<String, (Quantifier, String)>,
) -> Result<()> {
    let mut handlebars = Handlebars::new();

    handlebars.register_escape_fn(no_escape);

    handlebars
        .register_template_file(BASE_TEMPLATE, "templates/base.txt")
        .expect("Could not register base template with handlebars");

    compile_policy(&mut handlebars, policy_body, env)
}
