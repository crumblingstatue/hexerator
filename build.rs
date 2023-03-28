fn main() {
    if let Err(e) = vergen::EmitBuilder::builder()
        .git_sha(false)
        .git_commit_timestamp()
        .build_timestamp()
        .cargo_target_triple()
        .cargo_debug()
        .cargo_opt_level()
        .rustc_semver()
        .emit()
    {
        println!("cargo:warning=Vergen failed with error: {e}");
    }
}
