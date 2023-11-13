use anyhow::Result;
use paralegal_policy::{assert_error, paralegal_spdg::Identifier, Context, EdgeType, Marker, Node};
use std::sync::Arc;

macro_rules! marker {
    ($name:ident) => {{
        lazy_static::lazy_static! {
            static ref MARKER: Marker = Identifier::new_intern(stringify!($name));
        }
        *MARKER
    }};
}

macro_rules! policy {
    ($name:ident $(,)? $context:ident $(,)? $code:block) => {
        fn $name(ctx: Arc<Context>) -> Result<()> {
            ctx.named_policy(Identifier::new_intern(stringify!($name)), |$context| $code)
        }
    };
}

trait ContextExt {
    fn marked_nodes<'a>(&'a self, marker: Marker) -> Box<dyn Iterator<Item = Node<'a>> + 'a>;
}

impl ContextExt for Context {
    fn marked_nodes<'a>(&'a self, marker: Marker) -> Box<dyn Iterator<Item = Node<'a>> + 'a> {
        Box::new(
            self.desc()
                .controllers
                .keys()
                .copied()
                .flat_map(move |k| self.all_nodes_for_ctrl(k))
                .filter(move |node| self.has_marker(marker, *node)),
        )
    }
}

fn main() -> Result<()> {
    let dir = ".";
    let mut cmd = paralegal_policy::SPDGGenCommand::global();
    cmd.get_command()
        .args(["--inline-elision", "--model-version", "v2"]);

    cmd.run(dir)?.with_context(pol)?;
    println!("Policy successful");
    Ok(())
}

policy!(pol, ctx {
            let mut a_nodes = ctx.marked_nodes(marker!(a));
            let mut b_nodes = ctx.marked_nodes(marker!(b));
            assert_error!(ctx, a_nodes.any(|a| b_nodes.any(|b| ctx.flows_to(a, b, EdgeType::Data))));
            Ok(())
        });