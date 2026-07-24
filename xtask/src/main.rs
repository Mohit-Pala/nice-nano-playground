use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const APP_BASE: &str = "0x26000";
const FAMILY: &str = "NRF52840";

// can go up to 500, chip itself is working fine but going above 500 seems to depend upon the quality of solder and i fucking suck at this shit
// this should prob never be used unless running the debugger
const CHIP: &str = "nRF52840_xxAA";
const DEFAULT_SPEED_KHZ: &str = "100";
const BOOTLOADER_HEX: &str =
    "tools/nice_nano_bootloader-0.6.0_s140_6.1.1/nice_nano_bootloader-0.6.0_s140_6.1.1.hex";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // nav 1 dir up
        .expect("missing parent, achievement: how did we get here") 
        .to_path_buf()
}

// run commands
fn run(cmd: &mut Command) -> Result<(), String> {
    println!("+ {cmd:?}");
    let status = cmd.status().map_err(|e| format!("err: {e}"))?;
    if !status.success() {
        return Err(format!("exit status: {status}"));
    }
    Ok(())
}

fn cmd_uf2(release: bool, output: Option<PathBuf>) -> Result<(), String> {

    // paths for dirs and fw files
    let root = repo_root();
    let fw_dir = root.join("crates").join("ropk-fw");
    let scripts_dir = root.join("scripts");
    let uf2conv = scripts_dir.join("microsoft_uf2_scripts").join("uf2conv.py");
    let profile = if release { "release" } else { "debug" };
    let target_dir = fw_dir
        .join("target")
        .join("thumbv7em-none-eabihf")
        .join(profile);
    let bin_path = target_dir.join("ropk-fw.bin");
    let out_uf2 = output.unwrap_or_else(|| fw_dir.join("ropk-fw.uf2"));

    let profile_flag: &[&str] = if release { &["--release"] } else { &[] };

    println!("Build - ({profile}) -----");
    run(Command::new("cargo")
        .arg("build")
        .args(profile_flag)
        .current_dir(&fw_dir))?;

    println!("Objcopy - {} -----", bin_path.display());
    run(Command::new("cargo")
        .arg("objcopy")
        .args(profile_flag)
        .arg("--")
        .args(["-O", "binary"])
        .arg(&bin_path)
        .current_dir(&fw_dir))?;

    println!("uf2conv.py - {} -----", out_uf2.display());
    // this reqs uv, todo: add uv to docker to avoid py deps
    run(Command::new("uv")
        .arg("run")
        .arg("--project")
        .arg(&scripts_dir)
        .arg(&uf2conv)
        .args(["-f", FAMILY])
        .args(["-b", APP_BASE])
        .arg("-c")
        .arg("-o")
        .arg(&out_uf2)
        .arg(&bin_path))?;

    println!("\nDone: {}", out_uf2.display());
    println!("\nStored at: {}", out_uf2.file_name().unwrap().to_string_lossy());
    Ok(())
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let sub = args.next().unwrap_or_default();

    let result = match sub.as_str() {
        "uf2" | "" => {
            let mut release = true;
            let mut output = None;
            let rest: Vec<String> = args.collect();
            let mut i = 0;
            while i < rest.len() {
                match rest[i].as_str() {
                    "--debug" => release = false,
                    "-o" | "--output" => {
                        i += 1;
                        output = rest.get(i).map(PathBuf::from);
                    }
                    other => {
                        eprintln!("unknown arg: {other}");
                        return ExitCode::FAILURE;
                    }
                }
                i += 1;
            }
            cmd_uf2(release, output)
        }
        other => Err(format!(
            "unknown subcommand '{other}'"
        )),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("xtask: {e}");
            ExitCode::FAILURE
        }
    }
}
