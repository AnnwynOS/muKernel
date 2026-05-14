use std::{
    env,
    path::{Path, PathBuf},
    process::{self, Command, ExitStatus},
};

const TARGET: &str = "x86_64-kernel";
const KERNEL_BIN: &str = "kernel";
const QEMU_RAM: &str = "256M";

const OVMF_PATHS: &[&str] = &[
    "C:/Program Files/qemu/share/edk2-x86_64-code.fd",
    "C:/Program Files/QEMU/share/edk2-x86_64-code.fd",
    "C:/Program Files/qemu/share/ovmf-x86_64-code.fd",
    "C:/Program Files (x86)/qemu/share/edk2-x86_64-code.fd",
    // Linux
    "/usr/share/ovmf/OVMF.fd",
    "/usr/share/OVMF/OVMF_CODE.fd",
    "/usr/share/edk2/ovmf/OVMF_CODE.fd",
    // macOS
    "/opt/homebrew/share/ovmf/OVMF.fd",
];
fn main() {
    let args: Vec<String> = env::args().collect();
    let task = args.get(1).map(String::as_str).unwrap_or("help");

    match task {
        "build" => cmd_build(false),
        "build-release" => cmd_build(true),
        "run" => cmd_run(false, false),
        "run-release" => cmd_run(true, false),
        "run-bios" => cmd_run_bios(false),
        "debug" => cmd_run(false, true),
        "check" => cmd_check(),
        "clippy" => cmd_clippy(),
        "install-deps" => cmd_install_deps(),
        "help" | "--help" | "-h" => print_help(),
        other => {
            eprintln!("Tâche inconnue : '{other}'\n");
            print_help();
            process::exit(1);
        }
    }
}

fn print_help() {
    println!("Usage: cargo xtask <task>\n");
    println!("Tasks:");
    println!("  build           Compile le kernel (debug) et génère les images");
    println!("  build-release   Compile le kernel (release) et génère les images");
    println!("  run             Lance QEMU en mode UEFI (debug)");
    println!("  run-release     Lance QEMU en mode UEFI (release)");
    println!("  run-bios        Lance QEMU en mode BIOS legacy (debug)");
    println!("  debug           Lance QEMU et stub GDB sur :1234 (debug)");
    println!("  check           cargo check rapide (sans linker)");
    println!("  clippy          Linter");
    println!("  install-deps    Installe les composants rustup nécessaires");
}

fn cmd_build(release: bool) {
    let profile = if release { "release" } else { "debug" };
    println!("==> Compilation du kernel ({profile})…");
    build_kernel(release);

    let kernel_bin = kernel_bin_path(release);
    let (uefi_img, bios_img) = image_paths(release);

    println!("==> Assemblage des images disque…");
    assemble_images(&kernel_bin, &uefi_img, &bios_img);

    println!("\nBuild terminé :");
    println!("  UEFI : {}", uefi_img.display());
    println!("  BIOS : {}", bios_img.display());
}

fn cmd_run(release: bool, debug_stub: bool) {
    cmd_build(release);
    let (uefi_img, _) = image_paths(release);
    let ovmf = find_ovmf();

    println!("\n==> Lancement QEMU (UEFI)…");
    if debug_stub {
        println!("    GDB stub actif sur localhost:1234 — QEMU attend la connexion.");
        println!("    Dans un autre terminal :");
        println!("      rust-gdb target/{TARGET}/debug/kernel");
        println!("      (gdb) target remote :1234");
        println!("      (gdb) continue");
    }

    let mut cmd = qemu_base();
    cmd.args(["-drive", &format!("if=pflash,format=raw,readonly=on,file={}", ovmf)])
        .args(["-drive", &format!("format=raw,file={}", uefi_img.display())])
        .args(["-serial", "stdio"]);
    if debug_stub {
        cmd.args(["-s", "-S"]);
    }
    run_cmd(&mut cmd);
}

fn cmd_run_bios(release: bool) {
    cmd_build(release);
    let (_, bios_img) = image_paths(release);

    println!("\n==> Lancement QEMU (BIOS legacy)...");
    let mut cmd = qemu_base();
    cmd.args(["-drive", &format!("format=raw,file={}", bios_img.display())])
        .args(["-serial", "stdio"]);
    run_cmd(&mut cmd);
}

fn cmd_check() {
    println!("==> cargo check...");
    let target = target_json_path();
    let mut cmd = Command::new("cargo");
    cmd.args(["check", "--package", KERNEL_BIN])
        .args(["--target", &target.to_string_lossy()])
        .args(["-Z", "build-std=core,compiler_builtins"])
        .args(["-Z", "build-std-features=compiler-builtins-mem"])
        .args(["-Z", "json-target-spec"])
        .current_dir(workspace_root());
    run_cmd(&mut cmd);
}

fn cmd_clippy() {
    println!("==> cargo clippy...");
    let target = target_json_path();
    let mut cmd = Command::new("cargo");
    cmd.args(["clippy", "--package", KERNEL_BIN])
        .args(["--target", &target.to_string_lossy()])
        .args(["-Z", "build-std=core,compiler_builtins"])
        .args(["-Z", "build-std-features=compiler-builtins-mem"])
        .args(["-Z", "json-target-spec"])
        .args(["--", "-D", "warnings"])
        .current_dir(workspace_root());
    run_cmd(&mut cmd);
}

fn cmd_install_deps() {
    println!("==> Installation des composants rustup...");
    for component in &["rust-src", "llvm-tools-preview"] {
        let mut c = Command::new("rustup");
        c.args(["component", "add", component]);
        run_cmd(&mut c);
    }
    println!("\n==> QEMU et OVMF sont absents :");
    println!("  Arch : sudo pacman -S qemu-full ovmf");
    println!("  Ubuntu : sudo apt install qemu-system-x86 ovmf");
    println!("  Fedora : sudo dnf install qemu edk2-ovmf");
    println!("  macOS : brew install qemu");
    println!("\nPrêt !");
}

fn build_kernel(release: bool) {
    let target = target_json_path();
    let mut cmd = Command::new("cargo");
    cmd.args(["build", "--package", KERNEL_BIN])
        .args(["--target", &target.to_string_lossy()])
        .args(["-Z", "build-std=core,compiler_builtins"])
        .args(["-Z", "build-std-features=compiler-builtins-mem"])
        .args(["-Z", "json-target-spec"])
        .current_dir(workspace_root());
    if release {
        cmd.arg("--release");
    }
    run_cmd(&mut cmd);
}

fn assemble_images(kernel_bin: &Path, uefi_out: &Path, bios_out: &Path) {
    bootloader::UefiBoot::new(kernel_bin)
        .create_disk_image(uefi_out)
        .unwrap_or_else(|e| panic!("Impossible de créer l'image UEFI : {e}"));

    bootloader::BiosBoot::new(kernel_bin)
        .create_disk_image(bios_out)
        .unwrap_or_else(|e| panic!("Impossible de créer l'image BIOS : {e}"));
}

fn qemu_base() -> Command {
    let mut cmd = Command::new("qemu-system-x86_64");
    cmd.args(["-machine", "q35"])
        .args(["-m", QEMU_RAM])
        .args(["-cpu", "qemu64"])
        .args(["-vga", "std"]);
    cmd
}

fn find_ovmf() -> String {
    for path in OVMF_PATHS {
        if Path::new(path).exists() {
            return path.to_string();
        }
    }
    eprintln!("Erreur : OVMF introuvable.");
    eprintln!("");
    eprintln!("Windows : liste les fichiers disponibles avec :");
    eprintln!("  Get-ChildItem \"C:\\Program Files\\qemu\\share\\\" -Filter \"*.fd\"");
    eprintln!("");
    eprintln!("Linux  : sudo pacman -S ovmf  |  sudo apt install ovmf");
    eprintln!("macOS  : brew install qemu");
    eprintln!("");
    eprintln!("Chemins cherchés : {OVMF_PATHS:?}");
    process::exit(1);
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Impossible de trouver la racine du workspace")
        .to_path_buf()
}

fn target_json_path() -> PathBuf {
    workspace_root()
        .join("kernel")
        .join(format!("{TARGET}.json"))
}

fn kernel_bin_path(release: bool) -> PathBuf {
    workspace_root()
        .join("target")
        .join(TARGET)
        .join(if release { "release" } else { "debug" })
        .join(KERNEL_BIN)
}

fn image_paths(release: bool) -> (PathBuf, PathBuf) {
    let profile = if release { "release" } else { "debug" };
    let dir = workspace_root().join("target").join("images").join(profile);
    std::fs::create_dir_all(&dir).ok();
    (dir.join("kernel-uefi.img"), dir.join("kernel-bios.img"))
}

fn run_cmd(cmd: &mut Command) {
    let status: ExitStatus = cmd.status().unwrap_or_else(|e| {
        eprintln!("Erreur lors du lancement de {:?} : {e}", cmd.get_program());
        process::exit(1);
    });
    if !status.success() {
        process::exit(status.code().unwrap_or(1));
    }
}