use traverse_contracts::{
    CanonicalProposal, ManifestReference, MappingSource, ParallelScheduleErrorCode,
    ParallelScheduleLimits, ProposalEdge, ProposalLimits, ProposalMapping, ProposalNode,
    ProposalValidationErrorCode, ProposalValidationFailure, SnapshotDigests, WorkflowProposal,
    canonicalize_proposal, compute_parallel_schedule, proposal_digest, proposal_snapshot_digest,
};

fn expect_failure(
    result: Result<CanonicalProposal, ProposalValidationFailure>,
) -> Result<ProposalValidationFailure, String> {
    match result {
        Ok(_) => Err("validation unexpectedly succeeded".to_string()),
        Err(failure) => Ok(failure),
    }
}

fn manifest_reference() -> ManifestReference {
    ManifestReference {
        app_id: "expedition-planner".to_string(),
        app_version: "1.0.0".to_string(),
        manifest_digest: "sha256:manifest-digest".to_string(),
    }
}

fn node(node_id: &str, capability_id: &str) -> ProposalNode {
    ProposalNode {
        node_id: node_id.to_string(),
        capability_id: capability_id.to_string(),
        capability_version: "1.0.0".to_string(),
        artifact_digest: format!("sha256:{capability_id}-digest"),
    }
}

fn linear_proposal() -> WorkflowProposal {
    WorkflowProposal {
        kind: "workflow_proposal".to_string(),
        schema_version: "1.0.0".to_string(),
        proposal_id: "proposal-001".to_string(),
        workspace_id: "workspace-001".to_string(),
        app_manifest: manifest_reference(),
        nodes: vec![
            node("a", "content.comments.create-comment-draft"),
            node("b", "content.comments.publish-comment"),
        ],
        edges: vec![ProposalEdge {
            from_node_id: "a".to_string(),
            to_node_id: "b".to_string(),
        }],
        mappings: vec![ProposalMapping {
            source: MappingSource::Node {
                node_id: "a".to_string(),
            },
            source_path: "/draft_id".to_string(),
            target_node_id: "b".to_string(),
            target_path: "/draft_id".to_string(),
        }],
        initial_input: serde_json::json!({"comment_text": "hello", "resource_id": "r1"}),
    }
}

fn diamond_proposal() -> WorkflowProposal {
    let mut proposal = linear_proposal();
    proposal.nodes = vec![
        node("a", "content.comments.create-comment-draft"),
        node("b", "content.comments.publish-comment"),
        node("c", "content.comments.publish-comment"),
        node("d", "content.comments.publish-comment"),
    ];
    proposal.edges = vec![
        ProposalEdge {
            from_node_id: "a".to_string(),
            to_node_id: "b".to_string(),
        },
        ProposalEdge {
            from_node_id: "a".to_string(),
            to_node_id: "c".to_string(),
        },
        ProposalEdge {
            from_node_id: "b".to_string(),
            to_node_id: "d".to_string(),
        },
        ProposalEdge {
            from_node_id: "c".to_string(),
            to_node_id: "d".to_string(),
        },
    ];
    proposal.mappings = Vec::new();
    proposal
}

#[test]
fn canonicalizes_a_valid_linear_proposal_in_dependency_order() -> Result<(), String> {
    let canonical = canonicalize_proposal(linear_proposal(), &ProposalLimits::default())
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        canonical.execution_order,
        vec!["a".to_string(), "b".to_string()]
    );
    Ok(())
}

#[test]
fn diamond_graph_uses_lexicographic_tie_break_among_ready_nodes() -> Result<(), String> {
    // b and c both depend only on a; neither depends on the other. The
    // deterministic tie-break must always pick b before c.
    let mut proposal = linear_proposal();
    proposal.nodes = vec![
        node("a", "content.comments.create-comment-draft"),
        node("c", "content.comments.publish-comment"),
        node("b", "content.comments.publish-comment"),
        node("d", "content.comments.publish-comment"),
    ];
    proposal.edges = vec![
        ProposalEdge {
            from_node_id: "a".to_string(),
            to_node_id: "b".to_string(),
        },
        ProposalEdge {
            from_node_id: "a".to_string(),
            to_node_id: "c".to_string(),
        },
        ProposalEdge {
            from_node_id: "b".to_string(),
            to_node_id: "d".to_string(),
        },
        ProposalEdge {
            from_node_id: "c".to_string(),
            to_node_id: "d".to_string(),
        },
    ];
    proposal.mappings = Vec::new();

    let canonical = canonicalize_proposal(proposal, &ProposalLimits::default())
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        canonical.execution_order,
        vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string()
        ]
    );
    Ok(())
}

#[test]
fn wide_fan_out_and_fan_in_graph_orders_deterministically() -> Result<(), String> {
    // a fans out to three independent successors (b, c, e), each of which
    // feeds the same terminal node d — exercising both the "in-degree drops
    // to exactly zero" and "in-degree merely decreases" branches multiple
    // times across a single node's outgoing edges.
    let mut proposal = linear_proposal();
    proposal.nodes = vec![
        node("a", "content.comments.create-comment-draft"),
        node("b", "content.comments.publish-comment"),
        node("c", "content.comments.publish-comment"),
        node("e", "content.comments.publish-comment"),
        node("d", "content.comments.publish-comment"),
    ];
    proposal.edges = vec![
        ProposalEdge {
            from_node_id: "a".to_string(),
            to_node_id: "b".to_string(),
        },
        ProposalEdge {
            from_node_id: "a".to_string(),
            to_node_id: "c".to_string(),
        },
        ProposalEdge {
            from_node_id: "a".to_string(),
            to_node_id: "e".to_string(),
        },
        ProposalEdge {
            from_node_id: "b".to_string(),
            to_node_id: "d".to_string(),
        },
        ProposalEdge {
            from_node_id: "c".to_string(),
            to_node_id: "d".to_string(),
        },
        ProposalEdge {
            from_node_id: "e".to_string(),
            to_node_id: "d".to_string(),
        },
    ];
    proposal.mappings = Vec::new();

    let canonical = canonicalize_proposal(proposal, &ProposalLimits::default())
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        canonical.execution_order,
        vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "e".to_string(),
            "d".to_string(),
        ]
    );
    Ok(())
}

#[test]
fn rejects_a_cyclic_graph() -> Result<(), String> {
    let mut proposal = linear_proposal();
    proposal.edges.push(ProposalEdge {
        from_node_id: "b".to_string(),
        to_node_id: "a".to_string(),
    });
    proposal.mappings = Vec::new();

    let failure = expect_failure(canonicalize_proposal(proposal, &ProposalLimits::default()))?;
    assert!(
        failure
            .errors
            .iter()
            .any(|e| e.code == ProposalValidationErrorCode::CyclicGraph)
    );
    Ok(())
}

#[test]
fn rejects_a_self_loop_edge() -> Result<(), String> {
    let mut proposal = linear_proposal();
    proposal.mappings = Vec::new();
    proposal.edges = vec![ProposalEdge {
        from_node_id: "a".to_string(),
        to_node_id: "a".to_string(),
    }];

    let failure = expect_failure(canonicalize_proposal(proposal, &ProposalLimits::default()))?;
    assert!(
        failure
            .errors
            .iter()
            .any(|e| e.code == ProposalValidationErrorCode::SelfLoopEdge)
    );
    Ok(())
}

#[test]
fn rejects_duplicate_node_ids() -> Result<(), String> {
    let mut proposal = linear_proposal();
    proposal
        .nodes
        .push(node("a", "content.comments.publish-comment"));

    let failure = expect_failure(canonicalize_proposal(proposal, &ProposalLimits::default()))?;
    assert!(
        failure
            .errors
            .iter()
            .any(|e| e.code == ProposalValidationErrorCode::DuplicateNodeId)
    );
    Ok(())
}

#[test]
fn rejects_an_edge_to_an_unknown_node() -> Result<(), String> {
    let mut proposal = linear_proposal();
    proposal.edges.push(ProposalEdge {
        from_node_id: "a".to_string(),
        to_node_id: "missing".to_string(),
    });

    let failure = expect_failure(canonicalize_proposal(proposal, &ProposalLimits::default()))?;
    assert!(
        failure
            .errors
            .iter()
            .any(|e| e.code == ProposalValidationErrorCode::UnknownEdgeEndpoint)
    );
    Ok(())
}

#[test]
fn rejects_a_duplicate_edge() -> Result<(), String> {
    let mut proposal = linear_proposal();
    proposal.edges.push(ProposalEdge {
        from_node_id: "a".to_string(),
        to_node_id: "b".to_string(),
    });

    let failure = expect_failure(canonicalize_proposal(proposal, &ProposalLimits::default()))?;
    assert!(
        failure
            .errors
            .iter()
            .any(|e| e.code == ProposalValidationErrorCode::DuplicateEdge)
    );
    Ok(())
}

#[test]
fn rejects_a_mapping_to_an_unknown_target_node() -> Result<(), String> {
    let mut proposal = linear_proposal();
    proposal.mappings[0].target_node_id = "missing".to_string();

    let failure = expect_failure(canonicalize_proposal(proposal, &ProposalLimits::default()))?;
    assert!(
        failure
            .errors
            .iter()
            .any(|e| e.code == ProposalValidationErrorCode::UnknownMappingEndpoint)
    );
    Ok(())
}

#[test]
fn rejects_a_mapping_from_an_unknown_source_node() -> Result<(), String> {
    let mut proposal = linear_proposal();
    proposal.mappings[0].source = MappingSource::Node {
        node_id: "missing".to_string(),
    };

    let failure = expect_failure(canonicalize_proposal(proposal, &ProposalLimits::default()))?;
    assert!(
        failure
            .errors
            .iter()
            .any(|e| e.code == ProposalValidationErrorCode::UnknownMappingEndpoint)
    );
    Ok(())
}

#[test]
fn rejects_a_mapping_with_no_corresponding_declared_edge() -> Result<(), String> {
    let mut proposal = linear_proposal();
    proposal.edges.clear();

    let failure = expect_failure(canonicalize_proposal(proposal, &ProposalLimits::default()))?;
    assert!(
        failure
            .errors
            .iter()
            .any(|e| e.code == ProposalValidationErrorCode::MissingDependencyEdgeForMapping)
    );
    Ok(())
}

#[test]
fn accepts_a_mapping_from_initial_input_with_no_edge_required() -> Result<(), String> {
    let mut proposal = linear_proposal();
    proposal.mappings = vec![ProposalMapping {
        source: MappingSource::InitialInput,
        source_path: "/comment_text".to_string(),
        target_node_id: "a".to_string(),
        target_path: "/comment_text".to_string(),
    }];
    proposal.edges.clear();

    canonicalize_proposal(proposal, &ProposalLimits::default()).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[test]
fn rejects_an_ambiguous_multi_writer_target_path() -> Result<(), String> {
    let mut proposal = linear_proposal();
    proposal.mappings.push(ProposalMapping {
        source: MappingSource::InitialInput,
        source_path: "/comment_text".to_string(),
        target_node_id: "b".to_string(),
        target_path: "/draft_id".to_string(),
    });

    let failure = expect_failure(canonicalize_proposal(proposal, &ProposalLimits::default()))?;
    assert!(
        failure
            .errors
            .iter()
            .any(|e| e.code == ProposalValidationErrorCode::AmbiguousMultiWriterTarget)
    );
    Ok(())
}

#[test]
fn rejects_a_proposal_over_the_configured_node_limit() -> Result<(), String> {
    let proposal = linear_proposal();
    let limits = ProposalLimits {
        max_nodes: 1,
        ..ProposalLimits::default()
    };

    let failure = expect_failure(canonicalize_proposal(proposal, &limits))?;
    assert!(
        failure
            .errors
            .iter()
            .any(|e| e.code == ProposalValidationErrorCode::NodeLimitExceeded)
    );
    Ok(())
}

#[test]
fn rejects_a_proposal_over_the_configured_edge_limit() -> Result<(), String> {
    let proposal = linear_proposal();
    let limits = ProposalLimits {
        max_edges: 0,
        ..ProposalLimits::default()
    };

    let failure = expect_failure(canonicalize_proposal(proposal, &limits))?;
    assert!(
        failure
            .errors
            .iter()
            .any(|e| e.code == ProposalValidationErrorCode::EdgeLimitExceeded)
    );
    Ok(())
}

#[test]
fn rejects_a_proposal_over_the_configured_mapping_limit() -> Result<(), String> {
    let proposal = linear_proposal();
    let limits = ProposalLimits {
        max_mappings: 0,
        ..ProposalLimits::default()
    };

    let failure = expect_failure(canonicalize_proposal(proposal, &limits))?;
    assert!(
        failure
            .errors
            .iter()
            .any(|e| e.code == ProposalValidationErrorCode::MappingLimitExceeded)
    );
    Ok(())
}

#[test]
fn rejects_an_edge_from_an_unknown_node() -> Result<(), String> {
    let mut proposal = linear_proposal();
    proposal.edges.push(ProposalEdge {
        from_node_id: "missing".to_string(),
        to_node_id: "b".to_string(),
    });

    let failure = expect_failure(canonicalize_proposal(proposal, &ProposalLimits::default()))?;
    assert!(
        failure
            .errors
            .iter()
            .any(|e| e.code == ProposalValidationErrorCode::UnknownEdgeEndpoint)
    );
    Ok(())
}

#[test]
fn rejects_empty_required_fields() -> Result<(), String> {
    let mut proposal = linear_proposal();
    proposal.proposal_id = String::new();
    proposal.workspace_id = String::new();
    proposal.nodes[0].node_id = String::new();
    proposal.nodes[0].capability_id = String::new();

    let failure = expect_failure(canonicalize_proposal(proposal, &ProposalLimits::default()))?;
    assert!(
        failure
            .errors
            .iter()
            .filter(|e| e.code == ProposalValidationErrorCode::MissingRequiredField)
            .count()
            >= 4
    );
    Ok(())
}

#[test]
fn proposal_digest_is_stable_for_non_string_initial_input_values() {
    let mut proposal = linear_proposal();
    proposal.initial_input = serde_json::json!({
        "count": 42,
        "enabled": true,
        "note": serde_json::Value::Null,
        "nested": [1, false, null]
    });

    let digest_one = proposal_digest(&proposal);
    let digest_two = proposal_digest(&proposal);
    assert_eq!(digest_one, digest_two);
}

#[test]
fn rejects_a_proposal_over_the_configured_initial_input_byte_limit() -> Result<(), String> {
    let proposal = linear_proposal();
    let limits = ProposalLimits {
        max_initial_input_bytes: 4,
        ..ProposalLimits::default()
    };

    let failure = expect_failure(canonicalize_proposal(proposal, &limits))?;
    assert!(
        failure
            .errors
            .iter()
            .any(|e| e.code == ProposalValidationErrorCode::PayloadLimitExceeded)
    );
    Ok(())
}

#[test]
fn rejects_wrong_kind_and_schema_version() -> Result<(), String> {
    let mut proposal = linear_proposal();
    proposal.kind = "wrong".to_string();
    proposal.schema_version = "9.9.9".to_string();

    let failure = expect_failure(canonicalize_proposal(proposal, &ProposalLimits::default()))?;
    let codes: Vec<_> = failure.errors.iter().map(|e| e.code).collect();
    assert_eq!(
        codes
            .iter()
            .filter(|c| **c == ProposalValidationErrorCode::InvalidLiteral)
            .count(),
        2
    );
    Ok(())
}

#[test]
fn proposal_digest_is_stable_and_field_order_independent() -> Result<(), String> {
    let proposal = linear_proposal();
    let digest_one = proposal_digest(&proposal);
    let digest_two = proposal_digest(&proposal);
    assert_eq!(digest_one, digest_two);

    // Round-trip through JSON with reordered object keys must not change the digest.
    let value = serde_json::to_value(&proposal).map_err(|e| e.to_string())?;
    let json_text = serde_json::to_string(&value).map_err(|e| e.to_string())?;
    let reparsed: WorkflowProposal = serde_json::from_str(&json_text).map_err(|e| e.to_string())?;
    assert_eq!(proposal_digest(&reparsed), digest_one);
    Ok(())
}

#[test]
fn proposal_digest_changes_when_content_changes() {
    let mut proposal = linear_proposal();
    let original = proposal_digest(&proposal);
    proposal.initial_input = serde_json::json!({"comment_text": "different", "resource_id": "r1"});
    assert_ne!(proposal_digest(&proposal), original);
}

fn snapshots() -> SnapshotDigests {
    SnapshotDigests {
        manifest_digest: "manifest-1".to_string(),
        registry_digest: "registry-1".to_string(),
        binding_digest: "binding-1".to_string(),
        policy_digest: "policy-1".to_string(),
        budget_digest: "budget-1".to_string(),
    }
}

#[test]
fn snapshot_digest_changes_when_any_pinned_snapshot_changes_even_if_proposal_is_identical() {
    let digest = proposal_digest(&linear_proposal());
    let base = proposal_snapshot_digest(&digest, &snapshots());

    let mut changed = snapshots();
    changed.policy_digest = "policy-2".to_string();
    let with_changed_policy = proposal_snapshot_digest(&digest, &changed);

    assert_ne!(base, with_changed_policy);
}

// -- Parallel scheduling (spec 110 P2) ---------------------------------------

#[test]
fn compute_parallel_schedule_levelizes_a_linear_proposal_into_singleton_waves() -> Result<(), String>
{
    let canonical = canonicalize_proposal(linear_proposal(), &ProposalLimits::default())
        .map_err(|e| format!("{e:?}"))?;
    let schedule = compute_parallel_schedule(&canonical, &ParallelScheduleLimits::default())
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        schedule.waves,
        vec![vec!["a".to_string()], vec!["b".to_string()]]
    );
    Ok(())
}

#[test]
fn compute_parallel_schedule_levelizes_the_diamond_graph_into_a_concurrent_wave()
-> Result<(), String> {
    let canonical = canonicalize_proposal(diamond_proposal(), &ProposalLimits::default())
        .map_err(|e| format!("{e:?}"))?;
    let schedule = compute_parallel_schedule(&canonical, &ParallelScheduleLimits::default())
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        schedule.waves,
        vec![
            vec!["a".to_string()],
            vec!["b".to_string(), "c".to_string()],
            vec!["d".to_string()],
        ]
    );
    Ok(())
}

#[test]
fn compute_parallel_schedule_levelizes_the_wide_fan_out_graph() -> Result<(), String> {
    let mut proposal = diamond_proposal();
    proposal
        .nodes
        .push(node("e", "content.comments.publish-comment"));
    proposal.edges.push(ProposalEdge {
        from_node_id: "a".to_string(),
        to_node_id: "e".to_string(),
    });
    proposal.edges.push(ProposalEdge {
        from_node_id: "e".to_string(),
        to_node_id: "d".to_string(),
    });

    let canonical = canonicalize_proposal(proposal, &ProposalLimits::default())
        .map_err(|e| format!("{e:?}"))?;
    let schedule = compute_parallel_schedule(&canonical, &ParallelScheduleLimits::default())
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        schedule.waves,
        vec![
            vec!["a".to_string()],
            vec!["b".to_string(), "c".to_string(), "e".to_string()],
            vec!["d".to_string()],
        ]
    );
    Ok(())
}

#[test]
fn rejects_a_schedule_exceeding_the_configured_fan_out_limit() -> Result<(), String> {
    let canonical = canonicalize_proposal(diamond_proposal(), &ProposalLimits::default())
        .map_err(|e| format!("{e:?}"))?;
    let limits = ParallelScheduleLimits {
        max_fan_out: 1,
        ..ParallelScheduleLimits::default()
    };
    let Err(failure) = compute_parallel_schedule(&canonical, &limits) else {
        return Err("a fan-out over the configured limit must be rejected".to_string());
    };
    assert!(
        failure
            .errors
            .iter()
            .any(|e| e.code == ParallelScheduleErrorCode::FanOutExceeded)
    );
    Ok(())
}

#[test]
fn rejects_a_schedule_exceeding_the_configured_join_width_limit() -> Result<(), String> {
    let canonical = canonicalize_proposal(diamond_proposal(), &ProposalLimits::default())
        .map_err(|e| format!("{e:?}"))?;
    let limits = ParallelScheduleLimits {
        max_join_width: 1,
        ..ParallelScheduleLimits::default()
    };
    let Err(failure) = compute_parallel_schedule(&canonical, &limits) else {
        return Err("a join width over the configured limit must be rejected".to_string());
    };
    assert!(
        failure
            .errors
            .iter()
            .any(|e| e.code == ParallelScheduleErrorCode::JoinWidthExceeded)
    );
    Ok(())
}

#[test]
fn rejects_a_schedule_exceeding_the_configured_queue_depth_limit() -> Result<(), String> {
    let canonical = canonicalize_proposal(diamond_proposal(), &ProposalLimits::default())
        .map_err(|e| format!("{e:?}"))?;
    let limits = ParallelScheduleLimits {
        max_queue_depth: 1,
        ..ParallelScheduleLimits::default()
    };
    let Err(failure) = compute_parallel_schedule(&canonical, &limits) else {
        return Err("a queue depth over the configured limit must be rejected".to_string());
    };
    assert!(
        failure
            .errors
            .iter()
            .any(|e| e.code == ParallelScheduleErrorCode::QueueDepthExceeded)
    );
    Ok(())
}

#[test]
fn compute_parallel_schedule_reports_every_exceeded_bound_in_one_pass() -> Result<(), String> {
    let canonical = canonicalize_proposal(diamond_proposal(), &ProposalLimits::default())
        .map_err(|e| format!("{e:?}"))?;
    let limits = ParallelScheduleLimits {
        max_fan_out: 1,
        max_join_width: 1,
        max_queue_depth: 1,
        max_concurrent_nodes: 1,
    };
    let Err(failure) = compute_parallel_schedule(&canonical, &limits) else {
        return Err("every configured bound is violated and must be rejected".to_string());
    };
    let codes: std::collections::BTreeSet<_> = failure.errors.iter().map(|e| e.code).collect();
    assert!(codes.contains(&ParallelScheduleErrorCode::FanOutExceeded));
    assert!(codes.contains(&ParallelScheduleErrorCode::JoinWidthExceeded));
    assert!(codes.contains(&ParallelScheduleErrorCode::QueueDepthExceeded));
    Ok(())
}
