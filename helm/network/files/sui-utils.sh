SUI_CONFIG_DIR="${SUI_CONFIG_DIR}"
SUI_KEYS_DIR="${SUI_CONFIG_DIR}/keys"

init_sui_layout() {
  mkdir -p "$SUI_CONFIG_DIR" "$SUI_KEYS_DIR"
  cat >"${SUI_CONFIG_DIR}/client.yaml" <<EOF
---
keystore:
  File: ${SUI_CONFIG_DIR}/sui.keystore
envs: []
active_env: ~
active_address: ~
EOF
}

strip_scheme_flag_b64() {
  local key_b64="${1:?key_b64 is required}"
  printf '%s' "$key_b64" | base64 -d 2>/dev/null | dd bs=1 skip=1 2>/dev/null | base64 -w0
}

generate_keypair_b64() {
  local scheme="${1:?scheme is required}"
  local generated_json
  generated_json="$(cd "$SUI_KEYS_DIR" && sui keytool generate "$scheme" --json 2>&1)"
  printf '%s\n' "$generated_json" | awk -F'"' '/"suiAddress"/ {print $4; exit}'
}

generate_keypair_ed25519_b64() {
  local addr
  addr="$(generate_keypair_b64 ed25519)"
  local key_file="${SUI_KEYS_DIR}/${addr}.key"
  if [[ -z "$addr" || ! -f "$key_file" ]]; then
    echo "failed to generate ed25519 keypair" >&2
    exit 1
  fi
  cat "$key_file"
}

generate_keypair_bls12381_b64() {
  local addr
  addr="$(generate_keypair_b64 bls12381)"
  local key_file="${SUI_KEYS_DIR}/bls-${addr}.key"
  if [[ -z "$addr" || ! -f "$key_file" ]]; then
    echo "failed to generate bls12381 keypair" >&2
    exit 1
  fi
  cat "$key_file"
}
