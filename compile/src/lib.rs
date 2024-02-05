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

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
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

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct TwoNodeObligation<'a> {
    src: ASTNode<'a>,
    dest: ASTNode<'a>
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ThreeVarObligation<'a> {
    src: Variable<'a>,
    dest: Variable<'a>,
    checkpoint: Variable<'a>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum ASTNode<'a> {
    FlowsTo(TwoVarObligation<'a>),
    ControlFlow(TwoVarObligation<'a>),
    Through(ThreeVarObligation<'a>),
    And(Box<TwoNodeObligation<'a>>),
    Or(Box<TwoNodeObligation<'a>>),
    Conditional(Box<TwoNodeObligation<'a>>),
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

fn compile_policy_scope<'a>(
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

/*
wait... but if the same node is opening & closing two variables, 
how do you know which should be nested inside the other
 -- think b/c of this "same object" rule we've established, the *object*
 always goes first, e.g.
    A flows to B
    A has control flow influence on B
introduce B first, then A, because we need to establish B as *an* object
before we talk about A


minimum example:
all passwords flow to some encrypts and
all passwords flow to some encrypts

let [passwords, encrypts] (
    FlowsTo(passwords, encrypts) &&
    FlowsTo(passwords, encrypts)
)

if we compile to:
all passwords | some encrypts | flows_to(passwords, encrypts)
that's wrong (needs to be the same encrypts object), but
some encrypts | all passwords | flows_to(passwords, encrypts)
is correct

(this works)
all passwords flow to some encrypts1 and
all private_keys flow to some encrypts2

some encrypts1 | all passwords | flows_to(passwords, encrypts1) &&
some encrypts2 | all private_keys | flows_to(passwords, encrypts2)
*/

/*
STEP 1:
create a hashmap of ASTNode --> set of Variables they reference
traverse tree until you reach a leaf (flows to / control flow)
at that point, make first entry in hashmap
in non-leaf nodes, their entries should be the unique set of their children's results
*/

fn unionize_var_sets<'a>(left_set : &HashSet<Variable<'a>>, right_set: &HashSet<Variable<'a>>, union: &mut HashSet<Variable<'a>>) {
    let ref_union: HashSet<&Variable<'a>> = left_set.union(&right_set).collect();
    // TODO there must be a more idiomatic way of doing this
    for var_ref in ref_union {
        let var = var_ref.clone().to_owned();
        union.insert(var);
    }
}

// bottom-up tree traversal to determine the set of all variables that a node & its children reference
fn determine_var_scope<'a>(
    node: &ASTNode<'a>,
    references: &mut HashMap<ASTNode<'a>, HashSet<Variable<'a>>>,
) {
    let mut map: HashMap<&str, &str> = HashMap::new();
    match node {
        ASTNode::FlowsTo(obligation) | ASTNode::ControlFlow(obligation) => {
            references[node] = HashSet::from([obligation.src, obligation.dest]);
        },
        ASTNode::Through(obligation) => {
            references[node] = HashSet::from([obligation.src, obligation.dest, obligation.checkpoint]);
        },
        ASTNode::And(obligation) | ASTNode::Or(obligation) | ASTNode::Conditional(obligation) => {
            determine_var_scope(&obligation.src, references);
            determine_var_scope(&obligation.dest, references);
            
            // this node's var scope is the set of its children's
            let left_set: HashSet<Variable<'a>> = references[&obligation.src];
            let right_set: HashSet<Variable<'a>> = references[&obligation.dest];
            
            let mut union : HashSet<Variable<'a>> = HashSet::new();
            unionize_var_sets(&left_set, &right_set, &mut union);
            references[node] = union;

        },
    }
}

/*
STEP 2:
**Pretty Printing**

if you're opening and closing, output this:
let [var_name] (
    {rest}
)

at a non-leaf, 
rest = 
    {conjunction type}
    {recursive result}

at a leaf, rest = {leaf node}

let [encrypts] (
    And (
        let [passwords] (
            FlowsTo(passwords, encrypts)
        )
        let [private_keys] (
            FlowsTo(private_keys, encrypts)
        )
    )
)
*/

enum IntermediateNode<'a> {
    Binding(Box<BindingBody<'a>>),
    Conditional(Box<NonLeafNodeBody<'a>>),
    And(Box<NonLeafNodeBody<'a>>),
    Or(Box<NonLeafNodeBody<'a>>),
    FlowsTo(TwoVarObligation<'a>),
    ControlFlow(TwoVarObligation<'a>),
    Through(ThreeVarObligation<'a>),
}

struct BindingBody<'a> {
    variable: Variable<'a>,
    body: IntermediateNode<'a>
}

struct NonLeafNodeBody<'a> {
    src: IntermediateNode<'a>,
    dest: IntermediateNode<'a>
}

/*
STEP 2:
(recursive algorithm)
From the top:
For each var in that node's set:
    - if the node is a nonleaf:
        - if the var in the left child's set and the right child's set, this node introduces mark as visited
        - otherwise, do nothing in this node
    - if the node is a leaf, introduce any vars not in the visited set
*/
fn construct_intermediate_rep<'a>(
    node: &ASTNode<'a>,
    references: &mut HashMap<ASTNode<'a>, HashSet<Variable<'a>>>,
    visited: &mut HashSet<Variable<'a>>,
) -> IntermediateNode<'a> {
    match node {
        ASTNode::FlowsTo(obligation) | ASTNode::ControlFlow(obligation) => {
            // if src & dest both in visited, return LeafNode
            // if one of them is, return binding of that node with LeafNode as body
            // if neither of them are, return binding of dest, then src, then LeafNode as body
            let body = IntermediateNode::FlowsTo(obligation.clone());

            // TODO:
            // one, you need to fix the body declaration since it could also be control flow
            // two, I wonder if there's a more recursive way of doing this -- perhaps a recursive helper?
            // going through all of the permutations is going to get ugly (9 possibilities!) for through
            // the issue with recursion may be that you have to be careful about the order of introduction
            // (e.g., how dest comes before src).
            // but wait, for through this may not even matter because of how we call always_happens_before...
            // (on all the nodes marked a thing)
            if visited.contains(&obligation.src) && visited.contains(&obligation.dest) {
                body
            } else if visited.contains(&obligation.src) {
                IntermediateNode::Binding(Box::new(
                    BindingBody {
                        variable: obligation.src,
                        body
                    }))
            } else if visited.contains(&obligation.dest) {
                IntermediateNode::Binding(Box::new(
                    BindingBody {
                        variable: obligation.dest,
                        body
                    }))
            } else {
                IntermediateNode::Binding(Box::new(
                    BindingBody {
                        variable: obligation.dest,
                        body: IntermediateNode::Binding(Box::new(
                            BindingBody {
                                variable: obligation.src,
                                body
                            }
                        ))
                    }))
            }
        },
        ASTNode::Through(obligation) => {
            todo!();
        },
        ASTNode::And(obligation) | ASTNode::Or(obligation) | ASTNode::Conditional(obligation) => {
            todo!();
        },
    }
}

fn compile_ast<'a>(
    handlebars: &mut Handlebars,
    node: ASTNode<'a>,
    bindings: &Vec<VariableBinding>,
    registered_templates: &mut HashSet<&'a str>,
) -> String {
    let mut references: HashMap<ASTNode<'a>, HashSet<Variable<'a>>> = HashMap::new();
    determine_var_scope(
        &node,
        &mut references,
    );
    let mut visited: HashSet<Variable<'a>> = HashSet::new();
    construct_intermediate_rep(
        &node,
        &mut references,
        &mut visited,
    );

    // TODO some kind of error checking that vars in policy = vars in bindings
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

    let scope_res = compile_policy_scope(
        handlebars,
        policy_body.scope,
        &bindings,
        &mut registered_templates,
    );

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

    fs::write("compiled-policy.rs", &res)?;
    Ok(())
}

pub fn compile<'a>(policy_body: PolicyBody<'a>, env: Vec<VariableBinding>) -> Result<()> {
    let mut handlebars = Handlebars::new();
    handlebars.register_escape_fn(no_escape);
    compile_policy(&mut handlebars, policy_body, env)
}
