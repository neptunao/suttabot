use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use regex::Regex;
use std::cmp::Reverse;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::helpers::NEWS_DIR;

#[derive(Debug, Clone)]
pub struct NewsEntry {
    pub slug: String,
    pub date: NaiveDate,
    pub body: String,
}

pub enum ResolveResult {
    Empty,
    Single(NewsEntry),
    MultipleByDate(Vec<NewsEntry>),
    NotFound,
}

fn parse_news_filename(filename: &str) -> Option<(NaiveDate, String)> {
    let stripped = filename.trim_end_matches(".md");
    // minimum: "YYYY-MM-DD-X" = 12 chars
    if stripped.len() < 12 {
        return None;
    }
    let date_str = &stripped[..10];
    if stripped.as_bytes().get(10) != Some(&b'-') {
        return None;
    }
    let slug = &stripped[11..];
    if slug.is_empty() {
        return None;
    }
    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()?;
    Some((date, slug.to_string()))
}

fn is_valid_slug(slug: &str) -> bool {
    if slug.is_empty() {
        return false;
    }
    let re = Regex::new(r"^[a-z0-9]+(-[a-z0-9]+)*$").unwrap();
    re.is_match(slug)
}

pub fn list_all() -> Result<Vec<NewsEntry>> {
    let dir = Path::new(NEWS_DIR);
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut entries: Vec<NewsEntry> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().map(|ft| ft.is_file()).unwrap_or(false)
                && e.file_name().to_string_lossy().ends_with(".md")
        })
        .filter_map(|e| {
            let filename = e.file_name().to_string_lossy().into_owned();
            let (date, slug) = parse_news_filename(&filename)?;
            if !is_valid_slug(&slug) {
                log::warn!("Skipping news file with invalid slug: {}", filename);
                return None;
            }
            let body = fs::read_to_string(e.path()).ok()?;
            Some(NewsEntry { slug, date, body })
        })
        .collect();

    entries.sort_by_key(|e| Reverse(e.date));
    Ok(entries)
}

pub fn validate_all() -> Result<()> {
    let entries = list_all()?;
    let mut seen: HashSet<String> = HashSet::new();
    for entry in &entries {
        if !seen.insert(entry.slug.clone()) {
            return Err(anyhow!(
                "Duplicate slug '{}' found in news/ directory — each slug must be unique",
                entry.slug
            ));
        }
    }
    Ok(())
}

pub fn latest() -> Result<Option<NewsEntry>> {
    Ok(list_all()?.into_iter().next())
}

pub fn by_slug(slug: &str) -> Result<Option<NewsEntry>> {
    Ok(list_all()?.into_iter().find(|e| e.slug == slug))
}

pub fn by_date(date_str: &str) -> Result<Vec<NewsEntry>> {
    Ok(list_all()?
        .into_iter()
        .filter(|e| e.date.to_string() == date_str)
        .collect())
}

/// Resolve a user-supplied argument to one or more news entries.
/// Priority: full filename → date → slug.
/// Semver (`^v?\d+\.\d+\.\d+$`) and reserved words ("all", "off", "on") are handled by the callers.
pub fn resolve(arg: &str) -> Result<ResolveResult> {
    if arg.is_empty() {
        return Ok(ResolveResult::Empty);
    }

    // 1. Full filename: YYYY-MM-DD-slug or YYYY-MM-DD-slug.md
    let full_re =
        Regex::new(r"^\d{4}-\d{2}-\d{2}-[a-z0-9]+(-[a-z0-9]+)*(\.md)?$").unwrap();
    if full_re.is_match(arg) {
        let normalized = arg.trim_end_matches(".md");
        // slug starts after the 11th character (after "YYYY-MM-DD-")
        if normalized.len() > 11 {
            let slug = &normalized[11..];
            return match by_slug(slug)? {
                Some(entry) => Ok(ResolveResult::Single(entry)),
                None => Ok(ResolveResult::NotFound),
            };
        }
        return Ok(ResolveResult::NotFound);
    }

    // 2. Date: YYYY-MM-DD
    let date_re = Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    if date_re.is_match(arg) {
        let entries = by_date(arg)?;
        return match entries.len() {
            0 => Ok(ResolveResult::NotFound),
            1 => Ok(ResolveResult::Single(entries.into_iter().next().unwrap())),
            _ => Ok(ResolveResult::MultipleByDate(entries)),
        };
    }

    // 3. Slug: kebab-case
    if is_valid_slug(arg) {
        return match by_slug(arg)? {
            Some(entry) => Ok(ResolveResult::Single(entry)),
            None => Ok(ResolveResult::NotFound),
        };
    }

    Ok(ResolveResult::NotFound)
}
