use crate::constants::{
    DB_PATH, GENESIS_MOUNT_DIR, NODE_CONFIG_KEY, PVC_PATH, S3_CREDS_DIR, SUI_CONFIG_DIR,
};
use crate::error::Result;
use crate::support::components::S3CredsComponent;
use ech_board_common::keys::{S3_ACCESS_KEY, S3_SECRET_KEY};
use ech_k8s::{Component, CrMeta};
use k8s_openapi::api::core::v1::{
    Affinity, Container, ContainerPort, EmptyDirVolumeSource, EnvVar,
    EnvVarSource, PodAffinityTerm, PodAntiAffinity, PodSecurityContext, PodSpec,
    ResourceRequirements, SecretKeySelector, SecretVolumeSource, Volume, VolumeMount,
    WeightedPodAffinityTerm,
};
use k8s_openapi::apimachinery::pkg::{
    api::resource::Quantity, apis::meta::v1::LabelSelector,
    apis::meta::v1::LabelSelectorRequirement,
};
use std::collections::BTreeMap;

pub(crate) struct SuiNodePodBuilder<'a> {
    pub image: String,
    pub component_name: String,
    pub config_secret_name: String,
    pub ports: Vec<ContainerPort>,
    pub cpu: String,
    pub memory: String,
    pub enable_db_snapshot_download: bool,
    pub network: &'a crate::crds::EchBoardNetwork,
}

impl<'a> SuiNodePodBuilder<'a> {
    pub(crate) fn build(self) -> Result<PodSpec> {
        let network_name = self.network.cr_name()?;
        let s3_creds_secret = S3CredsComponent.instance_name(&network_name)?;
        let endpoint = self.network.spec.archive.endpoint.clone();
        let bucket = self.network.spec.archive.bucket.clone();
        let overrides = &self.network.spec.protocol_overrides;
        let image = self.image.clone();

        let mut init_containers = vec![Container {
            name: "download-genesis".into(),
            image: Some("minio/mc".into()),
            image_pull_policy: Some("IfNotPresent".into()),
            command: Some(vec!["/bin/sh".into(), "-ceu".into()]),
            args: Some(vec![format!(
                "mc alias set s3 \"{endpoint}\" \"$(cat {S3_CREDS_DIR}/{S3_ACCESS_KEY})\" \"$(cat {S3_CREDS_DIR}/{S3_SECRET_KEY})\" >/dev/null && mc get --quiet \"s3/{bucket}/genesis.blob\" \"{GENESIS_MOUNT_DIR}/genesis.blob\""
            )]),
            env: Some(vec![
                EnvVar {
                    name: "ECH_BOARD_S3_ENDPOINT".into(),
                    value: Some(endpoint.clone()),
                    ..Default::default()
                },
                EnvVar {
                    name: "ECH_BOARD_S3_BUCKET".into(),
                    value: Some(bucket.clone()),
                    ..Default::default()
                },
            ]),
            volume_mounts: Some(vec![
                VolumeMount {
                    name: "s3-creds".into(),
                    mount_path: S3_CREDS_DIR.into(),
                    read_only: Some(true),
                    ..Default::default()
                },
                VolumeMount {
                    name: "genesis".into(),
                    mount_path: GENESIS_MOUNT_DIR.into(),
                    read_only: Some(false),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        }];

        if self.enable_db_snapshot_download {
            init_containers.push(Container {
                name: "download-db".into(),
                image: Some("minio/mc".into()),
                image_pull_policy: Some("IfNotPresent".into()),
                command: Some(vec!["/bin/sh".into(), "-ceu".into()]),
                args: Some(vec![format!(
                    "mc alias set s3 \"$ECH_BOARD_S3_ENDPOINT\" \"$(cat {S3_CREDS_DIR}/{S3_ACCESS_KEY})\" \"$(cat {S3_CREDS_DIR}/{S3_SECRET_KEY})\" >/dev/null; LATEST=\"\"; while IFS= read -r line; do name=\"${{line##* }}\"; name=\"${{name%/}}\"; case \"$name\" in epoch_*) LATEST=\"$name\";; esac; done <<ENDOFLIST\n$(mc ls \"s3/$ECH_BOARD_S3_BUCKET/\" 2>/dev/null)\nENDOFLIST\n[ -n \"$LATEST\" ] && mc cp --recursive \"s3/$ECH_BOARD_S3_BUCKET/$LATEST/\" \"{DB_PATH}/live/\""
                )]),
                env: Some(vec![
                    EnvVar {
                        name: "ECH_BOARD_S3_ENDPOINT".into(),
                        value: Some(endpoint.clone()),
                        ..Default::default()
                    },
                    EnvVar {
                        name: "ECH_BOARD_S3_BUCKET".into(),
                        value: Some(bucket.clone()),
                        ..Default::default()
                    },
                ]),
                volume_mounts: Some(vec![
                    VolumeMount {
                        name: "s3-creds".into(),
                        mount_path: S3_CREDS_DIR.into(),
                        read_only: Some(true),
                        ..Default::default()
                    },
                    VolumeMount {
                        name: "data".into(),
                        mount_path: PVC_PATH.into(),
                        read_only: Some(false),
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            });
        }

        let volumes = vec![
            Volume {
                name: "config".into(),
                secret: Some(SecretVolumeSource {
                    secret_name: Some(self.config_secret_name),
                    optional: Some(false),
                    ..Default::default()
                }),
                ..Default::default()
            },
            Volume {
                name: "s3-creds".into(),
                secret: Some(SecretVolumeSource {
                    secret_name: Some(s3_creds_secret.clone()),
                    optional: Some(false),
                    ..Default::default()
                }),
                ..Default::default()
            },
            Volume {
                name: "genesis".into(),
                empty_dir: Some(EmptyDirVolumeSource::default()),
                ..Default::default()
            },
        ];

        Ok(PodSpec {
            termination_grace_period_seconds: Some(30),
            security_context: Some(PodSecurityContext {
                fs_group: Some(1000),
                ..Default::default()
            }),
            init_containers: Some(init_containers),
            containers: vec![Container {
                name: self.component_name,
                image: Some(image),
                image_pull_policy: Some("IfNotPresent".into()),
                command: Some(vec!["sui-node".into()]),
                resources: Some(ResourceRequirements {
                    requests: Some(BTreeMap::from([
                        ("cpu".into(), Quantity(self.cpu)),
                        ("memory".into(), Quantity(self.memory)),
                    ])),
                    ..Default::default()
                }),
                args: Some(vec![
                    "--config-path".into(),
                    format!("{SUI_CONFIG_DIR}/{NODE_CONFIG_KEY}"),
                ]),
                volume_mounts: Some(vec![
                    VolumeMount {
                        name: "data".into(),
                        mount_path: PVC_PATH.into(),
                        read_only: Some(false),
                        ..Default::default()
                    },
                    VolumeMount {
                        name: "config".into(),
                        mount_path: SUI_CONFIG_DIR.into(),
                        read_only: Some(true),
                        ..Default::default()
                    },
                    VolumeMount {
                        name: "genesis".into(),
                        mount_path: GENESIS_MOUNT_DIR.into(),
                        read_only: Some(true),
                        ..Default::default()
                    },
                    VolumeMount {
                        name: "s3-creds".into(),
                        mount_path: S3_CREDS_DIR.into(),
                        read_only: Some(true),
                        ..Default::default()
                    },
                ]),
                ports: Some(self.ports),
                env: Some(vec![
                    EnvVar {
                        name: "SUI_PROTOCOL_CONFIG_OVERRIDE_ENABLE".into(),
                        value: Some("1".into()),
                        ..Default::default()
                    },
                    EnvVar {
                        name: "SUI_PROTOCOL_CONFIG_OVERRIDE_max_gas_computation_bucket".into(),
                        value: Some(overrides.max_gas_computation_bucket.clone()),
                        ..Default::default()
                    },
                    EnvVar {
                        name: "SUI_PROTOCOL_CONFIG_OVERRIDE_base_tx_cost_fixed".into(),
                        value: Some(overrides.base_tx_cost_fixed.clone()),
                        ..Default::default()
                    },
                    EnvVar {
                        name: "SUI_PROTOCOL_CONFIG_OVERRIDE_max_num_new_move_object_ids".into(),
                        value: Some(overrides.max_num_new_move_object_ids.clone()),
                        ..Default::default()
                    },
                    EnvVar {
                        name: "AWS_ACCESS_KEY_ID".into(),
                        value_from: Some(EnvVarSource {
                            secret_key_ref: Some(SecretKeySelector {
                                key: S3_ACCESS_KEY.into(),
                                name: s3_creds_secret.clone(),
                                optional: Some(false),
                            }),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    EnvVar {
                        name: "AWS_SECRET_ACCESS_KEY".into(),
                        value_from: Some(EnvVarSource {
                            secret_key_ref: Some(SecretKeySelector {
                                key: S3_SECRET_KEY.into(),
                                name: s3_creds_secret.clone(),
                                optional: Some(false),
                            }),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            }],
            affinity: Some(Affinity {
                pod_anti_affinity: Some(PodAntiAffinity {
                    preferred_during_scheduling_ignored_during_execution: Some(vec![
                        WeightedPodAffinityTerm {
                            weight: 100,
                            pod_affinity_term: PodAffinityTerm {
                                label_selector: Some(LabelSelector {
                                    match_expressions: Some(vec![
                                        LabelSelectorRequirement {
                                            key: "ech.bz/owner".into(),
                                            operator: "Exists".into(),
                                            values: None,
                                        },
                                    ]),
                                    ..Default::default()
                                }),
                                topology_key: "kubernetes.io/hostname".into(),
                                ..Default::default()
                            },
                        },
                    ]),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            volumes: Some(volumes),
            ..Default::default()
        })
    }
}
