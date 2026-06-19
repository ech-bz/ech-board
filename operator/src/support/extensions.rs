use crate::config::OperatorSettings;
use crate::constants::{WORKER_CONFIG_DIR, WORKER_CONFIG_FILE_NAME, WORKER_SERVICE_ACCOUNT_NAME};
use crate::crds::{EchBoardNetwork, ExternalSecret, PushSecret};
use crate::error::Result;
use crate::support::components::WorkerConfigComponent;
use ech_k8s::{Component, CrMeta, K8sClient, NamespacedApi, NodeState, ResourcesExt, StoreExt};
use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec, StatefulSet, StatefulSetSpec};
use k8s_openapi::api::batch::v1::{Job, JobSpec};
use k8s_openapi::api::core::v1::{
    Container, EmptyDirVolumeSource, EnvVar, PersistentVolumeClaim, PersistentVolumeClaimSpec,
    PodSpec, PodTemplateSpec, Secret, SecretVolumeSource, Service, ServicePort, ServiceSpec,
    Volume, VolumeMount, VolumeResourceRequirements,
};
use k8s_openapi::apimachinery::pkg::{
    api::resource::Quantity, apis::meta::v1::LabelSelector, util::intstr::IntOrString,
};
use kube::api::ObjectMeta;
use serde::Serialize;
use std::collections::BTreeMap;

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
        labels: BTreeMap<String, String>,
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
        labels: BTreeMap<String, String>,
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
        name: impl AsRef<str> + Send + Sync,
        labels: BTreeMap<String, String>,
        template: PodTemplateSpec,
        storage_size: &str,
        storage_class_name: Option<String>,
    ) -> crate::error::Result<()> {
        let sts = StatefulSet {
            metadata: ObjectMeta {
                name: Some(name.as_ref().to_string()),
                labels: Some(labels.clone()),
                ..Default::default()
            },
            spec: Some(StatefulSetSpec {
                service_name: Some(name.as_ref().to_string()),
                replicas: Some(1),
                selector: LabelSelector {
                    match_labels: Some(labels.clone()),
                    ..Default::default()
                },
                template,
                volume_claim_templates: Some(vec![PersistentVolumeClaim {
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
                        storage_class_name,
                        ..Default::default()
                    }),
                    ..Default::default()
                }]),
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
        owner: impl AsRef<str> + Send + Sync,
        cluster: impl Component,
        remote: impl Component,
        keys: &[impl AsRef<str> + Send + Sync],
    ) -> std::result::Result<(), kube::Error> {
        self.apply(
            cluster.name(owner.as_ref()),
            &PushSecret {
                metadata: ObjectMeta {
                    name: Some(cluster.name(owner.as_ref())),
                    labels: Some(cluster.labels(owner.as_ref())),
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
                            name: cluster.name(owner.as_ref()),
                        },
                    },
                    data: keys
                        .iter()
                        .map(|k| crate::crds::PushSecretData {
                            r#match: crate::crds::PushSecretDataMatch {
                                secret_key: k.as_ref().into(),
                                remote_ref: crate::crds::PushSecretDataMatchRemoteRef {
                                    remote_key: remote.name(owner.as_ref()),
                                },
                            },
                        })
                        .collect(),
                },
                ..Default::default()
            },
        )
        .await
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
        owner: impl AsRef<str> + Send + Sync,
        cluster: impl Component,
        remote: impl Component,
        keys: &[(
            impl AsRef<str> + Send + Sync,
            Option<impl AsRef<str> + Send + Sync>,
        )],
    ) -> std::result::Result<(), kube::Error> {
        self.apply(
            cluster.name(owner.as_ref()),
            &ExternalSecret {
                metadata: ObjectMeta {
                    name: Some(cluster.name(owner.as_ref())),
                    labels: Some(cluster.labels(owner.as_ref())),
                    ..Default::default()
                },
                spec: crate::crds::ExternalSecretSpec {
                    refresh_interval: Some("1h".into()),
                    secret_store_ref: Some(crate::crds::ExternalSecretSecretStoreRef {
                        name: Some("backend".to_string()),
                        kind: Some(
                            crate::crds::ExternalSecretSecretStoreRefKind::ClusterSecretStore,
                        ),
                    }),
                    target: Some(crate::crds::ExternalSecretTarget {
                        name: Some(cluster.name(owner.as_ref())),
                        creation_policy: Some(
                            crate::crds::ExternalSecretTargetCreationPolicy::Owner,
                        ),
                        deletion_policy: Some(
                            crate::crds::ExternalSecretTargetDeletionPolicy::Retain,
                        ),
                        ..Default::default()
                    }),
                    data: Some(
                        keys.iter()
                            .map(
                                |(secret_key, remote_property)| crate::crds::ExternalSecretData {
                                    secret_key: secret_key.as_ref().into(),
                                    remote_ref: crate::crds::ExternalSecretDataRemoteRef {
                                        key: remote.name(owner.as_ref()),
                                        property: remote_property
                                            .as_ref()
                                            .map(|p| p.as_ref().into()),
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
            },
        )
        .await
    }
}

#[async_trait::async_trait]
impl<'a> ExternalSecretExt for NamespacedApi<'a, ExternalSecret> where
    NamespacedApi<'a, ExternalSecret>: ResourcesExt<ExternalSecret>
{
}

#[async_trait::async_trait]
pub(crate) trait WorkerExt {
    async fn run_worker(
        &self,
        settings: &OperatorSettings,
        network: &EchBoardNetwork,
        command: impl AsRef<str> + Send + Sync,
        job_component: impl Component,
        out_component: impl Component,
        config: impl Serialize + Send + Sync,
    ) -> Result<NodeState>;
}

#[async_trait::async_trait]
impl<'a> WorkerExt for K8sClient {
    async fn run_worker(
        &self,
        settings: &OperatorSettings,
        network: &EchBoardNetwork,
        command: impl AsRef<str> + Send + Sync,
        job_component: impl Component,
        out_component: impl Component,
        config: impl Serialize + Send + Sync,
    ) -> Result<NodeState> {
        let ns = network.cr_ns()?;
        let owner = network.cr_name()?;

        self.namespaced::<Secret>(&ns)
            .store_put(
                WorkerConfigComponent(job_component).name(&owner),
                WorkerConfigComponent(job_component).labels(&owner),
                BTreeMap::from([(
                    WORKER_CONFIG_FILE_NAME.to_string(),
                    serde_json::to_string(&config)?,
                )]),
            )
            .await?;

        let spec = PodSpec {
            containers: vec![Container {
                name: job_component.name(&owner),
                image: Some(settings.worker_image.clone()),
                image_pull_policy: Some("IfNotPresent".into()),
                args: Some(vec![
                    "--config".to_string(),
                    format!("{WORKER_CONFIG_DIR}/{WORKER_CONFIG_FILE_NAME}"),
                    command.as_ref().to_string(),
                ]),
                volume_mounts: Some(vec![
                    VolumeMount {
                        name: WorkerConfigComponent(job_component).name(&owner),
                        mount_path: WORKER_CONFIG_DIR.into(),
                        read_only: Some(true),
                        ..Default::default()
                    },
                    VolumeMount {
                        name: "tmp".to_string(),
                        mount_path: "/tmp".to_string(),
                        read_only: Some(false),
                        ..Default::default()
                    },
                    VolumeMount {
                        name: "s3proxy-ca".to_string(),
                        mount_path: "/certs".to_string(),
                        read_only: Some(true),
                        ..Default::default()
                    },
                ]),
                env: Some(vec![EnvVar {
                    name: "SSL_CERT_FILE".into(),
                    value: Some("/certs/tls.crt".into()),
                    ..Default::default()
                }]),
                ..Default::default()
            }],
            volumes: Some(vec![
                Volume {
                    name: WorkerConfigComponent(job_component).name(&owner),
                    secret: Some(SecretVolumeSource {
                        secret_name: Some(WorkerConfigComponent(job_component).name(&owner)),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                Volume {
                    name: "tmp".to_string(),
                    empty_dir: Some(EmptyDirVolumeSource {
                        medium: Some("Memory".to_string()),
                        size_limit: None,
                    }),
                    ..Default::default()
                },
                Volume {
                    name: "s3proxy-ca".to_string(),
                    secret: Some(SecretVolumeSource {
                        secret_name: Some("s3proxy-ca-secret".into()),
                        optional: Some(false),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            ]),
            service_account_name: Some(WORKER_SERVICE_ACCOUNT_NAME.to_string()),
            restart_policy: Some("Never".into()),
            ..Default::default()
        };

        match self
            .namespaced::<Secret>(&ns)
            .api()
            .get_opt(&out_component.name(&owner))
            .await?
        {
            None => {
                match self
                    .namespaced::<Job>(&ns)
                    .api()
                    .get_opt(&job_component.name(&owner))
                    .await?
                {
                    None => {
                        self.namespaced::<Job>(&ns)
                            .apply(
                                job_component.name(&owner),
                                &Job {
                                    metadata: ObjectMeta {
                                        name: Some(job_component.name(&owner)),
                                        labels: Some(job_component.labels(&owner)),
                                        namespace: Some(ns.clone()),
                                        ..Default::default()
                                    },
                                    spec: Some(JobSpec {
                                        backoff_limit: Some(1),
                                        ttl_seconds_after_finished: Some(60),
                                        template: PodTemplateSpec {
                                            metadata: Some(ObjectMeta {
                                                labels: Some(job_component.labels(&owner)),
                                                ..Default::default()
                                            }),
                                            spec: Some(spec),
                                        },
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                },
                            )
                            .await?
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
                                "job {} failed",
                                job_component.name(&owner)
                            )));
                        }
                    }
                }
                Ok(NodeState::Pending)
            }
            Some(_) => Ok(NodeState::Ready),
        }
    }
}
