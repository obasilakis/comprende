use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;
use std::io::{self, Read};

lazy_static! {
    // Hex addresses like 0x104fc4000 or 0x1a377d770
    static ref HEX_ADDR: Regex = Regex::new(r"0x[a-fA-F0-9]+").unwrap();
    // Bracketed addresses like [0x106111f74]
    static ref BRACKETED_HEX: Regex = Regex::new(r"\[0x[a-fA-F0-9]+\]").unwrap();
    // UUIDs like <4B0BCBB4-2271-376E-B5C3-CC18D418FC11>
    static ref UUID_PATTERN: Regex = Regex::new(r"<[A-F0-9]{8}-[A-F0-9]{4}-[A-F0-9]{4}-[A-F0-9]{4}-[A-F0-9]{12}>").unwrap();
    // Thread IDs like Thread_4243153
    static ref THREAD_ID: Regex = Regex::new(r"Thread_\d+").unwrap();
    // Timestamps like 07:28:03 or 22:18:29.360
    static ref TIMESTAMP: Regex = Regex::new(r"\b\d{2}:\d{2}:\d{2}(?:\.\d+)?").unwrap();
    // Large numbers (5+ digits) that are likely variable identifiers
    static ref LARGE_NUM: Regex = Regex::new(r"\b\d{5,}\b").unwrap();
    // Indentation pattern: leading whitespace and tree markers
    static ref INDENT_PATTERN: Regex = Regex::new(r"^([\s+!|:]+)").unwrap();
    // Binary image line pattern (macOS sample/crash reports)
    static ref BINARY_IMAGE: Regex = Regex::new(r"^\s*0x[a-fA-F0-9]+\s+-\s+0x[a-fA-F0-9]+\s+").unwrap();
    // System library paths
    static ref SYSTEM_LIB: Regex = Regex::new(r"/System/Library/|/usr/lib/").unwrap();
}

/// Normalize a line by replacing variable parts with placeholders
fn normalize_line(line: &str) -> String {
    let mut result = line.to_string();

    // Replace bracketed hex addresses first (more specific)
    result = BRACKETED_HEX.replace_all(&result, "<addr>").to_string();
    // Replace hex addresses
    result = HEX_ADDR.replace_all(&result, "<hex>").to_string();
    // Replace UUIDs
    result = UUID_PATTERN.replace_all(&result, "<uuid>").to_string();
    // Replace thread IDs
    result = THREAD_ID.replace_all(&result, "Thread_<id>").to_string();
    // Replace timestamps
    result = TIMESTAMP.replace_all(&result, "<time>").to_string();
    // Replace large numbers (but keep small ones like line offsets)
    result = LARGE_NUM.replace_all(&result, "<num>").to_string();

    result
}

/// Normalize indentation - strip it entirely for better grouping
fn normalize_indent(line: &str) -> String {
    INDENT_PATTERN.replace(line, "").to_string()
}

/// Group and deduplicate lines
struct LineGroup {
    normalized: String,
    count: usize,
}

fn process(input: &str) -> String {
    let lines: Vec<&str> = input.lines().collect();

    if lines.is_empty() {
        return String::new();
    }

    // Step 1: Separate binary images from other content
    let mut regular_lines: Vec<&str> = Vec::new();
    let mut system_images: Vec<&str> = Vec::new();
    let mut app_images: Vec<&str> = Vec::new();

    for line in &lines {
        if BINARY_IMAGE.is_match(line) {
            if SYSTEM_LIB.is_match(line) {
                system_images.push(line);
            } else {
                app_images.push(line);
            }
        } else {
            regular_lines.push(line);
        }
    }

    // Step 2: Normalize and group regular lines
    let mut groups: HashMap<String, LineGroup> = HashMap::new();

    for line in &regular_lines {
        // First normalize variable parts (hex, etc.)
        let normalized = normalize_line(line);
        // Then normalize indentation
        let key = normalize_indent(&normalized);

        groups
            .entry(key.clone())
            .and_modify(|g| g.count += 1)
            .or_insert_with(|| LineGroup {
                normalized: key,
                count: 1,
            });
    }

    // Step 3: Sort by count (descending), then alphabetically for stability
    let mut sorted_groups: Vec<_> = groups.into_values().collect();
    sorted_groups.sort_by(|a, b| {
        b.count.cmp(&a.count).then_with(|| a.normalized.cmp(&b.normalized))
    });

    // Step 4: Format output
    let mut output = Vec::new();

    for group in sorted_groups {
        if group.count == 1 {
            output.push(group.normalized);
        } else {
            output.push(format!("[{}x] {}", group.count, group.normalized));
        }
    }

    // Step 5: Add binary images summary
    if !system_images.is_empty() || !app_images.is_empty() {
        output.push(String::new());
        output.push("=== Binary Images ===".to_string());

        // Keep app/plugin images (they're relevant for debugging)
        for img in &app_images {
            output.push(normalize_line(img));
        }

        // Summarize system images
        if !system_images.is_empty() {
            output.push(format!("[{} system libraries omitted]", system_images.len()));
        }
    }

    output.join("\n")
}

fn main() -> io::Result<()> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let output = process(&input);
    if !output.is_empty() {
        println!("{}", output);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_normalization() {
        let input = "+   1744 ???  (in Live)  load address 0x104fc4000 + 0x115bc98  [0x10611fc98]";
        let normalized = normalize_line(input);
        assert_eq!(normalized, "+   1744 ???  (in Live)  load address <hex> + <hex>  <addr>");
    }

    #[test]
    fn test_indent_normalization() {
        let input = "+   1744 ???  (in Live)  load address <hex> + <hex>  <addr>";
        let result = normalize_indent(input);
        assert_eq!(result, "1744 ???  (in Live)  load address <hex> + <hex>  <addr>");
    }

    #[test]
    fn test_thread_id_normalization() {
        let input = "1744 Thread_4243153   DispatchQueue_1: com.apple.main-thread";
        let normalized = normalize_line(input);
        assert_eq!(normalized, "1744 Thread_<id>   DispatchQueue_1: com.apple.main-thread");
    }

    #[test]
    fn test_uuid_normalization() {
        let input = "<4B0BCBB4-2271-376E-B5C3-CC18D418FC11> /System/Library/foo";
        let normalized = normalize_line(input);
        assert_eq!(normalized, "<uuid> /System/Library/foo");
    }

    #[test]
    fn test_stack_trace_dedup() {
        let input = r#"+   1744 ???  (in Live)  load address 0x104fc4000 + 0x114df74  [0x106111f74]
+   1744 ???  (in Live)  load address 0x104fc4000 + 0x115c9c0  [0x1061209c0]
+   1744 ???  (in Live)  load address 0x104fc4000 + 0x1e99770  [0x106e5d770]"#;

        let output = process(input);
        // All three lines should be deduped into one with count 3
        assert!(output.contains("[3x]"));
        assert!(output.contains("(in Live)"));
    }

    #[test]
    fn test_sshd_logs() {
        let input = r#"Dec 10 07:28:03 LabSZ sshd[24245]: Failed password for root from 112.95.230.3 port 54087 ssh2
Dec 10 07:28:05 LabSZ sshd[24245]: Failed password for root from 112.95.230.3 port 55618 ssh2
Dec 10 07:28:08 LabSZ sshd[24245]: Failed password for root from 112.95.230.3 port 57138 ssh2"#;

        let output = process(input);
        // All three lines should be deduped (port numbers normalized as large nums)
        assert!(output.contains("[3x]"));
        assert!(output.contains("Failed password"));
    }
}
