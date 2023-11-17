policy!(community_prop, ctx {
    let db_write = marker!(db_write);
    let community = marker!(community);
    let community_delete_check = marker!(community_delete_check);
    let community_ban_check = marker!(community_ban_check);

    for c_id in ctx.desc().controllers.keys() {
        for community_struct in ctx
            .all_nodes_for_ctrl(*c_id)
            .filter(|n| ctx.has_marker(community, *n))
        {
            let mut delete_checks = ctx
                .all_nodes_for_ctrl(*c_id)
                .filter(|n| ctx.has_marker(community_delete_check, *n));

            let mut ban_checks = ctx
                .all_nodes_for_ctrl(*c_id)
                .filter(|n| ctx.has_marker(community_ban_check, *n));


            let delete_authorized_writes : Vec<Node> = ctx
                .all_nodes_for_ctrl(*c_id)
                .filter(|n|
                    ctx.has_marker(db_write, *n) &&
                    ctx.flows_to(community_struct, *n, EdgeType::Data) &&
                    delete_checks.any(|auth| ctx.has_ctrl_flow_influence(auth, *n))).collect();

            let delete_ok = !delete_authorized_writes.is_empty();

            if !delete_ok {
                bail!("Found a failure");
            }

            let ban_authorized_writes : Vec<Node> = ctx
                .all_nodes_for_ctrl(*c_id)
                .filter(|n|
                    ctx.has_marker(db_write, *n) &&
                    ctx.flows_to(community_struct, *n, EdgeType::Data) &&
                    ban_checks.any(|auth| ctx.has_ctrl_flow_influence(auth, *n))).collect();

            let ban_ok = !ban_authorized_writes.is_empty();

            assert_error!(ctx, ban_ok, "Unauthorized commmunity write: no ban check");
            if !ban_ok {
                bail!("Found a failure");
            }
        }
    }
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
