include!("../../../scripts/xtask_post_bundle.rs");

fn main() -> nih_plug_xtask::Result<()> {
    nih_plug_xtask::main()?;
    let plugin = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned();
    let runtime = plugin.join("runtime/libonnxruntime.dylib");
    let licenses = plugin.join("licenses");
    post_bundle(
        "Dereverb",
        &[
            BundleCopy {
                source: &runtime,
                dest_dir: "Contents/Frameworks",
                sign_separately: true,
            },
            BundleCopy {
                source: &licenses,
                dest_dir: "Contents/Resources/licenses",
                sign_separately: false,
            },
        ],
    )
}
