use std::process::Command;

fn main() {
    let git_commit = Command::new("git").args(["rev-parse", "--short", "HEAD"]).output();

    let git_commit = match git_commit {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        _ => String::from("unknown"),
    };

    println!("cargo:rustc-env=MMF_GIT_COMMIT={}", git_commit);
}
