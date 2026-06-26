use crate::constants::{
    GENESIS_MOUNT_DIR, NODE_CONFIG_KEY, PVC_PATH, S3_CREDS_DIR, SUI_CONFIG_DIR,
};
use crate::crds::{ExternalSecret, PushSecret};
use crate::error::Result;
use crate::support::components::S3CredsComponent;
use ech_board_common::keys::{S3_ACCESS_KEY, S3_SECRET_KEY};
use ech_k8s::{Component, CrMeta, NamespacedApi, NodeState, ResourcesExt};
use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec, StatefulSet, StatefulSetSpec};
use k8s_openapi::api::batch::v1::Job;
use k8s_openapi::api::core::v1::{
    Affinity, Container, EmptyDirVolumeSource, EnvVar, EnvVarSource, PersistentVolumeClaim,
    PersistentVolumeClaimSpec, PodAffinityTerm, PodAntiAffinity, PodSecurityContext, PodSpec,
    PodTemplateSpec, ResourceRequirements, SecretKeySelector, SecretVolumeSource, Service,
    ServicePort, ServiceSpec, Volume, VolumeMount, VolumeResourceRequirements,
    WeightedPodAffinityTerm,
};
use k8s_openapi::apimachinery::pkg::{
    api::resource::Quantity, apis::meta::v1::LabelSelector,
    apis::meta::v1::LabelSelectorRequirement, util::intstr::IntOrString,
};
use kube::ResourceExt;
use kube::api::ObjectMeta;
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;

#[async_trait::async_trait]
pub(crate) trait JobArtifactExt: ResourcesExt<Job> {
    async fn reconcile_job_artifact<'a, ArtifactReady>(
        &self,
        job: Job,
        artifact_ready: ArtifactReady,
    ) -> Result<NodeState>
    where
        ArtifactReady:
            Fn() -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'a>> + Send + Sync + 'a;
}

#[async_trait::async_trait]
impl<'a> JobArtifactExt for NamespacedApi<'a, Job>
where
    NamespacedApi<'a, Job>: ResourcesExt<Job>,
{
    async fn reconcile_job_artifact<'b, ArtifactReady>(
        &self,
        job: Job,
        artifact_ready: ArtifactReady,
    ) -> Result<NodeState>
    where
        ArtifactReady:
            Fn() -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'b>> + Send + Sync + 'b,
    {
        let job_name = job.name_any();

        let namespace = self.api().namespace().ok_or_else(|| {
            crate::error::OperatorError::ControllerFatal(
                "job reconciliation requires a namespaced handle".to_string(),
            )
        })?;

        if artifact_ready().await? {
            return Ok(NodeState::Ready);
        }

        match self.api().get_opt(&job_name).await? {
            None => {
                tracing::info!(name = %job_name, namespace, "creating job");
                self.apply(job_name, &job).await?;
                Ok(NodeState::Pending)
            }
            Some(job) => {
                if job
                    .status
                    .as_ref()
                    .and_then(|status| status.failed)
                    .unwrap_or_default()
                    > 0
                {
                    return Err(crate::error::OperatorError::ControllerFatal(format!(
                        "job {job_name} failed"
                    )));
                }
                if job
                    .status
                    .as_ref()
                    .and_then(|status| status.succeeded)
                    .unwrap_or_default()
                    > 0
                    && !artifact_ready().await?
                {
                    return Err(crate::error::OperatorError::ControllerFatal(format!(
                        "job {job_name} completed but the artifact is still missing"
                    )));
                }
                Ok(NodeState::Pending)
            }
        }
    }
}

#[async_trait::async_trait]
pub(crate) trait ReadyReplicasExt {
    async fn ready_replicas(&self, name: impl AsRef<str> + Send + Sync) -> Result<i64>;
}

#[async_trait::async_trait]
impl<'a> ReadyReplicasExt for NamespacedApi<'a, Deployment> {
    async fn ready_replicas(&self, name: impl AsRef<str> + Send + Sync) -> Result<i64> {
        let Some(obj): Option<Deployment> = self.api().get_opt(name.as_ref()).await? else {
            return Ok(0);
        };
        Ok(obj
            .status
            .as_ref()
            .and_then(|status| status.ready_replicas)
            .unwrap_or_default() as i64)
    }
}

#[async_trait::async_trait]
impl<'a> ReadyReplicasExt for NamespacedApi<'a, StatefulSet> {
    async fn ready_replicas(&self, name: impl AsRef<str> + Send + Sync) -> Result<i64> {
        let Some(obj): Option<StatefulSet> = self.api().get_opt(name.as_ref()).await? else {
            return Ok(0);
        };
        Ok(obj
            .status
            .as_ref()
            .and_then(|status| status.ready_replicas)
            .unwrap_or_default() as i64)
    }
}

#[async_trait::async_trait]
pub(crate) trait HeadlessServiceExt: ResourcesExt<Service> {
    async fn apply_headless_service(
        &self,
        name: impl AsRef<str> + Send + Sync,
        labels: &BTreeMap<String, String>,
        port_name: impl AsRef<str> + Send + Sync,
        port: i32,
    ) -> std::result::Result<(), kube::Error> {
        let name = name.as_ref();
        let service = Service {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                labels: Some(labels.clone()),
                ..Default::default()
            },
            spec: Some(ServiceSpec {
                cluster_ip: Some("None".into()),
                publish_not_ready_addresses: Some(true),
                selector: Some(labels.clone()),
                ports: Some(vec![ServicePort {
                    name: Some(port_name.as_ref().to_string()),
                    port,
                    target_port: Some(IntOrString::Int(port)),
                    ..Default::default()
                }]),
                ..Default::default()
            }),
            ..Default::default()
        };
        self.apply(name, &service).await
    }
}

#[async_trait::async_trait]
impl<'a> HeadlessServiceExt for NamespacedApi<'a, Service> where
    NamespacedApi<'a, Service>: ResourcesExt<Service>
{
}

#[async_trait::async_trait]
pub(crate) trait DeploymentExt: ResourcesExt<Deployment> {
    async fn apply_deployment(
        &self,
        name: impl AsRef<str> + Send + Sync,
        labels: &BTreeMap<String, String>,
        replicas: i64,
        template: PodTemplateSpec,
    ) -> std::result::Result<(), kube::Error> {
        let name = name.as_ref();
        let deployment = Deployment {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                labels: Some(labels.clone()),
                ..Default::default()
            },
            spec: Some(DeploymentSpec {
                replicas: Some(replicas as i32),
                selector: LabelSelector {
                    match_labels: Some(labels.clone()),
                    ..Default::default()
                },
                template,
                ..Default::default()
            }),
            ..Default::default()
        };
        self.apply(name, &deployment).await
    }
}

#[async_trait::async_trait]
impl<'a> DeploymentExt for NamespacedApi<'a, Deployment> where
    NamespacedApi<'a, Deployment>: ResourcesExt<Deployment>
{
}

#[async_trait::async_trait]
pub(crate) trait SingletonStatefulSetExt: ResourcesExt<StatefulSet> {
    async fn apply_singleton_stateful_set(
        &self,
        name: &str,
        labels: &BTreeMap<String, String>,
        component: &str,
        config_secret_name: &str,
        ports: Vec<k8s_openapi::api::core::v1::ContainerPort>,
        storage_size: &str,
        storage_class_name: Option<String>,
        cpu: &str,
        memory: &str,
        network: &crate::crds::EchBoardNetwork,
    ) -> crate::error::Result<()> {
        let network_name = network.cr_name()?;
        let sts = StatefulSet {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                labels: Some(labels.clone()),
                ..Default::default()
            },
            spec: Some(StatefulSetSpec {
                service_name: Some(name.to_string()),
                replicas: Some(1),
                selector: LabelSelector {
                    match_labels: Some(labels.clone()),
                    ..Default::default()
                },
                template: PodTemplateSpec {
                    metadata: Some(ObjectMeta {
                        labels: Some(labels.clone()),
                        ..Default::default()
                    }),
                    spec: Some(PodSpec {
                        termination_grace_period_seconds: Some(30),
                        security_context: Some(PodSecurityContext {
                            fs_group: Some(1000),
                            ..Default::default()
                        }),
                        init_containers: Some(vec![Container {
                            name: "download-genesis".into(),
                            image: Some("minio/mc".into()),
                            image_pull_policy: Some("IfNotPresent".into()),
                            command: Some(vec!["/bin/sh".into(), "-ceu".into()]),
                            args: Some(vec![format!(
                                "mc alias set s3 \"$ECH_BOARD_S3_ENDPOINT\" \"$(cat {S3_CREDS_DIR}/{S3_ACCESS_KEY})\" \"$(cat {S3_CREDS_DIR}/{S3_SECRET_KEY})\" >/dev/null && mc get --quiet \"s3/$ECH_BOARD_S3_BUCKET/genesis.blob\" \"{GENESIS_MOUNT_DIR}/genesis.blob\""
                            )]),
                            env: Some(vec![
                                EnvVar {
                                    name: "ECH_BOARD_S3_ENDPOINT".into(),
                                    value: Some(network.spec.archive.endpoint.clone()),
                                    ..Default::default()
                                },
                                EnvVar {
                                    name: "ECH_BOARD_S3_BUCKET".into(),
                                    value: Some(network.spec.archive.bucket.clone()),
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
                        }]),
                        containers: vec![Container {
                            name: component.into(),
                            image: Some(network.spec.images.sui_node.clone()),
                            image_pull_policy: Some("IfNotPresent".into()),
                            resources: Some(ResourceRequirements {
                                requests: Some(BTreeMap::from([
                                    ("cpu".into(), Quantity(cpu.to_string())),
                                    ("memory".into(), Quantity(memory.to_string())),
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
                            ports: Some(ports),
                            env: Some(vec![
                                EnvVar {
                                    name: "SUI_PROTOCOL_CONFIG_OVERRIDE_ENABLE".into(),
                                    value: Some("1".into()),
                                    ..Default::default()
                                },
                                EnvVar {
                                    name: "SUI_PROTOCOL_CONFIG_OVERRIDE_max_gas_computation_bucket"
                                        .into(),
                                    value: Some(
                                        network
                                            .spec
                                            .protocol_overrides
                                            .max_gas_computation_bucket
                                            .clone(),
                                    ),
                                    ..Default::default()
                                },
                                EnvVar {
                                    name: "SUI_PROTOCOL_CONFIG_OVERRIDE_base_tx_cost_fixed".into(),
                                    value: Some(
                                        network.spec.protocol_overrides.base_tx_cost_fixed.clone(),
                                    ),
                                    ..Default::default()
                                },
                                EnvVar {
                                    name:
                                        "SUI_PROTOCOL_CONFIG_OVERRIDE_max_num_new_move_object_ids"
                                            .into(),
                                    value: Some(
                                        network
                                            .spec
                                            .protocol_overrides
                                            .max_num_new_move_object_ids
                                            .clone(),
                                    ),
                                    ..Default::default()
                                },
                                EnvVar {
                                    name: "AWS_ACCESS_KEY_ID".into(),
                                    value_from: Some(EnvVarSource {
                                        secret_key_ref: Some(SecretKeySelector {
                                            key: S3_ACCESS_KEY.into(),
                                            name: S3CredsComponent.instance_name(&network_name)?,
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
                                            name: S3CredsComponent.instance_name(&network_name)?,
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
                        volumes: Some(vec![
                            Volume {
                                name: "config".into(),
                                secret: Some(SecretVolumeSource {
                                    secret_name: Some(config_secret_name.to_string()),
                                    optional: Some(false),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            },
                            Volume {
                                name: "s3-creds".into(),
                                secret: Some(SecretVolumeSource {
                                    secret_name: Some(
                                        S3CredsComponent.instance_name(&network_name)?,
                                    ),
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
                        ]),
                        ..Default::default()
                    }),
                },
                volume_claim_templates: Some(vec![
                    PersistentVolumeClaim {
                        metadata: ObjectMeta {
                            name: Some("data".into()),
                            ..Default::default()
                        },
                        spec: Some(PersistentVolumeClaimSpec {
                            access_modes: Some(vec!["ReadWriteOnce".into()]),
                            resources: Some(VolumeResourceRequirements {
                                requests: Some(BTreeMap::from([(
                                    "storage".into(),
                                    Quantity(storage_size.to_string()),
                                )])),
                                ..Default::default()
                            }),
                            storage_class_name: storage_class_name.clone(),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            }),
            ..Default::default()
        };
        self.apply(name, &sts).await.map_err(Into::into)
    }
}

#[async_trait::async_trait]
impl<'a> SingletonStatefulSetExt for NamespacedApi<'a, StatefulSet> where
    NamespacedApi<'a, StatefulSet>: ResourcesExt<StatefulSet>
{
}

#[async_trait::async_trait]
pub(crate) trait PushSecretExt: ResourcesExt<PushSecret> {
    async fn apply_push_secret(
        &self,
        push_name: impl AsRef<str> + Send + Sync,
        labels: BTreeMap<String, String>,
        cluster_name: impl AsRef<str> + Send + Sync,
        remote_name: impl AsRef<str> + Send + Sync,
        keys: &[impl AsRef<str> + Send + Sync],
    ) -> std::result::Result<(), kube::Error> {
        let push_name = push_name.as_ref();
        let cluster_name = cluster_name.as_ref();
        let remote_name = remote_name.as_ref();
        let push_secret = PushSecret {
            metadata: ObjectMeta {
                name: Some(push_name.to_string()),
                labels: Some(labels),
                ..Default::default()
            },
            spec: crate::crds::PushSecretSpec {
                update_policy: "IfNotExists".into(),
                deletion_policy: "None".into(),
                refresh_interval: "1h".into(),
                secret_store_refs: vec![crate::crds::PushSecretSecretStoreRefs {
                    name: "backend".into(),
                    kind: "ClusterSecretStore".into(),
                }],
                selector: crate::crds::PushSecretSelector {
                    secret: crate::crds::PushSecretSelectorSecret {
                        name: cluster_name.into(),
                    },
                },
                data: keys
                    .iter()
                    .map(|k| crate::crds::PushSecretData {
                        r#match: crate::crds::PushSecretDataMatch {
                            secret_key: k.as_ref().into(),
                            remote_ref: crate::crds::PushSecretDataMatchRemoteRef {
                                remote_key: remote_name.into(),
                            },
                        },
                    })
                    .collect(),
            },
            ..Default::default()
        };
        self.apply(push_name, &push_secret).await
    }
}

#[async_trait::async_trait]
impl<'a> PushSecretExt for NamespacedApi<'a, PushSecret> where
    NamespacedApi<'a, PushSecret>: ResourcesExt<PushSecret>
{
}

#[async_trait::async_trait]
pub(crate) trait ExternalSecretExt: ResourcesExt<ExternalSecret> {
    async fn apply_external_secret(
        &self,
        eso_name: impl AsRef<str> + Send + Sync,
        labels: BTreeMap<String, String>,
        cluster_name: impl AsRef<str> + Send + Sync,
        remote_name: impl AsRef<str> + Send + Sync,
        keys: &[(
            impl AsRef<str> + Send + Sync,
            Option<impl AsRef<str> + Send + Sync>,
        )],
    ) -> std::result::Result<(), kube::Error> {
        let eso_name = eso_name.as_ref();
        let cluster_name = cluster_name.as_ref();
        let remote_name = remote_name.as_ref();
        let external_secret = ExternalSecret {
            metadata: ObjectMeta {
                name: Some(eso_name.to_string()),
                labels: Some(labels),
                ..Default::default()
            },
            spec: crate::crds::ExternalSecretSpec {
                refresh_interval: Some("1h".into()),
                secret_store_ref: Some(crate::crds::ExternalSecretSecretStoreRef {
                    name: Some("backend".to_string()),
                    kind: Some(crate::crds::ExternalSecretSecretStoreRefKind::ClusterSecretStore),
                }),
                target: Some(crate::crds::ExternalSecretTarget {
                    name: Some(cluster_name.into()),
                    creation_policy: Some(crate::crds::ExternalSecretTargetCreationPolicy::Owner),
                    deletion_policy: Some(crate::crds::ExternalSecretTargetDeletionPolicy::Retain),
                    ..Default::default()
                }),
                data: Some(
                    keys.iter()
                        .map(
                            |(secret_key, remote_property)| crate::crds::ExternalSecretData {
                                secret_key: secret_key.as_ref().into(),
                                remote_ref: crate::crds::ExternalSecretDataRemoteRef {
                                    key: remote_name.into(),
                                    property: remote_property.as_ref().map(|p| p.as_ref().into()),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        )
                        .collect(),
                ),
                ..Default::default()
            },
            ..Default::default()
        };
        self.apply(eso_name, &external_secret).await
    }
}

#[async_trait::async_trait]
impl<'a> ExternalSecretExt for NamespacedApi<'a, ExternalSecret> where
    NamespacedApi<'a, ExternalSecret>: ResourcesExt<ExternalSecret>
{
}
