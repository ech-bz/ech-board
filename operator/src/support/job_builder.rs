use k8s_openapi::api::{
    batch::v1::{Job, JobSpec},
    core::v1::{Container, PodSpec, PodTemplateSpec, Volume, VolumeMount},
};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use std::collections::BTreeMap;

pub(crate) struct JobBuilder {
    pub image: String,
    pub name: String,
    pub namespace: String,
    pub args: Vec<String>,
    pub labels: BTreeMap<String, String>,
    pub volumes: Vec<Volume>,
    pub mounts: Vec<VolumeMount>,
    pub service_account_name: String,
}

impl JobBuilder {
    pub(crate) fn build(self) -> Job {
        Job {
            metadata: ObjectMeta {
                name: Some(self.name.clone()),
                namespace: Some(self.namespace),
                labels: Some(self.labels.clone()),
                ..Default::default()
            },
            spec: Some(JobSpec {
                backoff_limit: Some(1),
                ttl_seconds_after_finished: Some(3600),
                template: PodTemplateSpec {
                    metadata: Some(ObjectMeta {
                        labels: Some(self.labels),
                        ..Default::default()
                    }),
                    spec: Some(PodSpec {
                        containers: vec![Container {
                            name: self.name,
                            image: Some(self.image),
                            image_pull_policy: Some("IfNotPresent".into()),
                            args: Some(self.args),
                            volume_mounts: if self.mounts.is_empty() {
                                None
                            } else {
                                Some(self.mounts)
                            },
                            ..Default::default()
                        }],
                        volumes: if self.volumes.is_empty() {
                            None
                        } else {
                            Some(self.volumes)
                        },
                        service_account_name: Some(self.service_account_name),
                        restart_policy: Some("Never".into()),
                        ..Default::default()
                    }),
                },
                ..Default::default()
            }),
            ..Default::default()
        }
    }
}
