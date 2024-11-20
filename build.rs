use anyhow::Result;
use vergen_gitcl::{Emitter, GitclBuilder};

#[tokio::main]
async fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=NULL");

    Emitter::default()
        .add_instructions(&GitclBuilder::all_git()?)?
        .emit()?;

    Ok(())
}
