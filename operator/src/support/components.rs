use ech_k8s::Component;
use serde::Serialize;

#[derive(Clone, Serialize, Component)]
#[component(name = "s3-creds")]
pub(crate) struct S3CredsComponent;

#[derive(Clone, Serialize, Component)]
#[component(name = "pushsecret")]
pub(crate) struct PsComponent<C: Component>(pub(crate) C);

#[derive(Clone, Serialize, Component)]
#[component(name = "externalsecret")]
pub(crate) struct EsComponent<C: Component>(pub(crate) C);

#[derive(Clone, Serialize, Component)]
#[component(name = "worker-output")]
pub(crate) struct WorkerOutputComponent<C: Component>(pub(crate) C);
