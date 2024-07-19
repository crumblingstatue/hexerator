use {
    std::error::Error,
    vergen_gitcl::{BuildBuilder, CargoBuilder, Emitter, GitclBuilder, RustcBuilder},
};

fn main() -> Result<(), Box<dyn Error>> {
    let gitcl = GitclBuilder::default().sha(false).commit_timestamp(true).build()?;
    let build = BuildBuilder::default().build_timestamp(true).build()?;
    let cargo = CargoBuilder::default()
        .target_triple(true)
        .debug(true)
        .opt_level(true)
        .build()?;
    let rustc = RustcBuilder::default().semver(true).build()?;
    Emitter::default()
        .add_instructions(&gitcl)?
        .add_instructions(&build)?
        .add_instructions(&cargo)?
        .add_instructions(&rustc)?
        .emit()?;
    Ok(())
}
