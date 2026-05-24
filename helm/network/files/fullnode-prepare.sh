#!/usr/bin/env bash
set -euo pipefail

OUTPUT_CONFIG="${OUTPUT_CONFIG:?OUTPUT_CONFIG is required}"
OUTPUT_KEYS_DIR="${OUTPUT_KEYS_DIR:?OUTPUT_KEYS_DIR is required}"
SEED_PEERS_FILE="${SEED_PEERS_FILE:?SEED_PEERS_FILE is required}"
FULLNODE_RPC_PORT="${FULLNODE_RPC_PORT:?FULLNODE_RPC_PORT is required}"
FULLNODE_METRICS_PORT="${FULLNODE_METRICS_PORT:?FULLNODE_METRICS_PORT is required}"
FULLNODE_P2P_PORT="${FULLNODE_P2P_PORT:?FULLNODE_P2P_PORT is required}"
FULLNODE_NET_PORT="${FULLNODE_NET_PORT:?FULLNODE_NET_PORT is required}"
FULLNODE_ADMIN_PORT="${FULLNODE_ADMIN_PORT:?FULLNODE_ADMIN_PORT is required}"
VALIDATOR_LIST_FILE="${VALIDATOR_LIST_FILE:?VALIDATOR_LIST_FILE is required}"

[ -s "$VALIDATOR_LIST_FILE" ] || { echo "validator list is empty: $VALIDATOR_LIST_FILE" >&2; exit 1; }

if [[ ! -s "$SEED_PEERS_FILE" ]]; then
  echo "seed peers file is missing or empty: $SEED_PEERS_FILE" >&2
  exit 1
fi

init_sui_layout
mkdir -p "$OUTPUT_KEYS_DIR"

protocol_key_file="${OUTPUT_KEYS_DIR}/protocol-key-pair"
worker_key_file="${OUTPUT_KEYS_DIR}/worker-key-pair"
account_key_file="${OUTPUT_KEYS_DIR}/account-key-pair"
network_key_file="${OUTPUT_KEYS_DIR}/network-key-pair"

printf '%s' "$(generate_keypair_bls12381_b64)" > "$protocol_key_file"
printf '%s' "$(generate_keypair_ed25519_b64)" > "$worker_key_file"
printf '%s' "$(generate_keypair_ed25519_b64)" > "$account_key_file"
printf '%s' "$(generate_keypair_ed25519_b64)" > "$network_key_file"

protocol_key_pair="$(cat "$protocol_key_file")"
worker_key_pair="$(cat "$worker_key_file")"
account_key_pair="$(cat "$account_key_file")"
network_key_pair="$(cat "$network_key_file")"

cat >"$OUTPUT_CONFIG" <<EOF
protocol-key-pair:
  value: ${protocol_key_pair}
worker-key-pair:
  value: ${worker_key_pair}
account-key-pair:
  value: ${account_key_pair}
network-key-pair:
  value: ${network_key_pair}
db-path: /data/fullnode-db
network-address: /ip4/127.0.0.1/tcp/${FULLNODE_NET_PORT}/https
json-rpc-address: "0.0.0.0:${FULLNODE_RPC_PORT}"
rpc:
  enable-indexing: true
metrics-address: "0.0.0.0:${FULLNODE_METRICS_PORT}"
admin-interface-port: ${FULLNODE_ADMIN_PORT}
enable-index-processing: true
sync-post-process-one-tx: false
jsonrpc-server-type: ~
grpc-load-shed: ~
grpc-concurrency-limit: ~
p2p-config:
  listen-address: "0.0.0.0:${FULLNODE_P2P_PORT}"
  external-address: /ip4/127.0.0.1/udp/${FULLNODE_P2P_PORT}
EOF

cat "$SEED_PEERS_FILE" >>"$OUTPUT_CONFIG"

cat >>"$OUTPUT_CONFIG" <<'EOF'
  state-sync:
    checkpoint-content-timeout-ms: 10000
genesis:
  genesis-file-location: /config/genesis.blob
EOF
