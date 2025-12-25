use std::path::Path;

fn main() {
    let uid = nix::unistd::getuid().as_raw();
    println!("UID: {}", uid);

    let path_str = format!(
        "/sys/fs/cgroup/user.slice/user-{}.slice/user@{}.service",
        uid, uid
    );
    println!("Checking path: {}", path_str);

    let path = Path::new(&path_str);
    if path.exists() {
        println!("Path exists!");
        // Check write permission by trying to open dir? Or just metadata.
        match std::fs::metadata(&path) {
            Ok(md) => {
                println!("Metadata: {:?}", md.permissions());
                println!("Is Dir: {}", md.is_dir());
            }
            Err(e) => println!("Failed to get metadata: {}", e),
        }
    } else {
        println!("Path DOES NOT exist via std::path::Path::exists()");
    }
}
