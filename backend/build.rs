use std::process::Command;

fn main() {
    // Rerun this script if the Git HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
    
    // Get the short git hash
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok();

    let git_hash = if let Some(out) = output {
        String::from_utf8(out.stdout).unwrap_or_default().trim().to_string()
    } else {
        "unknown".to_string()
    };

    // Export the hash as an environment variable for the compiler
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
}
