// version of the policy the compiler outputs
policy!(community_prop, ctx {
    // always
    let mut db_write_nodes = marked_nodes(marker!(db_write));
    let mut community_struct_nodes = marked_nodes(marker!(community));
    let mut delete_check_nodes = marked_nodes(marker!(community_delete_check));
    let mut ban_check_nodes = marked_nodes(marker!(community_ban_check));

    // if community_struct
    community_struct_nodes.all(|community_struct| {
        // flows to write
        let community_writes : Vec<Node> = ctx
            .influencees(community_struct, EdgeType::Data)
            .filter(|n| db_write_nodes.contains(n))
            .collect();
        // then
        community_writes.all(|write| {
            delete_check_nodes.any(|delete_check| {
                // community struct flows to delete check and
                ctx.flows_to(community_struct, delete_check, EdgeType::Data) &&
                // delete check has ctrl flow influence on the write
                ctx.has_ctrl_influence(delete_check, write) &&

                ban_check_nodes.any(|ban_check| {
                    // community struct flows to ban check and
                    ctx.flows_to(community_struct, ban_check, EdgeType::Data) &&
                    // ban check has ctrl flow influence on the write
                    ctx.has_ctrl_influence(ban_check, write)
                })
            })
        })
    });
    Ok(())
});

// This is the ideal/optimized version of the policy
// Note that the delete / ban checks happen separately,
// which has better performance and allows for more helpful error messages
policy!(community_prop, ctx {
    let mut db_write_nodes = marked_nodes(marker!(db_write));
    let mut community_struct_nodes = marked_nodes(marker!(community));
    let mut delete_check_nodes = marked_nodes(marker!(community_delete_check));
    let mut ban_check_nodes = marked_nodes(marker!(community_ban_check));

    // if some community_struct
    community_struct_nodes.all(|community_struct| {
        // flows to some write
        let community_writes : Vec<Node> = ctx
            .influencees(community_struct, EdgeType::Data)
            .filter(|n| db_write_nodes.contains(n))
            .collect();
        // then
        community_writes.all(|write| {
            let has_delete_check = delete_check_nodes.any(|delete_check| {
                // community struct flows to delete check and
                ctx.flows_to(community_struct, delete_check, EdgeType::Data) &&
                // delete check has ctrl flow influence on the write
                ctx.has_ctrl_influence(delete_check, write)
            });

            assert_error!(ctx, has_delete_check, "Unauthorized community write: no delete check");

            let has_ban_check = ban_check_nodes.any(|ban_check| {
                // community struct flows to ban check and
                ctx.flows_to(community_struct, ban_check, EdgeType::Data) &&
                // ban check has ctrl flow influence on the write
                ctx.has_ctrl_influence(ban_check, write)
            });

            assert_error!(ctx, has_ban_check, "Unauthorized community write: no ban check");
        })
    })
    Ok(())
});

policy!(instance_prop, ctx {
    let user_read = marker!(db_user_read);
    let db_read = marker!(db_read);
    let db_write = marker!(db_write);
    let instance_delete_check = marker!(instance_delete_check);
    let instance_ban_check = marker!(instance_ban_check);

    for c_id in ctx.desc().controllers.keys() {
        for sink in ctx
            .all_nodes_for_ctrl(*c_id)
            .filter(|n| ((ctx.has_marker(db_read, *n) && !ctx.has_marker(user_read, *n)) || ctx.has_marker(db_write, *n)))
        {
            let mut delete_checks =
                ctx.all_nodes_for_ctrl(*c_id)
                .filter(|n| ctx.has_marker(instance_delete_check, *n));

            let mut ban_checks = ctx
                .all_nodes_for_ctrl(*c_id)
                .filter(|n| ctx.has_marker(instance_ban_check, *n));

            let delete_ok = delete_checks.any(|auth| ctx.has_ctrl_flow_influence(auth, sink));
            let ban_ok = ban_checks.any(|auth| ctx.has_ctrl_flow_influence(auth, sink));
            let ok = delete_ok && ban_ok;

            assert_error!(ctx, ok, "Missing ban or delete check for instance authorization");
            if !ok {
                bail!("Found a failure");
            }
        }
    }

    Ok(())
});
