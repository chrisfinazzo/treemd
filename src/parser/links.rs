//! Link detection and parsing from markdown content.
//!
//! This module provides functionality to extract and parse various types of links
//! from markdown documents, including relative file links, anchor links, wikilinks,
//! and external URLs.

use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use std::path::PathBuf;

/// Represents a link found in markdown content.
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    /// Display text of the link
    pub text: String,
    /// The target this link points to
    pub target: LinkTarget,
    /// Byte offset in the source content where the link starts
    pub offset: usize,
}

/// The different types of link targets supported.
#[derive(Debug, Clone, PartialEq)]
pub enum LinkTarget {
    /// Internal anchor link to a heading in the current document (e.g., `#installation`)
    Anchor(String),

    /// Relative file path, optionally with an anchor (e.g., `./docs/api.md#usage`)
    RelativeFile {
        path: PathBuf,
        anchor: Option<String>,
    },

    /// Wikilink format used in Obsidian and other PKM tools (e.g., `[[filename]]`)
    WikiLink {
        target: String,
        alias: Option<String>,
    },

    /// External URL (e.g., `https://example.com`)
    External(String),
}

impl LinkTarget {
    /// Get a string representation of the link target for display/search
    pub fn as_str(&self) -> String {
        match self {
            LinkTarget::Anchor(a) => format!("#{}", a),
            LinkTarget::RelativeFile { path, anchor } => {
                if let Some(a) = anchor {
                    format!("{}#{}", path.display(), a)
                } else {
                    path.display().to_string()
                }
            }
            LinkTarget::WikiLink { target, alias } => {
                if let Some(a) = alias {
                    format!("[[{}|{}]]", target, a)
                } else {
                    format!("[[{}]]", target)
                }
            }
            LinkTarget::External(url) => url.clone(),
        }
    }
}

impl Link {
    /// Create a new link.
    pub fn new(text: String, target: LinkTarget, offset: usize) -> Self {
        Self {
            text,
            target,
            offset,
        }
    }
}

/// Extract all links from markdown content.
///
/// This function parses the markdown and identifies all link types:
/// - Standard markdown links: `[text](url)`
/// - Wikilinks: `[[target]]` or `[[target|alias]]`
/// - Autolinks: `<https://example.com>`
///
/// # Arguments
///
/// * `content` - The markdown content to parse
///
/// # Returns
///
/// A vector of `Link` structs representing all links found in the content.
pub fn extract_links(content: &str) -> Vec<Link> {
    let mut links = Vec::new();

    // First pass: extract standard markdown links
    let parser = Parser::new(content).into_offset_iter();
    let mut in_link = false;
    let mut link_text = String::new();
    let mut link_url = String::new();
    let mut link_offset = 0;

    for (event, range) in parser {
        match event {
            Event::Start(Tag::Link { dest_url, .. }) => {
                in_link = true;
                link_url = dest_url.to_string();
                link_offset = range.start;
            }
            Event::Text(text) if in_link => {
                link_text.push_str(&text);
            }
            Event::End(TagEnd::Link) => {
                if in_link {
                    let target = parse_link_target(&link_url);
                    links.push(Link::new(link_text.clone(), target, link_offset));
                    link_text.clear();
                    link_url.clear();
                    in_link = false;
                }
            }
            _ => {}
        }
    }

    // Second pass: extract wikilinks (not parsed by pulldown-cmark)
    extract_wikilinks(content, &mut links);

    links
}

/// Parse a link URL into a LinkTarget.
fn parse_link_target(url: &str) -> LinkTarget {
    if let Some(anchor) = url.strip_prefix('#') {
        // Anchor link within current document
        LinkTarget::Anchor(anchor.to_string())
    } else if url.starts_with("http://") || url.starts_with("https://") {
        // External URL
        LinkTarget::External(url.to_string())
    } else {
        // Relative file path, possibly with anchor
        if let Some((path, anchor)) = url.split_once('#') {
            LinkTarget::RelativeFile {
                path: PathBuf::from(path),
                anchor: Some(anchor.to_string()),
            }
        } else {
            LinkTarget::RelativeFile {
                path: PathBuf::from(url),
                anchor: None,
            }
        }
    }
}

/// Extract wikilinks from content.
///
/// Wikilinks have the format:
/// - `[[target]]` - simple wikilink
/// - `[[target|alias]]` - wikilink with custom display text
fn extract_wikilinks(content: &str, links: &mut Vec<Link>) {
    let mut chars = content.char_indices().peekable();

    while let Some((i, c)) = chars.next() {
        if c == '[' {
            // Check if this is the start of a wikilink [[
            if let Some(&(_, next_c)) = chars.peek() {
                if next_c == '[' {
                    chars.next(); // consume second '['

                    // Find the closing ]]
                    let mut wikilink_content = String::new();
                    let mut found_closing = false;
                    let offset = i;

                    while let Some((_, c)) = chars.next() {
                        if c == ']' {
                            if let Some(&(_, next_c)) = chars.peek() {
                                if next_c == ']' {
                                    chars.next(); // consume second ']'
                                    found_closing = true;
                                    break;
                                }
                            }
                        }
                        wikilink_content.push(c);
                    }

                    if found_closing && !wikilink_content.is_empty() {
                        // Parse the wikilink content
                        let (target, alias, display_text) =
                            if let Some((target, alias)) = wikilink_content.split_once('|') {
                                (
                                    target.trim().to_string(),
                                    Some(alias.trim().to_string()),
                                    alias.trim().to_string(),
                                )
                            } else {
                                (
                                    wikilink_content.trim().to_string(),
                                    None,
                                    wikilink_content.trim().to_string(),
                                )
                            };

                        links.push(Link::new(
                            display_text,
                            LinkTarget::WikiLink { target, alias },
                            offset,
                        ));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_anchor_link() {
        let md = "See [Installation](#installation) for details.";
        let links = extract_links(md);

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "Installation");
        assert_eq!(
            links[0].target,
            LinkTarget::Anchor("installation".to_string())
        );
    }

    #[test]
    fn test_extract_relative_file_link() {
        let md = "Check [API docs](./docs/api.md) for more.";
        let links = extract_links(md);

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "API docs");
        match &links[0].target {
            LinkTarget::RelativeFile { path, anchor } => {
                assert_eq!(path, &PathBuf::from("./docs/api.md"));
                assert_eq!(anchor, &None);
            }
            _ => panic!("Expected RelativeFile link"),
        }
    }

    #[test]
    fn test_extract_relative_file_link_with_anchor() {
        let md = "See [usage guide](../guide.md#usage) here.";
        let links = extract_links(md);

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "usage guide");
        match &links[0].target {
            LinkTarget::RelativeFile { path, anchor } => {
                assert_eq!(path, &PathBuf::from("../guide.md"));
                assert_eq!(anchor, &Some("usage".to_string()));
            }
            _ => panic!("Expected RelativeFile link"),
        }
    }

    #[test]
    fn test_extract_external_link() {
        let md = "Visit [GitHub](https://github.com) now.";
        let links = extract_links(md);

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "GitHub");
        assert_eq!(
            links[0].target,
            LinkTarget::External("https://github.com".to_string())
        );
    }

    #[test]
    fn test_extract_wikilink_simple() {
        let md = "See [[README]] for info.";
        let links = extract_links(md);

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "README");
        match &links[0].target {
            LinkTarget::WikiLink { target, alias } => {
                assert_eq!(target, "README");
                assert_eq!(alias, &None);
            }
            _ => panic!("Expected WikiLink"),
        }
    }

    #[test]
    fn test_extract_wikilink_with_alias() {
        let md = "Check [[README.md|readme file]] here.";
        let links = extract_links(md);

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "readme file");
        match &links[0].target {
            LinkTarget::WikiLink { target, alias } => {
                assert_eq!(target, "README.md");
                assert_eq!(alias, &Some("readme file".to_string()));
            }
            _ => panic!("Expected WikiLink"),
        }
    }

    #[test]
    fn test_extract_multiple_links() {
        let md = r#"
# Documentation

See [Installation](#installation) first.
Then check [API docs](./api.md) and [[contributing]].
Visit [GitHub](https://github.com/user/repo) for source.
"#;
        let links = extract_links(md);

        assert_eq!(links.len(), 4);

        // Standard markdown links come first (in order)
        assert_eq!(links[0].text, "Installation");
        assert!(matches!(links[0].target, LinkTarget::Anchor(_)));

        assert_eq!(links[1].text, "API docs");
        assert!(matches!(links[1].target, LinkTarget::RelativeFile { .. }));

        assert_eq!(links[2].text, "GitHub");
        assert!(matches!(links[2].target, LinkTarget::External(_)));

        // Wikilinks come after (extracted in second pass)
        assert_eq!(links[3].text, "contributing");
        assert!(matches!(links[3].target, LinkTarget::WikiLink { .. }));
    }

    #[test]
    fn test_empty_content() {
        let md = "";
        let links = extract_links(md);
        assert_eq!(links.len(), 0);
    }

    #[test]
    fn test_no_links() {
        let md = "This is just plain text with no links.";
        let links = extract_links(md);
        assert_eq!(links.len(), 0);
    }

    #[test]
    fn test_malformed_wikilink() {
        let md = "This has [[incomplete wikilink";
        let links = extract_links(md);
        assert_eq!(links.len(), 0); // Should not extract malformed links
    }
}
