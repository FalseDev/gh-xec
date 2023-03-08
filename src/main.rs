#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

use clap::{Args, Parser, Subcommand};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    env,
    io::{Cursor, Write},
    process::{Command as SysCommand, Stdio},
};

macro_rules! fzf {
    ($vector:expr, $field: ident) => {{
        let choice = fzf_select($vector.iter().map(|a| a.$field.to_owned()).collect());
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
    if let Some((_, ext)) = asset.name.rsplit_once(".") {
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
    println!("Extracting {} using tar", filename);
    SysCommand::new("tar")
        .args(["--extract", "--file", filename, "--one-top-level"])
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
}

fn install_deb(filename: &str) {
    println!("Installing {} using dpkg", filename);
    SysCommand::new("sudo")
        .args(["dpkg", "--install", filename])
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
}

fn gh_api<'a, T>(mut args: Vec<String>) -> T
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

fn fzf_select(choices: Vec<String>) -> String {
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

pub type Releases = Vec<Release>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub url: String,
    #[serde(rename = "assets_url")]
    pub assets_url: String,
    #[serde(rename = "upload_url")]
    pub upload_url: String,
    #[serde(rename = "html_url")]
    pub html_url: String,
    pub id: i64,
    pub author: Author,
    #[serde(rename = "node_id")]
    pub node_id: String,
    #[serde(rename = "tag_name")]
    pub tag_name: String,
    #[serde(rename = "target_commitish")]
    pub target_commitish: String,
    pub name: String,
    pub draft: bool,
    pub prerelease: bool,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "published_at")]
    pub published_at: String,
    pub assets: Vec<Asset>,
    #[serde(rename = "tarball_url")]
    pub tarball_url: String,
    #[serde(rename = "zipball_url")]
    pub zipball_url: String,
    pub body: String,
    pub reactions: Option<Reactions>,
    #[serde(rename = "mentions_count")]
    pub mentions_count: Option<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Author {
    pub login: String,
    pub id: i64,
    #[serde(rename = "node_id")]
    pub node_id: String,
    #[serde(rename = "avatar_url")]
    pub avatar_url: String,
    #[serde(rename = "gravatar_id")]
    pub gravatar_id: String,
    pub url: String,
    #[serde(rename = "html_url")]
    pub html_url: String,
    #[serde(rename = "followers_url")]
    pub followers_url: String,
    #[serde(rename = "following_url")]
    pub following_url: String,
    #[serde(rename = "gists_url")]
    pub gists_url: String,
    #[serde(rename = "starred_url")]
    pub starred_url: String,
    #[serde(rename = "subscriptions_url")]
    pub subscriptions_url: String,
    #[serde(rename = "organizations_url")]
    pub organizations_url: String,
    #[serde(rename = "repos_url")]
    pub repos_url: String,
    #[serde(rename = "events_url")]
    pub events_url: String,
    #[serde(rename = "received_events_url")]
    pub received_events_url: String,
    #[serde(rename = "type")]
    pub type_field: String,
    #[serde(rename = "site_admin")]
    pub site_admin: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub url: String,
    pub id: i64,
    #[serde(rename = "node_id")]
    pub node_id: String,
    pub name: String,
    pub label: String,
    pub uploader: Uploader,
    #[serde(rename = "content_type")]
    pub content_type: String,
    pub state: String,
    pub size: i64,
    #[serde(rename = "download_count")]
    pub download_count: i64,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "updated_at")]
    pub updated_at: String,
    #[serde(rename = "browser_download_url")]
    pub browser_download_url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Uploader {
    pub login: String,
    pub id: i64,
    #[serde(rename = "node_id")]
    pub node_id: String,
    #[serde(rename = "avatar_url")]
    pub avatar_url: String,
    #[serde(rename = "gravatar_id")]
    pub gravatar_id: String,
    pub url: String,
    #[serde(rename = "html_url")]
    pub html_url: String,
    #[serde(rename = "followers_url")]
    pub followers_url: String,
    #[serde(rename = "following_url")]
    pub following_url: String,
    #[serde(rename = "gists_url")]
    pub gists_url: String,
    #[serde(rename = "starred_url")]
    pub starred_url: String,
    #[serde(rename = "subscriptions_url")]
    pub subscriptions_url: String,
    #[serde(rename = "organizations_url")]
    pub organizations_url: String,
    #[serde(rename = "repos_url")]
    pub repos_url: String,
    #[serde(rename = "events_url")]
    pub events_url: String,
    #[serde(rename = "received_events_url")]
    pub received_events_url: String,
    #[serde(rename = "type")]
    pub type_field: String,
    #[serde(rename = "site_admin")]
    pub site_admin: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reactions {
    pub url: String,
    #[serde(rename = "total_count")]
    pub total_count: i64,
    #[serde(rename = "+1")]
    pub n1: i64,
    #[serde(rename = "-1")]
    pub n12: i64,
    pub laugh: i64,
    pub hooray: i64,
    pub confused: i64,
    pub heart: i64,
    pub rocket: i64,
    pub eyes: i64,
}
