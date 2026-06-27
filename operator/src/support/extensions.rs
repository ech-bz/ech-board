use crate::crds::{ExternalSecret, PushSecret};
use crate::error::Result;
use ech_k8s::{NamespacedApi, NodeState, ResourcesExt};
use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec, StatefulSet, StatefulSetSpec};
use k8s_openapi::api::batch::v1::Job;
use k8s_openapi::api::core::v1::{
    PersistentVolumeClaim, PersistentVolumeClaimSpec, PodTemplateSpec, Service, ServicePort,
    ServiceSpec, VolumeResourceRequirements,
};
use k8s_openapi::apimachinery::pkg::{
    api::resource::Quantity, apis::meta::v1::LabelSelector, util::intstr::IntOrString,
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
        template: PodTemplateSpec,
        storage_size: &str,
        storage_class_name: Option<String>,
    ) -> crate::error::Result<()> {
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
                template,
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
                            storage_class_name,
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
