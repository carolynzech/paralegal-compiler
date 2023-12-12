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
        let mut card_nodes = marked_nodes(marker!(credit_card));
    let mut sink_nodes = marked_nodes(marker!(store));
    let mut consent_nodes = marked_nodes(marker!(future_usage_decision));

    card_nodes.all(|card| {
        let sink_nodes_that_meet_condition : Vec<Node> = ctx
                .influencees(card, EdgeType::Data)
                .filter(|n| sink_nodes.contains(n))
                .collect();

        let is_compliant = sink_nodes_that_meet_condition.all(|sink| {
            consent_nodes.any(|consent|
                ctx.has_ctrl_influence(consent, sink)
        )
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
