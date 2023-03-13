// Netdata plugin binary name is required to end with ".plugin" suffix.
//
// As it is impossible to have a crate or produce a binary containing a dot symbol using just cargo
// itself, it became necessary to wrap `cargo build` procedure into `xtask` to rename resulting
// binary after the build step.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

type DynError = Box<dyn std::error::Error>;

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{}", e);
        std::process::exit(-1);
    }
}

fn try_main() -> Result<(), DynError> {
    let task = env::args().nth(1);
    match task.as_deref() {
        Some("build") => build()?,
        _ => print_help(),
    }
    Ok(())
}

fn print_help() {
    eprintln!("Use `cargo xtask build` to build a properly-named-plugin-binary")
}

fn build() -> Result<(), DynError> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let status = Command::new(cargo)
        .env("XTASK_PLUGIN_BUILD", "")
        .current_dir(project_root())
        .args(["build", "--release"])
        .status()?;

    if !status.success() {
        Err("cargo build failed")?;
    }

    let src = target_dir().join("release/hs110-plugin");
    let dst = target_dir().join("release/hs110.plugin");
    fs::rename(src, dst.clone())?;

    println!(
        "\n====\n\
        Plugin has been built: {dst:?}\n\n\
        Place the resulting binary into netdata's `custom-plugins.d` directory.\n\
        Which is usually `/etc/netdata/custom-plugins.d` or\n\
        `/usr/local/etc/netdata/custom-plugins.d`\n\n\
        Then create a `/etc/netdata/hs110.conf` (or `/usr/local/etc/netdata/hs110.conf`)\n\
        file specifying addresses of smartplugs, like in the example below:\n\
        ```\n\
    hosts:
    - 192.168.0.124
    - 192.168.0.156
    - 192.168.0.155\
        \n```\n\n\
        And then restart netdata service.\n\
        After that you should see a `Smartplugs` section appeared in netdata web interface."
    );

    Ok(())
}

fn target_dir() -> PathBuf {
    match option_env!("CARGO_TARGET_DIR") {
        Some(explicit_path) => Path::new(explicit_path).to_path_buf(),
        None => project_root().join("target"),
    }
}

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}
