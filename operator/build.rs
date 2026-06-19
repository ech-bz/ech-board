use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result};
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use kopium::{Derive, SchemaMode, TypeGenerator};

const CRDS: &[(&str, &str)] = &[
    ("../external-crds/pushsecret.yaml", "push_secret.rs"),
    ("../external-crds/externalsecret.yaml", "external_secret.rs"),
    (
        "../charts/ech-board-operator-crds/templates/echboardnetwork.yaml",
        "ech_board_network.rs",
    ),
];

fn main() -> Result<()> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").context("OUT_DIR not set")?);
    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").context("CARGO_MANIFEST_DIR not set")?);

    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("build.rs").display()
    );

    let generator = TypeGenerator::builder()
        .schema_mode(SchemaMode::Derived)
        .emit_docs(true)
        .smart_derive_elision(true)
        .derive(Derive::all("JsonSchema"))
        .derive(Derive::all("Default"))
        .build();

    for (input, output) in CRDS {
        let input_path = manifest_dir.join(input);
        println!("cargo:rerun-if-changed={}", input_path.display());
        let yaml = fs::read_to_string(&input_path)
            .with_context(|| format!("read {}", input_path.display()))?;
        let crd: CustomResourceDefinition = serde_saphyr::from_str(&yaml)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| format!("parse CRD from {}", input_path.display()))?;
        let code = generator
            .generate_rust_types_for(&crd, None::<&str>)
            .with_context(|| format!("kopium codegen for {}", input_path.display()))?;
        fs::write(out_dir.join(output), code).with_context(|| format!("write {output}"))?;
    }

    Ok(())
}
