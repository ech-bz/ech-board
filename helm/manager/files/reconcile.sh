#!/bin/sh
set -eu

DEFAULT_SECRET_STORE_NAME="${DEFAULT_SECRET_STORE_NAME:-backend}"
DEFAULT_SECRET_STORE_KIND="${DEFAULT_SECRET_STORE_KIND:-ClusterSecretStore}"

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "ERROR: required command not found: $1" >&2
    exit 1
  fi
}

validate_positive_int() {
  case "$1" in
    ''|*[!0-9]*|0)
      echo "ERROR: $2 must be a positive integer" >&2
      exit 1
      ;;
  esac
}

cr_get() {
  kubectl -n "$1" get echboardnetwork "$2" -o jsonpath="{${3}}"
}

cr_optional() {
  value="$(kubectl -n "$1" get echboardnetwork "$2" -o jsonpath="{${3}}" 2>/dev/null || true)"
  if [ "$value" = "<no value>" ]; then
    value=""
  fi
  printf '%s\n' "$value"
}

render_values() {
  values_file="$1"
  namespace="$2"
  validator_count="$3"
  images_sui_node="$4"
  images_sui_tools="$5"
  images_relay="$6"
  gitops_source_namespace="$7"
  gitops_source_name="$8"
  gitops_artifact_subdir="$9"
  relay_replicas="${10}"
  relay_port="${11}"
  relay_cpu="${12}"
  relay_memory="${13}"
  sponsor_gas_object_count="${14}"
  sponsor_gas_budget="${15}"
  sponsor_gas_price="${16}"
  fullnode_replicas="${17}"
  fullnode_port_rpc="${18}"
  fullnode_port_metrics="${19}"
  fullnode_port_p2p="${20}"
  fullnode_port_net="${21}"
  fullnode_port_admin="${22}"
  fullnode_cpu="${23}"
  fullnode_memory="${24}"
  fullnode_storage_size="${25}"
  fullnode_storage_class_name="${26}"
  validator_port_p2p="${27}"
  validator_cpu="${28}"
  validator_memory="${29}"
  validator_storage_size="${30}"
  validator_storage_class_name="${31}"
  protocol_base_tx_cost_fixed="${32}"
  protocol_storage_gas_price="${33}"

  cat >"$values_file" <<EOF
namespace: ${namespace}
images:
  sui_node: ${images_sui_node}
  sui_tools: ${images_sui_tools}
  relay: ${images_relay}
gitops:
  sourceNamespace: ${gitops_source_namespace}
  sourceName: ${gitops_source_name}
  artifactSubdir: ${gitops_artifact_subdir}
relay:
  replicas: ${relay_replicas}
  port: ${relay_port}
  cpu: ${relay_cpu}
  memory: ${relay_memory}
  sponsor:
    gas_object_count: ${sponsor_gas_object_count}
    gas_budget: "${sponsor_gas_budget}"
    gas_price: "${sponsor_gas_price}"
fullnode:
  replicas: ${fullnode_replicas}
  port_rpc: ${fullnode_port_rpc}
  port_metrics: ${fullnode_port_metrics}
  port_p2p: ${fullnode_port_p2p}
  port_net: ${fullnode_port_net}
  port_admin: ${fullnode_port_admin}
  cpu: ${fullnode_cpu}
  memory: ${fullnode_memory}
  storage:
    size: ${fullnode_storage_size}
    className: "${fullnode_storage_class_name}"
validator:
  replicas: ${validator_count}
  port_p2p: ${validator_port_p2p}
  cpu: ${validator_cpu}
  memory: ${validator_memory}
  storage:
    size: ${validator_storage_size}
    className: "${validator_storage_class_name}"
protocol_overrides:
  base_tx_cost_fixed: "${protocol_base_tx_cost_fixed}"
  storage_gas_price: "${protocol_storage_gas_price}"
secretStore:
  name: ${DEFAULT_SECRET_STORE_NAME}
  kind: ${DEFAULT_SECRET_STORE_KIND}
EOF
}

count_validators_from_seed_peers() {
  grep -o 'sui-validator-[0-9][0-9]*-0\.sui-validator' "$1" | sort -u | wc -l | tr -d ' '
}

upsert_configmap_from_files() {
  cm_namespace="$1"
  cm_name="$2"
  shift 2

  if kubectl -n "$cm_namespace" get configmap "$cm_name" >/dev/null 2>&1; then
    cm_manifest="$(mktemp)"
    kubectl -n "$cm_namespace" create configmap "$cm_name" "$@" --dry-run=client -o yaml > "$cm_manifest"
    kubectl -n "$cm_namespace" replace -f "$cm_manifest"
    rm -f "$cm_manifest"
  else
    kubectl -n "$cm_namespace" create configmap "$cm_name" "$@"
  fi
}

reconcile_one() {
  namespace="$1"
  network_name="$2"

  gitops_source_namespace="$(cr_get "$namespace" "$network_name" '.spec.gitops.sourceNamespace')"
  gitops_source_name="$(cr_get "$namespace" "$network_name" '.spec.gitops.sourceName')"
  gitops_artifact_subdir="$(cr_get "$namespace" "$network_name" '.spec.gitops.artifactSubdir')"

  artifact_url="$(kubectl -n "$gitops_source_namespace" get gitrepository "$gitops_source_name" -o jsonpath='{.status.artifact.url}' 2>/dev/null || true)"
  if [ -z "$artifact_url" ] || [ "$artifact_url" = "<no value>" ]; then
    echo "[$namespace/$network_name] waiting for Flux GitRepository artifact"
    return 0
  fi

  work_dir="$(mktemp -d)"
  artifact_dir="${work_dir}/artifact"
  artifact_tarball="${work_dir}/artifact.tar.gz"
  artifact_subdir="${gitops_artifact_subdir#/}"
  artifact_subdir="${artifact_subdir%/}"
  mkdir -p "$artifact_dir"
  trap 'rm -rf "$work_dir"' HUP INT TERM EXIT

  echo "[$namespace/$network_name] fetching ${artifact_url}"
  if ! wget -q -O "$artifact_tarball" "$artifact_url"; then
    echo "[$namespace/$network_name] failed to download Flux artifact from source-controller" >&2
    trap - HUP INT TERM EXIT
    rm -rf "$work_dir"
    return 1
  fi

  if ! tar -xzf "$artifact_tarball" -C "$artifact_dir"; then
    echo "[$namespace/$network_name] failed to unpack Flux artifact tarball" >&2
    trap - HUP INT TERM EXIT
    rm -rf "$work_dir"
    return 1
  fi

  case "$artifact_subdir" in
    ''|'.')
      genesis_file="$(find "$artifact_dir" -path "*/genesis.blob" -type f | head -n1 || true)"
      seed_peers_file="$(find "$artifact_dir" -path "*/seed-peers.yaml" -type f | head -n1 || true)"
      ;;
    *)
      genesis_file="$(find "$artifact_dir" -path "*/${artifact_subdir}/genesis.blob" -type f | head -n1 || true)"
      seed_peers_file="$(find "$artifact_dir" -path "*/${artifact_subdir}/seed-peers.yaml" -type f | head -n1 || true)"
      ;;
  esac

  if [ -z "$genesis_file" ] || [ -z "$seed_peers_file" ]; then
    echo "[$namespace/$network_name] missing generated genesis artifacts under artifactSubdir '${gitops_artifact_subdir}'" >&2
    trap - HUP INT TERM EXIT
    rm -rf "$work_dir"
    return 1
  fi

  validator_count="$(count_validators_from_seed_peers "$seed_peers_file")"
  validate_positive_int "$validator_count" "derived validator count"

  bootstrap_cm_name="${network_name}-ech-board-bootstrap"
  upsert_configmap_from_files "$namespace" "$bootstrap_cm_name" \
    --from-file=genesis.blob="$genesis_file" \
    --from-file=seed-peers.yaml="$seed_peers_file"

  values_file="${work_dir}/values.yaml"
  render_values \
    "$values_file" \
    "$namespace" \
    "$validator_count" \
    "$(cr_get "$namespace" "$network_name" '.spec.images.sui_node')" \
    "$(cr_get "$namespace" "$network_name" '.spec.images.sui_tools')" \
    "$(cr_get "$namespace" "$network_name" '.spec.images.relay')" \
    "$gitops_source_namespace" \
    "$gitops_source_name" \
    "$gitops_artifact_subdir" \
    "$(cr_get "$namespace" "$network_name" '.spec.relay.replicas')" \
    "$(cr_get "$namespace" "$network_name" '.spec.relay.port')" \
    "$(cr_get "$namespace" "$network_name" '.spec.relay.cpu')" \
    "$(cr_get "$namespace" "$network_name" '.spec.relay.memory')" \
    "$(cr_get "$namespace" "$network_name" '.spec.relay.sponsor.gas_object_count')" \
    "$(cr_get "$namespace" "$network_name" '.spec.relay.sponsor.gas_budget')" \
    "$(cr_get "$namespace" "$network_name" '.spec.relay.sponsor.gas_price')" \
    "$(cr_get "$namespace" "$network_name" '.spec.fullnode.replicas')" \
    "$(cr_get "$namespace" "$network_name" '.spec.fullnode.port_rpc')" \
    "$(cr_get "$namespace" "$network_name" '.spec.fullnode.port_metrics')" \
    "$(cr_get "$namespace" "$network_name" '.spec.fullnode.port_p2p')" \
    "$(cr_get "$namespace" "$network_name" '.spec.fullnode.port_net')" \
    "$(cr_get "$namespace" "$network_name" '.spec.fullnode.port_admin')" \
    "$(cr_get "$namespace" "$network_name" '.spec.fullnode.cpu')" \
    "$(cr_get "$namespace" "$network_name" '.spec.fullnode.memory')" \
    "$(cr_get "$namespace" "$network_name" '.spec.fullnode.storage.size')" \
    "$(cr_optional "$namespace" "$network_name" '.spec.fullnode.storage.className')" \
    "$(cr_get "$namespace" "$network_name" '.spec.validator.port_p2p')" \
    "$(cr_get "$namespace" "$network_name" '.spec.validator.cpu')" \
    "$(cr_get "$namespace" "$network_name" '.spec.validator.memory')" \
    "$(cr_get "$namespace" "$network_name" '.spec.validator.storage.size')" \
    "$(cr_optional "$namespace" "$network_name" '.spec.validator.storage.className')" \
    "$(cr_get "$namespace" "$network_name" '.spec.protocol_overrides.base_tx_cost_fixed')" \
    "$(cr_get "$namespace" "$network_name" '.spec.protocol_overrides.storage_gas_price')"

  values_cm_name="${network_name}-ech-board-values"
  upsert_configmap_from_files "$namespace" "$values_cm_name" \
    --from-file=values.yaml="$values_file"
  kubectl -n "$namespace" label configmap "$values_cm_name" reconcile.fluxcd.io/watch=Enabled --overwrite >/dev/null

  observed_generation="$(kubectl -n "$namespace" get echboardnetwork "$network_name" -o jsonpath='{.metadata.generation}')"
  kubectl -n "$namespace" patch echboardnetwork "$network_name" --subresource=status --type=merge -p "{\"status\":{\"phase\":\"Ready\",\"observedGeneration\":${observed_generation},\"validatorCount\":${validator_count},\"ready\":\"true\"}}" >/dev/null 2>&1 || true

  trap - HUP INT TERM EXIT
  rm -rf "$work_dir"

  echo "[$namespace/$network_name] reconciled bootstrap ConfigMap ${bootstrap_cm_name} and values ConfigMap ${values_cm_name}"
}

main() {
  require_command kubectl
  require_command wget
  require_command tar
  require_command grep
  require_command sort
  require_command wc
  require_command tr
  require_command mktemp
  require_command find
  require_command head

  networks="$(kubectl get echboardnetworks -A -o jsonpath='{range .items[*]}{.metadata.namespace}{" "}{.metadata.name}{"\n"}{end}' 2>/dev/null || true)"
  if [ -z "$networks" ]; then
    echo "No EchBoardNetwork resources found"
    return 0
  fi

  printf '%s\n' "$networks" | while IFS=' ' read -r ns name; do
    [ -n "$ns" ] || continue
    reconcile_one "$ns" "$name" || true
  done
}

main "$@"
