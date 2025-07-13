use chrono::Local;

fn fetch_git_hash() -> Option<String> {
    use std::process::Command;

    let hash = Command::new("git")
        .args(["rev-parse", "--short"])
        .arg("HEAD")
        .output();

    hash.ok()
        .map(|hash_out| String::from_utf8_lossy(&hash_out.stdout).to_string())
}

fn main() {
    if let Some(hash) = fetch_git_hash() {
        println!("cargo:rustc-env=GIT_HASH={hash}");
    } else {
        println!("cargo:rustc-env=GIT_HASH=unavailable");
    }
    let compile_time = Local::now();
    println!("cargo:rustc-env=BUILD_DATE={}", compile_time.date_naive());
    println!(
        "cargo:rustc-env=BUILD_TIME={}",
        compile_time.time().format("%H:%M")
    );
}
