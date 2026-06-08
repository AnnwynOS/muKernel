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
    "/usr/share/ovmf/OVMF.fd",
    "/usr/share/OVMF/OVMF_CODE.fd",
    "/usr/share/edk2/ovmf/OVMF_CODE.fd",
    "/opt/homebrew/share/ovmf/OVMF.fd",
];

fn main() {
    let args: Vec<String> = env::args().collect();
    let task = args.get(1).map(String::as_str).unwrap_or("help");
    match task {
        "build"         => cmd_build(false),
        "build-release" => cmd_build(true),
        "run"           => cmd_run(false, false),
        "run-init" => cmd_run_with_init(false),
        "run-release"   => cmd_run(true, false),
        "run-bios"      => cmd_run_bios(false),
        "debug"         => cmd_run(false, true),
        "check"         => cmd_check(),
        "clippy"        => cmd_clippy(),
        "install-deps"  => cmd_install_deps(),
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
    println!("  build / build-release / run / run-init / run-release / run-bios / debug / check / clippy / install-deps");
}

fn cmd_build(release: bool) {
    let profile = if release { "release" } else { "debug" };
    println!("==> Compilation ({profile})…");
    build_kernel(release);
    let kernel_bin = kernel_bin_path(release);
    let (uefi_img, bios_img) = image_paths(release);
    println!("==> Assemblage des images…");
    assemble_images(&kernel_bin, &uefi_img, &bios_img);
    println!("  UEFI : {}\n  BIOS : {}", uefi_img.display(), bios_img.display());
}

fn cmd_run(release: bool, debug_stub: bool) {
    cmd_build(release);
    let (uefi_img, _) = image_paths(release);
    let ovmf = find_ovmf();
    println!("\n==> QEMU UEFI…");
    let mut cmd = qemu_base();
    cmd.args(["-drive", &format!("if=pflash,format=raw,readonly=on,file={}", ovmf)])
        .args(["-drive", &format!("format=raw,file={}", uefi_img.display())])
        .args(["-serial", "stdio"]);
    if debug_stub { cmd.args(["-s", "-S"]); }
    run_cmd(&mut cmd);
}

fn cmd_run_with_init(release: bool) {
    let kernel_root = workspace_root();
    let init_abo = kernel_root.join("kernel").join("assets").join("init.abo");
    if !init_abo.exists() {
        eprintln!("Error: kernel/assets/init.abo not found.");
        eprintln!("Generate it with:");
        eprintln!("cd ../init && make install");
        std::process::exit(1);
    }
    println!("==> init.abo found ({} bytes)", std::fs::metadata(&init_abo).unwrap().len());

    let profile = if release { "release" } else { "debug" };
    println!("==> Compilation avec embedded-init ({profile})...");

    let target = target_json_path();
    let mut cmd = Command::new("cargo");
    cmd.args(["build", "--package", KERNEL_BIN])
        .args(["--target", &target.to_string_lossy()])
        .args(["-Z", "build-std=core,compiler_builtins"])
        .args(["-Z", "build-std-features=compiler-builtins-mem"])
        .args(["-Z", "json-target-spec"])
        .args(["--features", "kernel/embedded-init"])
        .current_dir(workspace_root());
    if release { cmd.arg("--release"); }
    run_cmd(&mut cmd);

    let kernel_bin = kernel_bin_path(release);
    let (uefi_img, bios_img) = image_paths(release);
    assemble_images(&kernel_bin, &uefi_img, &bios_img);

    println!("==> QEMU avec init.abo embarqué...");
    let ovmf = find_ovmf();
    let mut qemu = qemu_base();
    qemu.args(["-drive", &format!("if=pflash,format=raw,readonly=on,file={}", ovmf)])
        .args(["-drive", &format!("format=raw,file={}", uefi_img.display())])
        .args(["-serial", "stdio"]);
    run_cmd(&mut qemu);
}


fn cmd_run_bios(release: bool) {
    cmd_build(release);
    let (_, bios_img) = image_paths(release);
    let mut cmd = qemu_base();
    cmd.args(["-drive", &format!("format=raw,file={}", bios_img.display())])
        .args(["-serial", "stdio"]);
    run_cmd(&mut cmd);
}

fn cmd_check() {
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
    for component in &["rust-src", "llvm-tools-preview"] {
        let mut c = Command::new("rustup");
        c.args(["component", "add", component]);
        run_cmd(&mut c);
    }
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
    if release { cmd.arg("--release"); }
    run_cmd(&mut cmd);
}

fn assemble_images(kernel_bin: &Path, uefi_out: &Path, bios_out: &Path) {
    // La config bootloader est embarquée dans le binaire (BootloaderConfig),
    // donc pas besoin de set_physical_memory_offset ici.
    bootloader::UefiBoot::new(kernel_bin)
        .create_disk_image(uefi_out)
        .unwrap_or_else(|e| panic!("UEFI image: {e}"));
    bootloader::BiosBoot::new(kernel_bin)
        .create_disk_image(bios_out)
        .unwrap_or_else(|e| panic!("BIOS image: {e}"));
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
        if Path::new(path).exists() { return path.to_string(); }
    }
    eprintln!("OVMF introuvable. Chemins : {OVMF_PATHS:?}");
    process::exit(1);
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}
fn target_json_path() -> PathBuf {
    workspace_root().join("kernel").join(format!("{TARGET}.json"))
}
fn kernel_bin_path(release: bool) -> PathBuf {
    workspace_root().join("target").join(TARGET)
        .join(if release { "release" } else { "debug" }).join(KERNEL_BIN)
}
fn image_paths(release: bool) -> (PathBuf, PathBuf) {
    let dir = workspace_root().join("target").join("images")
        .join(if release { "release" } else { "debug" });
    std::fs::create_dir_all(&dir).ok();
    (dir.join("kernel-uefi.img"), dir.join("kernel-bios.img"))
}
fn run_cmd(cmd: &mut Command) {
    let status: ExitStatus = cmd.status().unwrap_or_else(|e| {
        eprintln!("Erreur : {e}"); process::exit(1);
    });
    if !status.success() { process::exit(status.code().unwrap_or(1)); }
}