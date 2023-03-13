fn main() {
    if option_env!("XTASK_PLUGIN_BUILD").is_none() {
        println!("cargo:warning=Use `cargo xtask build` to produce a plugin binary named according to netdata requirements");
    }
}
