use crate::{
    config::OperatorSettings,
    crds::EchBoardNetwork,
    error::OperatorError,
    reconcilers::{
        bootstrap::BootstrapReconciler,
        fullnode_rpc::FullnodeRpcReconciler,
        genesis::GenesisReconciler,
        key_node::{
            KeyArchiveComponent, KeyFullnodeComponent, KeyNodeReconciler, KeySponsorComponent,
            KeyValidatorComponent,
        },
        move_publish::MovePublishReconciler,
        prune_fullnodes::PruneFullnodesReconciler,
        prune_keys::PruneKeysReconciler,
        prune_validators::PruneValidatorsReconciler,
        seed_peers::SeedPeersReconciler,
        workload_archive::WorkloadArchiveReconciler,
        workload_checkpoint_blob::WorkloadCheckpointBlobReconciler,
        workload_fullnodes::WorkloadFullnodeReconciler,
        workload_graphql::WorkloadGraphqlReconciler,
        workload_indexer_alt::WorkloadIndexerAltReconciler,
        workload_relay::WorkloadRelayReconciler,
        workload_validators::WorkloadValidatorReconciler,
    },
};
use ech_k8s::{Graph, GraphError, Workflow};

#[derive(Clone, Debug)]
pub(crate) struct BoardWorkflow {
    pub(crate) operator: OperatorSettings,
}

impl Workflow for BoardWorkflow {
    type Crd = EchBoardNetwork;
    type Error = OperatorError;

    fn build_graph(
        &self,
        cr: &EchBoardNetwork,
    ) -> Result<Graph<Self::Crd, Self::Error>, GraphError> {
        let mut graph = Graph::new();
        let validator_count = cr.spec.validator.replicas as usize;
        let fullnode_count = cr.spec.fullnode.replicas as usize;

        graph.add(BootstrapReconciler, vec![])?;
        graph.add(PruneKeysReconciler, vec![BootstrapReconciler.into()])?;
        graph.add(PruneValidatorsReconciler, vec![BootstrapReconciler.into()])?;
        graph.add(PruneFullnodesReconciler, vec![BootstrapReconciler.into()])?;
        graph.add(
            FullnodeRpcReconciler,
            (0..fullnode_count)
                .map(|ordinal| {
                    WorkloadFullnodeReconciler {
                        ordinal,
                        operator: self.operator.clone(),
                    }
                    .into()
                })
                .chain([PruneFullnodesReconciler.into()]),
        )?;

        for ordinal in 0..validator_count {
            graph.add(
                KeyNodeReconciler::new(KeyValidatorComponent { ordinal }, self.operator.clone()),
                vec![PruneKeysReconciler.into()],
            )?;
            graph.add(
                WorkloadValidatorReconciler {
                    ordinal,
                    operator: self.operator.clone(),
                },
                vec![
                    PruneValidatorsReconciler.into(),
                    KeyNodeReconciler::new(
                        KeyValidatorComponent { ordinal },
                        self.operator.clone(),
                    )
                    .into(),
                    GenesisReconciler::new(self.operator.clone()).into(),
                ],
            )?;
        }

        for ordinal in 0..fullnode_count {
            graph.add(
                KeyNodeReconciler::new(KeyFullnodeComponent { ordinal }, self.operator.clone()),
                vec![PruneKeysReconciler.into()],
            )?;
            graph.add(
                WorkloadFullnodeReconciler {
                    ordinal,
                    operator: self.operator.clone(),
                },
                vec![
                    PruneFullnodesReconciler.into(),
                    KeyNodeReconciler::new(KeyFullnodeComponent { ordinal }, self.operator.clone())
                        .into(),
                    SeedPeersReconciler::new(self.operator.clone()).into(),
                    GenesisReconciler::new(self.operator.clone()).into(),
                ],
            )?;
        }

        graph.add(
            KeyNodeReconciler::new(KeySponsorComponent, self.operator.clone()),
            vec![BootstrapReconciler.into()],
        )?;
        graph.add(
            GenesisReconciler::new(self.operator.clone()),
            (0..validator_count)
                .map(|ordinal| {
                    KeyNodeReconciler::new(KeyValidatorComponent { ordinal }, self.operator.clone())
                        .into()
                })
                .chain([KeyNodeReconciler::new(KeySponsorComponent, self.operator.clone()).into()]),
        )?;
        graph.add(
            SeedPeersReconciler::new(self.operator.clone()),
            vec![GenesisReconciler::new(self.operator.clone()).into()],
        )?;

        graph.add(
            KeyNodeReconciler::new(KeyArchiveComponent, self.operator.clone()),
            vec![BootstrapReconciler.into()],
        )?;
        graph.add(
            WorkloadArchiveReconciler {
                operator: self.operator.clone(),
            },
            vec![
                KeyNodeReconciler::new(KeyArchiveComponent, self.operator.clone()).into(),
                SeedPeersReconciler::new(self.operator.clone()).into(),
                GenesisReconciler::new(self.operator.clone()).into(),
            ],
        )?;
        graph.add(
            MovePublishReconciler::new(self.operator.clone()),
            vec![
                KeyNodeReconciler::new(KeySponsorComponent, self.operator.clone()).into(),
                FullnodeRpcReconciler.into(),
            ],
        )?;
        graph.add(
            WorkloadRelayReconciler::new(self.operator.clone()),
            vec![
                GenesisReconciler::new(self.operator.clone()).into(),
                MovePublishReconciler::new(self.operator.clone()).into(),
            ],
        )?;
        graph.add(
            WorkloadCheckpointBlobReconciler::new(self.operator.clone()),
            vec![
                WorkloadArchiveReconciler {
                    operator: self.operator.clone(),
                }
                .into(),
                GenesisReconciler::new(self.operator.clone()).into(),
            ],
        )?;

        graph.add(
            WorkloadIndexerAltReconciler::new(self.operator.clone()),
            vec![
                WorkloadArchiveReconciler {
                    operator: self.operator.clone(),
                }
                .into(),
                GenesisReconciler::new(self.operator.clone()).into(),
            ],
        )?;

        graph.add(
            WorkloadGraphqlReconciler::new(self.operator.clone()),
            vec![WorkloadIndexerAltReconciler::new(self.operator.clone()).into()],
        )?;

        Ok(graph)
    }
}
