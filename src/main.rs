#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

mod releases;
use crate::releases::Releases;
use clap::{Args, Parser, Subcommand};
use serde::de::DeserializeOwned;
use std::{
    env,
    io::{Cursor, Write},
    process::{Command as SysCommand, Stdio},
};

macro_rules! fzf {
    ($vector:expr, $field: ident) => {{
        let choice = fzf_select(
            &$vector
                .iter()
                .map(|a| a.$field.to_owned())
                .collect::<Vec<_>>(),
        );
        $vector
            .into_iter()
            .filter(|a| a.$field == choice)
            .next()
            .unwrap()
    }};
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    Install(InstallArgs),
}

#[derive(Args)]
struct InstallArgs {
    repo: String,
}

fn main() {
    let cli = Cli::parse();
    if cli.command.is_none() {
        println!("No command specified");
        return;
    }
    match &cli.command.unwrap() {
        Command::Install(args) => install(args),
    }
}

fn install(args: &InstallArgs) {
    let releases: Releases = gh_api(vec![format!("repos/{}/releases", args.repo)]);
    let release = fzf!(releases, tag_name);
    let asset = fzf!(release.assets, name);
    SysCommand::new("gh")
        .args([
            "release",
            "download",
            "--repo",
            args.repo.as_str(),
            release.tag_name.as_str(),
            "--pattern",
            asset.name.as_str(),
        ])
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
    if let Some((_, ext)) = asset.name.rsplit_once('.') {
        match ext {
            "deb" => install_deb(&asset.name),
            "tar" | "gz" | "tgz" => extract_tar(&asset.name),
            "zip" => extract_zip(&asset.name),

            _ => println!("Unknown file extension: not handled automatically"),
        }
        std::fs::remove_file(asset.name).unwrap();
    } else {
        println!("No file extension: assuming binary");
        std::fs::rename(asset.name, env::var("HOME").unwrap() + "/.local/bin/").unwrap();
    }
}

fn extract_zip(filename: &str) {
    todo!();
}

fn extract_tar(filename: &str) {
    println!("Extracting {filename} using tar");
    SysCommand::new("tar")
        .args(["--extract", "--file", filename, "--one-top-level"])
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
}

fn install_deb(filename: &str) {
    println!("Installing {filename} using dpkg");
    SysCommand::new("sudo")
        .args(["dpkg", "--install", filename])
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
}

fn gh_api<T>(mut args: Vec<String>) -> T
where
    T: DeserializeOwned,
{
    args.insert(0, "api".into());
    let output = SysCommand::new("gh")
        .args(&args)
        .output()
        .expect("gh command failed");
    serde_json::from_reader(Cursor::new(output.stdout)).unwrap()
}

fn fzf_select(choices: &[String]) -> String {
    let mut fzf_proc = SysCommand::new("fzf")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Unable to spawn fzf");
    fzf_proc
        .stdin
        .take()
        .expect("Unable to pipe to fzf stdin")
        .write_all(choices.join("\n").as_bytes())
        .unwrap();
    let output = fzf_proc.wait_with_output().expect("Failed to read stdout");
    String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_string()
}
