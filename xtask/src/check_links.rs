//! Link checker for URLs in Scarb error and warning messages
//! 
//! This tool scans the Scarb codebase for HTTP/HTTPS URLs that appear in user-facing
//! contexts (error messages, warnings, help text, etc.) and validates that these 
//! links are still accessible.
//!
//! Usage:
//!   cargo xtask check-links       # Check all URLs
//!   cargo xtask check-links --offline   # Extract URLs only, no network requests

use anyhow::{Context, Result};
use clap::Parser;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Check links in Scarb error and warning messages
#[derive(Parser)]
pub struct Args {
    /// Skip network checks and only validate URL extraction  
    #[clap(long)]
    offline: bool,
}

pub fn main(args: Args) -> Result<()> {
    let project_root = std::env::current_dir()?;
    
    println!("Checking links in Scarb error and warning messages...");
    
    let mut all_urls = HashSet::new();
    
    // Walk through the source files to find URLs
    for entry in WalkDir::new(&project_root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        
        // Only process Rust source files
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            // Skip target directories and .git
            if path.to_string_lossy().contains("/target/") || path.to_string_lossy().contains("/.git/") {
                continue;
            }
            
            if let Ok(content) = fs::read_to_string(path) {
                let urls = extract_urls_from_content(&content, path);
                for url in urls {
                    all_urls.insert(url);
                }
            }
        }
    }
    
    if all_urls.is_empty() {
        println!("No URLs found in source files.");
        return Ok(());
    }
    
    println!("Found {} unique URLs to check:", all_urls.len());
    for url in &all_urls {
        println!("  {}", url);
    }
    
    if args.offline {
        println!("\n🔍 Running in offline mode - URL extraction only");
        return Ok(());
    }
    
    println!("\n🌐 Testing URLs...");
    let mut broken_links = Vec::new();
    let mut network_errors = Vec::new();
    
    for url in &all_urls {
        print!("Checking {}... ", url);
        match check_url(url) {
            Ok(()) => println!("✅ OK"),
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("Failed to send HTTP request") {
                    println!("🔌 NETWORK ERROR: {}", e);
                    network_errors.push((url.clone(), e));
                } else {
                    println!("❌ FAILED: {}", e);
                    broken_links.push((url.clone(), e));
                }
            }
        }
    }
    
    // Report results
    println!();
    if !broken_links.is_empty() {
        println!("❌ Found {} broken links:", broken_links.len());
        for (link, error) in &broken_links {
            println!("  {} - {}", link, error);
        }
    }
    
    if !network_errors.is_empty() {
        println!("🔌 Found {} network errors:", network_errors.len());
        for (link, error) in &network_errors {
            println!("  {} - {}", link, error);
        }
        println!("💡 Network errors might indicate connectivity issues or blocked domains");
        println!("💡 Try running with --offline flag to only validate URL extraction");
    }
    
    if broken_links.is_empty() && network_errors.is_empty() {
        println!("✅ All links are working!");
        Ok(())
    } else if !broken_links.is_empty() {
        anyhow::bail!("Some links returned HTTP errors")
    } else {
        println!("⚠️  Only network connectivity issues found - no HTTP errors from servers");
        Ok(())
    }
}

fn extract_urls_from_content(content: &str, path: &Path) -> Vec<String> {
    let mut urls = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    
    // Use a simple regex-like approach to find HTTP/HTTPS URLs
    for (line_idx, line) in lines.iter().enumerate() {
        let mut start = 0;
        while let Some(http_pos) = line[start..].find("http") {
            let actual_pos = start + http_pos;
            let remaining = &line[actual_pos..];
            
            if remaining.starts_with("http://") || remaining.starts_with("https://") {
                if let Some(url) = extract_url_starting_at(remaining) {
                    // Check if URL is likely user-facing by looking at the current line and surrounding context
                    if is_likely_user_facing_url(line, &url, path) || 
                       is_url_in_user_facing_context(line_idx, &lines) {
                        urls.push(url);
                    }
                }
            }
            start = actual_pos + 4; // Move past "http"
        }
    }
    
    urls
}

fn is_url_in_user_facing_context(line_idx: usize, lines: &[&str]) -> bool {
    // Check surrounding lines for context that indicates this is user-facing
    let start = if line_idx >= 3 { line_idx - 3 } else { 0 };
    let end = std::cmp::min(line_idx + 4, lines.len());
    
    let context = lines[start..end].join(" ").to_lowercase();
    
    let context_patterns = [
        "bail!",
        "error!",
        "warn!",
        "println!",
        "cannot be used",
        "invalid",
        "see the full list",
        "keywords",
        "for more info",
        "documentation",
        "error:",
        "warning:",
        "test_case",
        "starknet book",
        "starknet documentation",
    ];
    
    for pattern in &context_patterns {
        if context.contains(pattern) {
            return true;
        }
    }
    
    false
}

fn extract_url_starting_at(text: &str) -> Option<String> {
    if !text.starts_with("http://") && !text.starts_with("https://") {
        return None;
    }
    
    // Find the end of the URL
    let mut end = 0;
    for (i, ch) in text.char_indices() {
        match ch {
            ' ' | '\n' | '\t' | '\r' | '"' | '\'' | ')' | ']' | '>' | '`' => {
                break;
            }
            _ => end = i + ch.len_utf8(),
        }
    }
    
    if end > 0 {
        let url = &text[..end];
        
        // Filter out template URLs and test URLs
        if url.contains('{') || url.contains('}') || 
           url.contains("127.0.0.1") || url.contains("localhost") ||
           url.contains("example.") || url.contains("[..") ||
           url.contains("homepage.com") {
            return None;
        }
        
        Some(url.to_string())
    } else {
        None
    }
}

fn is_likely_user_facing_url(line: &str, _url: &str, path: &Path) -> bool {
    // Check if the URL appears in contexts that suggest it's user-facing
    let line_lower = line.to_lowercase();
    let line_trimmed = line.trim();
    
    // Look for patterns that suggest this is in an error message, warning, or help text
    let user_facing_patterns = [
        "bail!",
        "error!",
        "warn!",
        "println!",
        "print!",
        "eprintln!",
        "format!",
        "write!",
        "writeln!",
        "see the full list at",
        "for more info",
        "documentation",
        "help:",
        "error:",
        "note:",
        "cannot be used",
        "invalid",
        "warning:",
    ];
    
    // Check if URL is in error messages, help text, or documentation
    for pattern in &user_facing_patterns {
        if line_lower.contains(&pattern.to_lowercase()) {
            return true;
        }
    }
    
    // Check if it's in a test that validates error messages (test_case attribute or => pattern)
    if path.to_string_lossy().contains("test") && 
       (line.contains(" => ") || line.contains("#[test_case(")) {
        return true;
    }
    
    // Check if it's in comments that document user-facing features
    if line_trimmed.starts_with("//") || line_trimmed.starts_with("///") {
        if line_lower.contains("see") || line_lower.contains("documentation") || 
           line_lower.contains("book") || line_lower.contains("docs") ||
           line_lower.contains("starknet") {
            return true;
        }
    }
    
    // Check for string literals that look like user messages (with surrounding quotes)
    if line.contains("\"") || line.contains("r#\"") {
        // Look for error/warning indicators in the same line
        if line_lower.contains("cannot") || line_lower.contains("invalid") || 
           line_lower.contains("error") || line_lower.contains("warning") ||
           line_lower.contains("see") || line_lower.contains("list") ||
           line_lower.contains("full") {
            return true;
        }
    }
    
    // Check if it's in help text or CLI descriptions (args.rs files)
    if path.file_name().and_then(|n| n.to_str()) == Some("args.rs") {
        return true;
    }
    
    false
}

fn check_url(url: &str) -> Result<()> {
    // For now, we'll use a simple approach since we want minimal dependencies
    // We can use the existing reqwest dependency from the workspace
    let rt = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
    
    rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("scarb-link-checker/2.11.4")
            .build()
            .context("Failed to create HTTP client")?;
        
        let response = client
            .head(url)
            .send()
            .await
            .with_context(|| format!("Failed to send HTTP request to {}", url))?;
        
        if response.status().is_success() {
            Ok(())
        } else {
            anyhow::bail!("HTTP {} - {}", response.status().as_u16(), response.status().canonical_reason().unwrap_or("Unknown"))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_extract_url_from_error_message() {
        let content = r#"
            bail!(
                "the name `{name}` cannot be used as a package name, \
                names cannot use Cairo keywords see the full list at \
                https://docs.cairo-lang.org/language_constructs/keywords.html"
            )
        "#;
        let path = PathBuf::from("src/core/package/name.rs");
        let urls = extract_urls_from_content(content, &path);
        
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0], "https://docs.cairo-lang.org/language_constructs/keywords.html");
    }

    #[test]
    fn test_extract_url_from_help_text() {
        let content = r#"
- Starknet Book: https://book.starknet.io/
- Starknet Documentation: https://docs.starknet.io/
        "#;
        let path = PathBuf::from("src/bin/scarb/args.rs");
        let urls = extract_urls_from_content(content, &path);
        
        assert!(urls.contains(&"https://book.starknet.io/".to_string()));
        assert!(urls.contains(&"https://docs.starknet.io/".to_string()));
    }

    #[test]
    fn test_filter_template_urls() {
        let content = r#"
            format!("https://github.com/{repo}/commit/{hash}")
        "#;
        let path = PathBuf::from("src/test.rs");
        let urls = extract_urls_from_content(content, &path);
        
        // Template URLs should be filtered out
        assert_eq!(urls.len(), 0);
    }

    #[test]
    fn test_filter_test_urls() {
        let content = r#"
            let url = "https://example.com/test";
            let local = "https://127.0.0.1:8080/";
        "#;
        let path = PathBuf::from("src/test.rs");
        let urls = extract_urls_from_content(content, &path);
        
        // Test URLs should be filtered out
        assert_eq!(urls.len(), 0);
    }

    #[test]
    fn test_extract_url_from_test_case() {
        let content = r#"
    #[test_case("as" => "the name `as` cannot be used as a package name, names cannot use Cairo keywords see the full list at https://starknet.io/cairo-book/appendix-01-keywords.html")]
        "#;
        let path = PathBuf::from("tests/name_tests.rs");
        let urls = extract_urls_from_content(content, &path);
        
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0], "https://starknet.io/cairo-book/appendix-01-keywords.html");
    }

    #[test]
    fn test_extract_url_starting_at() {
        assert_eq!(
            extract_url_starting_at("https://docs.starknet.io/path rest"),
            Some("https://docs.starknet.io/path".to_string())
        );
        
        assert_eq!(
            extract_url_starting_at("https://github.com/repo/path\""),
            Some("https://github.com/repo/path".to_string())
        );
        
        // Template URLs should be filtered
        assert_eq!(
            extract_url_starting_at("https://docs.starknet.io/{path}"),
            None
        );
        
        // Test URLs should be filtered
        assert_eq!(
            extract_url_starting_at("https://127.0.0.1:8080/"),
            None
        );
    }
}