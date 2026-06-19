use ech_k8s::Component;
use serde::Serialize;

#[derive(Clone, Copy, Serialize, Component)]
#[component(name = "worker-output")]
pub(crate) struct WorkerOutputComponent<C: Component>(pub(crate) C);

#[derive(Clone, Copy, Serialize, Component)]
#[component(name = "worker-config")]
pub(crate) struct WorkerConfigComponent<C: Component>(pub(crate) C);

#[derive(Clone, Copy, Serialize, Component)]
#[component(name = "download-genesis")]
pub(crate) struct WorkerDownloadGenesisComponent<C: Component>(pub(crate) C);

#[derive(Clone, Copy, Serialize, Component)]
#[component(name = "download-db")]
pub(crate) struct WorkerDownloadDbComponent<C: Component>(pub(crate) C);
