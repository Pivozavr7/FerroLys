fn main() {
    // Библиотека material-components (вендор, MIT) — каталог "material".
    let material = std::path::Path::new(
        &std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo"),
    )
    .join("material/material.slint");
    let config = slint_build::CompilerConfiguration::new().with_library_paths(
        std::collections::HashMap::from([("material".to_string(), material)]),
    );
    slint_build::compile_with_config("ui/app.slint", config).expect("Slint build failed");
}
