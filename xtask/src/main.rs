use std::env;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

mod decode;

const APP_BASE: &str = "0x26000";
const FAMILY: &str = "NRF52840";

// can go up to 500, chip itself is working fine but going above 500 seems to depend upon the quality of solder and i fucking suck at this shit
// this should prob never be used unless running the debugger
// (unused until a probe-rs flash/debug subcommand lands)
#[allow(dead_code)]
const CHIP: &str = "nRF52840_xxAA";
#[allow(dead_code)]
const DEFAULT_SPEED_KHZ: &str = "100";
#[allow(dead_code)]
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

const DECODE_USAGE: &str = "\
usage: cargo xtask decode [--raw] [--grep TEXT] [--file PATH]... [HEX...]

  HEX...        one payload ([rtype][body..]) as hex: 'f1 2e 06 ..', 'f12e06..',
                '0xf1, 0x2e' or a pasted defmt line '... Payload : [f1, 2e, ..]'
  --file PATH   decode every line of a file; a directory means all *.hex in it
  (nothing)     read lines from stdin, e.g. a saved defmt log
  --grep TEXT   only use lines containing TEXT (e.g. --grep Payload on a log)
  --raw         input is the radio RAM dump [LEN][S1][payload..] (C rfrx)

exits non-zero if any line failed to parse or decode";

fn hex_files(path: &Path) -> Result<Vec<PathBuf>, String> {
    if !path.is_dir() {
        return Ok(vec![path.to_path_buf()]);
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "hex"))
        .collect();
    files.sort();
    Ok(files)
}

fn cmd_decode(args: Vec<String>) -> Result<(), String> {
    let mut framing = decode::Framing::Payload;
    let mut grep: Option<String> = None;
    let mut files: Vec<PathBuf> = Vec::new();
    let mut hex_args: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                println!("{DECODE_USAGE}");
                return Ok(());
            }
            "--raw" => framing = decode::Framing::Raw,
            "--grep" => {
                i += 1;
                grep = Some(args.get(i).ok_or("--grep needs TEXT")?.clone());
            }
            "--file" | "-f" => {
                i += 1;
                let p = args.get(i).ok_or("--file needs PATH")?;
                files.extend(hex_files(Path::new(p))?);
            }
            other => hex_args.push(other.to_string()),
        }
        i += 1;
    }

    // (label, text) sources
    let mut sources: Vec<(String, String)> = Vec::new();
    for f in &files {
        let text =
            std::fs::read_to_string(f).map_err(|e| format!("{}: {e}", f.display()))?;
        sources.push((f.display().to_string(), text));
    }
    if !hex_args.is_empty() {
        sources.push(("args".into(), hex_args.join(" ")));
    }
    if sources.is_empty() {
        let mut text = String::new();
        std::io::stdin()
            .read_to_string(&mut text)
            .map_err(|e| format!("stdin: {e}"))?;
        sources.push(("stdin".into(), text));
    }

    let mut total = 0;
    let mut failed = false;
    for (label, text) in sources {
        let text = match &grep {
            Some(g) => text
                .lines()
                .filter(|l| l.contains(g.as_str()))
                .collect::<Vec<_>>()
                .join("\n"),
            None => text,
        };
        if files.len() > 1 || label != "args" && label != "stdin" {
            println!("==== {label}");
        }
        let (out, n, err) = decode::decode_text(&text, framing);
        print!("{out}");
        total += n;
        failed |= err;
    }

    println!("{total} payload(s) decoded{}", if failed { ", with errors" } else { "" });
    if failed {
        Err("one or more payloads failed to parse/decode".into())
    } else {
        Ok(())
    }
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
        "decode" => cmd_decode(args.collect()),
        other => Err(format!(
            "unknown subcommand '{other}' (try: uf2, decode)"
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
