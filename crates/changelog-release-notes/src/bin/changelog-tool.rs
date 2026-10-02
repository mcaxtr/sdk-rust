//! Shared changelog operations for SDK release scripts.

use changelog_release_notes::{
    ReleaseOptions, check_fragments, core_notes, prepare_release, release_section,
};
use chrono::NaiveDate;
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let command = args
        .next()
        .ok_or("expected check, prepare, notes, or core-notes")?;
    let mut options = BTreeMap::new();
    while let Some(key) = args.next() {
        if !key.starts_with("--") {
            return Err(format!("unexpected argument: {key}"));
        }
        let value = if matches!(key.as_str(), "--require-new" | "--allow-empty") {
            String::new()
        } else {
            args.next()
                .ok_or_else(|| format!("missing value for {key}"))?
        };
        if options.insert(key.clone(), value).is_some() {
            return Err(format!("duplicate option: {key}"));
        }
    }
    let allowed: &[&str] = match command.as_str() {
        "check" => &["--repo", "--fragments", "--base", "--head", "--require-new"],
        "prepare" => &[
            "--repo",
            "--fragments",
            "--changelog",
            "--version",
            "--date",
            "--allow-empty",
            "--breaking-heading",
        ],
        "notes" => &["--repo", "--changelog", "--version"],
        "core-notes" => &["--repo", "--submodule", "--version", "--from", "--to"],
        _ => return Err(format!("unknown command: {command}")),
    };
    for key in options.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("unknown option: {key}"));
        }
    }
    let get = |key: &str| options.get(key).map(String::as_str);
    let required = |key: &str| get(key).ok_or_else(|| format!("missing {key}"));
    let repo = match get("--repo") {
        Some(repo) => PathBuf::from(repo),
        None => env::current_dir().map_err(|e| e.to_string())?,
    };
    let repo = repo.canonicalize().map_err(|e| e.to_string())?;
    let changelog = Path::new(get("--changelog").unwrap_or("CHANGELOG.md"));
    let directory = Path::new(get("--fragments").unwrap_or("changelog"));
    match command.as_str() {
        "check" => check_fragments(
            &repo,
            directory,
            get("--base"),
            get("--head").unwrap_or("HEAD"),
            options.contains_key("--require-new"),
        )
        .map_err(|e| e.to_string())?,
        "prepare" => {
            let date = NaiveDate::parse_from_str(required("--date")?, "%Y-%m-%d")
                .map_err(|e| e.to_string())?;
            let count = prepare_release(
                &repo,
                changelog,
                directory,
                &ReleaseOptions {
                    version: required("--version")?,
                    date,
                    allow_empty: options.contains_key("--allow-empty"),
                    breaking_heading: get("--breaking-heading"),
                },
            )
            .map_err(|e| e.to_string())?;
            println!(
                "Prepared release {}; consumed {count} changelog fragments",
                required("--version")?
            );
        }
        "notes" => {
            let text = fs::read_to_string(repo.join(changelog)).map_err(|e| e.to_string())?;
            print!(
                "{}",
                release_section(&text, required("--version")?).map_err(|e| e.to_string())?
            );
        }
        "core-notes" => {
            let notes = core_notes(
                &repo,
                Path::new(required("--submodule")?),
                required("--version")?,
                get("--from"),
                get("--to").unwrap_or("HEAD"),
            )
            .map_err(|e| e.to_string())?;
            if !notes.is_empty() {
                println!("{notes}");
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}
