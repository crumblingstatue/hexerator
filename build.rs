use vergen::{vergen, Config};

fn main() {
    let mut vergen_cfg = Config::default();
    *vergen_cfg.git_mut().skip_if_error_mut() = true;
    if let Err(e) = vergen(vergen_cfg) {
        println!("cargo:warning=Vergen failed with error: {e}");
    }
}
