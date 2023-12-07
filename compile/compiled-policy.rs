use anyhow::Result;
use paralegal_policy::{
    assert_error, paralegal_spdg::Identifier, Context, Diagnostics, EdgeType, Marker, Node,
};
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

policy!(pol, ctx {
    let mut community_struct_nodes = marked_nodes(marker!(community));
    let mut delete_check_nodes = marked_nodes(marker!(community_delete_check));
    let mut ban_check_nodes = marked_nodes(marker!(community_ban_check));
    let mut write_nodes = marked_nodes(marker!(db_write));

    community_struct_nodes.all(|community_struct| {
    let write_nodes_that_meet_condition : Vec<Node> = ctx
            .influencees(community_struct, EdgeType::Data)
            .filter(|n| write_nodes.contains(n))
            .collect();

    let is_compliant = write_nodes_that_meet_condition.all(|write| {
        delete_check_nodes.any(|delete_check|
            ctx.flows_to(community_struct, delete_check, EdgeType::Data)
            && ctx.has_ctrl_influence(delete_check, write)
            && ban_check_nodes.any(|ban_check|
                ctx.flows_to(community_struct, ban_check, EdgeType::Data)
                && ctx.has_ctrl_influence(ban_check, write)
        ))
    });

    assert_error!(ctx, is_compliant, "Policy failed.");
    Ok(())
})
});

fn main() -> Result<()> {
    let dir = ".";
    let cmd = paralegal_policy::SPDGGenCommand::global();
    cmd.run(dir)?.with_context(pol)?;
    println!("Policy successful");
    Ok(())
}
