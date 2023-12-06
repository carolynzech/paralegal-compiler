use handlebars::{no_escape, Handlebars};
use lazy_static::lazy_static;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::io::Result;

pub mod parsers;

const BASE_TEMPLATE: &str = "base";
const ALWAYS_TEMPLATE: &str = "always";
const INTRODUCE_VAR: &str = "first-var-reference";
const FLOWS_TO_TEMPLATE: &str = "flows-to";
const CONTROL_FLOW_TEMPLATE: &str = "control-flow";
const THROUGH_TEMPLATE: &str = "through";
const IF_FLOWS_TO_SOME_SOME: &str = "if-flows-to-some-some";

lazy_static! {
    static ref TEMPLATES: HashMap<&'static str, &'static str> = {
        let m = HashMap::from([
            (BASE_TEMPLATE, "templates/base.txt"),
            (INTRODUCE_VAR, "templates/first-var-reference.txt"),
            (FLOWS_TO_TEMPLATE, "templates/flows-to.txt"),
            (CONTROL_FLOW_TEMPLATE, "templates/control-flow.txt"),
            (THROUGH_TEMPLATE, "templates/through.txt"),
            (ALWAYS_TEMPLATE, "templates/scope/always.txt"),
            (IF_FLOWS_TO_SOME_SOME, "templates/if-flows-to/some-some.txt"),
        ]);
        m
    };
}

/* TODOs
    (Functionality)
    - For "a flows to b", instead of getting every node marked b, then filtering
      for the ones that a flows to, call influencees to start from what a flows to
      and filter to the ones marked b.
    - conditionals: have multiples? perhaps only allowed after periods.
    - parentheses to change order that obligations are enforced (e.g., A and (B or C)))
    - add "In <controller name>" in addition to Always/Sometimes, meaning Paralegal should apply
      the policy to the controller with that name
    - “is authorized by” primitive as syntactic sugar
    - possible syntactic sugar for flows to / control flow influence
    - negation : "no quantifier" / "does not flow to"
    - "one" quantifier

    (Good Practice / User Experience / Nits)
    - better error handling
    - pass template file paths as arguments instead of string literals
    - escaping {{}} in Rust code w/o overwriting no-escape for HTML characters
    - better leveraging of handlebars functionality (partials)
    - cargo new for the policy and write a template a Cargo.toml for it as well
    - better separate concerns in this repository (break up parsers into multiple files, etc.)
*/

#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
pub enum Quantifier {
    Some,
    All,
    // No,
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

#[derive(Debug, PartialEq, Eq)]
pub enum PolicyScope {
    Always,
    Sometimes,
    // AnalysisPoint(&'a str),
}

impl From<&str> for PolicyScope {
    fn from(s: &str) -> Self {
        match s {
            "always" => PolicyScope::Always,
            "sometimes" => PolicyScope::Sometimes,
            &_ => unimplemented!("no other quantifiers supported"),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct PolicyBody<'a> {
    scope: PolicyScope,
    body: ASTNode<'a>,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize)]
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
    checkpoint: Variable<'a>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ASTNode<'a> {
    FlowsTo(TwoVarObligation<'a>),
    ControlFlow(TwoVarObligation<'a>),
    Through(Box<ThroughData<'a>>),
    Conjunction(Box<ConjunctionData<'a>>),
    Conditional(Box<ConditionalData<'a>>),
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
pub struct VariableBinding<'a> {
    variable: Variable<'a>,
    quantifier: Quantifier,
    marker: &'a str,
}

fn func_call(q: &Quantifier) -> &str {
    match q {
        Quantifier::Some => "any",
        Quantifier::All => "all",
        // Quantifier::No => todo!(),
    }
}

fn register_and_render_template<'a, T: serde::Serialize, U: serde::Serialize>(
    handlebars: &mut Handlebars,
    map: &mut HashMap<T, U>,
    registered_templates: &mut HashSet<&'a str>,
    name: &'a str,
) -> String {
    if !registered_templates.contains(&name) {
        handlebars
            .register_template_file(name, TEMPLATES[name])
            .expect(&format!(
                "Could not register {name} template with handlebars"
            ));
        registered_templates.insert(name);
    }
    handlebars
        .render(name, &map)
        .expect(&format!("Could not render {name} handlebars template"))
}

fn compile_scope<'a>(
    handlebars: &mut Handlebars,
    scope: PolicyScope,
    bindings: &Vec<VariableBinding>,
    mut registered_templates: &mut HashSet<&'a str>,
) -> String {
    match scope {
        PolicyScope::Always => {
            let mut map: HashMap<&str, Vec<VariableBinding>> = HashMap::new();
            map.insert("bindings", bindings.to_vec());

            register_and_render_template(
                handlebars,
                &mut map,
                &mut registered_templates,
                ALWAYS_TEMPLATE,
            )
        }
        PolicyScope::Sometimes => {
            todo!()
        }
    }
}

fn find_variable<'a>(
    bindings: &Vec<VariableBinding<'a>>,
    variable: &Variable<'a>,
) -> VariableBinding<'a> {
    bindings
        .iter()
        .find(|b| b.variable == *variable)
        .unwrap()
        .to_owned()
}

// Called when a variable is referenced for the first time in the policy body
fn introduce_variable<'a>(
    handlebars: &mut Handlebars,
    variable: &Variable<'a>,
    bindings: &Vec<VariableBinding>,
    visited: &mut HashSet<Variable<'a>>,
    registered_templates: &mut HashSet<&'a str>,
) -> String {
    let mut map: HashMap<&str, &str> = HashMap::new();
    if visited.contains(variable) {
        return String::new();
    }
    visited.insert(variable.clone());

    let binding = find_variable(bindings, &variable);

    map.insert("src_var", variable.name);
    map.insert("src_func_call", func_call(&binding.quantifier));
    // TODO what goes for body?

    register_and_render_template(handlebars, &mut map, registered_templates, INTRODUCE_VAR)
}

// TODO need to add logic to introduce the variable in the policy if this is the first time it's been referenced
fn traverse_ast<'a>(
    handlebars: &mut Handlebars,
    node: ASTNode<'a>,
    bindings: &Vec<VariableBinding>,
    visited: &mut HashSet<Variable<'a>>,
    registered_templates: &mut HashSet<&'a str>,
) -> String {
    let mut map: HashMap<&str, &str> = HashMap::new();
    match node {
        ASTNode::FlowsTo(obligation) => {
            map.insert("src_var", obligation.src.name);
            map.insert("dest_var", obligation.dest.name);
            let src_intro = introduce_variable(
                handlebars,
                &obligation.src,
                bindings,
                visited,
                registered_templates,
            );
            let dest_intro = introduce_variable(
                handlebars,
                &obligation.dest,
                bindings,
                visited,
                registered_templates,
            );
            let flows_to_clause = register_and_render_template(
                handlebars,
                &mut map,
                registered_templates,
                FLOWS_TO_TEMPLATE,
            );
            format!("{src_intro}{dest_intro}{flows_to_clause}")
        }
        ASTNode::ControlFlow(obligation) => {
            map.insert("src_var", obligation.src.name);
            map.insert("dest_var", obligation.dest.name);
            let src_intro = introduce_variable(
                handlebars,
                &obligation.src,
                bindings,
                visited,
                registered_templates,
            );
            let dest_intro = introduce_variable(
                handlebars,
                &obligation.dest,
                bindings,
                visited,
                registered_templates,
            );
            let control_flow_clause = register_and_render_template(
                handlebars,
                &mut map,
                registered_templates,
                CONTROL_FLOW_TEMPLATE,
            );
            format!("{src_intro}{dest_intro}{control_flow_clause}")
        }
        ASTNode::Through(through_data) => {
            match through_data.flows_to {
                ASTNode::FlowsTo(obligation) => {
                    map.insert("src_var", obligation.src.name);
                    map.insert("dest_var", obligation.dest.name);
                }
                _ => panic!("should not have anything other than FlowsTo as src of Through node"),
            };
            map.insert("checkpoint", through_data.checkpoint.name);
            register_and_render_template(
                handlebars,
                &mut map,
                registered_templates,
                THROUGH_TEMPLATE,
            )
        }
        ASTNode::Conjunction(conjunction_data) => match conjunction_data.typ {
            Conjunction::And => {
                let left_res = traverse_ast(
                    handlebars,
                    conjunction_data.src,
                    bindings,
                    visited,
                    registered_templates,
                );
                let right_res = traverse_ast(
                    handlebars,
                    conjunction_data.dest,
                    bindings,
                    visited,
                    registered_templates,
                );
                format!("{left_res} && {right_res}")
            }
            Conjunction::Or => {
                let left_res = traverse_ast(
                    handlebars,
                    conjunction_data.src,
                    bindings,
                    visited,
                    registered_templates,
                );
                let right_res = traverse_ast(
                    handlebars,
                    conjunction_data.dest,
                    bindings,
                    visited,
                    registered_templates,
                );
                format!("{left_res} || {right_res}")
            }
        },
        ASTNode::Conditional(conditional_data) => {
            match conditional_data.premise {
                ASTNode::FlowsTo(premise_ob) => {
                    map.insert("src_var", premise_ob.src.name);
                    map.insert("dest_var", premise_ob.dest.name);

                    // we'll introduce these variables in the if-specific templates,
                    // so no need for an introduce_variable call
                    // mark them as visited to avoid redundant introductions
                    visited.insert(premise_ob.src.clone());
                    visited.insert(premise_ob.dest.clone());

                    let obligation_body = traverse_ast(
                        handlebars,
                        conditional_data.obligation,
                        bindings,
                        visited,
                        registered_templates,
                    );
                    map.insert("obligation", &obligation_body);

                    let src_binding = find_variable(bindings, &premise_ob.src);
                    let dest_binding = find_variable(bindings, &premise_ob.dest);
                    match (src_binding.quantifier, dest_binding.quantifier) {
                        (Quantifier::Some, Quantifier::Some) => register_and_render_template(
                            handlebars,
                            &mut map,
                            registered_templates,
                            IF_FLOWS_TO_SOME_SOME,
                        ),
                        _ => todo!(),
                    }
                }
                _ => todo!(),
            }
        }
    }
}

fn compile_ast<'a>(
    handlebars: &mut Handlebars,
    node: ASTNode<'a>,
    bindings: &Vec<VariableBinding>,
    registered_templates: &mut HashSet<&'a str>,
) -> String {
    let mut visited: HashSet<Variable<'a>> = HashSet::new();
    traverse_ast(
        handlebars,
        node,
        bindings,
        &mut visited,
        registered_templates,
    )
}

fn compile_policy<'a>(
    handlebars: &mut Handlebars,
    policy_body: PolicyBody<'a>,
    bindings: Vec<VariableBinding>,
) -> Result<()> {
    let mut map: HashMap<&str, &str> = HashMap::new();
    // TODO: it may be easier to understand this codebase if you just
    // register all the templates up front, regardless of whether you use them
    let mut registered_templates: HashSet<&str> = HashSet::new();

    let scope_res = compile_scope(
        handlebars,
        policy_body.scope,
        &bindings,
        &mut registered_templates,
    );
    // dbg!(&scope_res);
    map.insert("scope", &scope_res);

    let ast_res = compile_ast(
        handlebars,
        policy_body.body,
        &bindings,
        &mut registered_templates,
    );
    map.insert("obligation", &ast_res);

    let res = register_and_render_template(
        handlebars,
        &mut map,
        &mut registered_templates,
        BASE_TEMPLATE,
    );

    fs::write("test.rs", &res)?;
    Ok(())
}

pub fn compile<'a>(policy_body: PolicyBody<'a>, env: Vec<VariableBinding>) -> Result<()> {
    let mut handlebars = Handlebars::new();
    handlebars.register_escape_fn(no_escape);
    compile_policy(&mut handlebars, policy_body, env)
}
