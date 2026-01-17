use lazy_static::lazy_static;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::io::{self, Read};

lazy_static! {
    // Patterns for sub-token normalization
    static ref HEX_PATTERN: Regex = Regex::new(r"^0x[a-fA-F0-9]+$").unwrap();
    static ref BRACKETED_HEX: Regex = Regex::new(r"^\[0x[a-fA-F0-9]+\]$").unwrap();
    static ref PURE_NUMBER: Regex = Regex::new(r"^\d+$").unwrap();
    static ref TIMESTAMP_PATTERN: Regex = Regex::new(r"^\d{2}:\d{2}:\d{2}$").unwrap();
}

/// Represents a normalized token that may contain variable parts
#[derive(Clone, Debug)]
struct NormalizedToken {
    /// The display form (original or with placeholders)
    pattern: String,
    /// Whether this token should be treated as inherently variable
    is_variable: bool,
}

/// Normalize a single token, detecting patterns that indicate variability
fn normalize_token(token: &str) -> NormalizedToken {
    // Hex addresses like 0x104fc4000
    if HEX_PATTERN.is_match(token) {
        return NormalizedToken {
            pattern: "<hex>".to_string(),
            is_variable: true,
        };
    }

    // Bracketed hex like [0x106111f74]
    if BRACKETED_HEX.is_match(token) {
        return NormalizedToken {
            pattern: "[<hex>]".to_string(),
            is_variable: true,
        };
    }

    // Timestamps like 07:28:03
    if TIMESTAMP_PATTERN.is_match(token) {
        return NormalizedToken {
            pattern: "<time>".to_string(),
            is_variable: true,
        };
    }

    // Pure numbers (5+ digits are likely variable identifiers)
    if PURE_NUMBER.is_match(token) && token.len() >= 5 {
        return NormalizedToken {
            pattern: "<num>".to_string(),
            is_variable: true,
        };
    }

    // Default: keep as-is
    NormalizedToken {
        pattern: token.to_string(),
        is_variable: false,
    }
}

/// Tokenize a line by whitespace
fn tokenize(line: &str) -> Vec<String> {
    line.split_whitespace().map(String::from).collect()
}

/// Tokenize and normalize a line
fn tokenize_normalized(line: &str) -> Vec<NormalizedToken> {
    line.split_whitespace().map(normalize_token).collect()
}

/// Compute Shannon entropy: H = -Σ p(x) log2 p(x)
fn compute_entropy(counts: &HashMap<String, usize>, total: usize) -> f64 {
    if total == 0 {
        return 0.0;
    }
    let total_f = total as f64;
    counts
        .values()
        .map(|count| {
            let p = *count as f64 / total_f;
            if p > 0.0 {
                -p * p.log2()
            } else {
                0.0
            }
        })
        .sum()
}

/// A pattern group with its template, count, and sample values
struct PatternGroup {
    template: String,
    count: usize,
    samples: Vec<Vec<String>>, // samples[var_index] = vec of sample values
    var_types: Vec<String>,    // type hint for each variable (e.g., "hex", "num", "")
}

/// A line with its normalized tokens and original tokens
struct ParsedLine {
    normalized: Vec<NormalizedToken>,
    original: Vec<String>,
}

/// Calculate Jaccard similarity between two templates
fn jaccard_similarity(t1: &[String], t2: &[String]) -> f64 {
    let set1: HashSet<&String> = t1.iter().filter(|s| !s.starts_with('<')).collect();
    let set2: HashSet<&String> = t2.iter().filter(|s| !s.starts_with('<')).collect();

    if set1.is_empty() && set2.is_empty() {
        return 1.0;
    }

    let intersection = set1.intersection(&set2).count();
    let union = set1.union(&set2).count();

    if union == 0 {
        return 0.0;
    }

    intersection as f64 / union as f64
}

/// Process input and return compacted output
fn process(input: &str) -> String {
    // Parse all lines
    let parsed_lines: Vec<ParsedLine> = input
        .lines()
        .map(|line| ParsedLine {
            normalized: tokenize_normalized(line),
            original: tokenize(line),
        })
        .collect();

    if parsed_lines.is_empty() {
        return String::new();
    }

    // Step 1: Group lines by token count (length-based pre-grouping)
    let mut length_groups: HashMap<usize, Vec<&ParsedLine>> = HashMap::new();
    for line in &parsed_lines {
        length_groups
            .entry(line.original.len())
            .or_default()
            .push(line);
    }

    // Step 2: Process each length group separately
    let mut all_groups: Vec<PatternGroup> = Vec::new();

    for (_len, lines) in length_groups {
        if lines.is_empty() {
            continue;
        }

        let max_cols = lines.iter().map(|l| l.original.len()).max().unwrap_or(0);

        // Collect column statistics within this length group
        let mut column_stats: Vec<HashMap<String, usize>> = vec![HashMap::new(); max_cols];
        let mut column_counts: Vec<usize> = vec![0; max_cols];
        let mut inherent_variable: Vec<bool> = vec![false; max_cols];

        for line in &lines {
            for (col, token) in line.normalized.iter().enumerate() {
                // Track if any token in this column is inherently variable
                if token.is_variable {
                    inherent_variable[col] = true;
                }
                // Use the normalized pattern for statistics
                *column_stats[col].entry(token.pattern.clone()).or_insert(0) += 1;
                column_counts[col] += 1;
            }
        }

        // Calculate per-column entropy within this group
        let entropies: Vec<f64> = column_stats
            .iter()
            .zip(column_counts.iter())
            .map(|(stats, count)| compute_entropy(stats, *count))
            .collect();

        let threshold = determine_threshold(&column_stats, &column_counts, &entropies);

        // Identify which columns are variable (entropy-based OR inherently variable)
        let is_variable: Vec<bool> = (0..max_cols)
            .map(|col| {
                inherent_variable.get(col).copied().unwrap_or(false)
                    || entropies.get(col).map(|e| *e > threshold).unwrap_or(false)
            })
            .collect();

        // Determine type hints for variable columns
        let var_type_hints: Vec<String> = (0..max_cols)
            .map(|col| {
                if !is_variable.get(col).copied().unwrap_or(false) {
                    return String::new();
                }
                // Check if all values in this column match a pattern
                let stats = &column_stats[col];
                if stats.len() == 1 {
                    let key = stats.keys().next().unwrap();
                    if key == "<hex>" {
                        return "hex".to_string();
                    }
                    if key == "[<hex>]" {
                        return "hex".to_string();
                    }
                    if key == "<num>" {
                        return "num".to_string();
                    }
                    if key == "<time>" {
                        return "time".to_string();
                    }
                }
                String::new()
            })
            .collect();

        // Group lines by their template pattern
        let mut groups: HashMap<String, PatternGroup> = HashMap::new();

        for line in &lines {
            let mut template_parts = Vec::new();
            let mut var_values: Vec<String> = Vec::new();
            let mut var_types: Vec<String> = Vec::new();
            let mut var_idx = 0;

            for (col, token) in line.original.iter().enumerate() {
                if is_variable.get(col).copied().unwrap_or(false) {
                    template_parts.push(format!("<{}>", var_idx));
                    var_values.push(token.clone());
                    var_types.push(var_type_hints.get(col).cloned().unwrap_or_default());
                    var_idx += 1;
                } else {
                    template_parts.push(token.clone());
                }
            }

            let template = template_parts.join(" ");

            groups
                .entry(template.clone())
                .and_modify(|g| {
                    g.count += 1;
                    // Add sample values (keep up to 3 unique per variable)
                    for (i, val) in var_values.iter().enumerate() {
                        if g.samples.len() > i
                            && g.samples[i].len() < 3
                            && !g.samples[i].contains(val)
                        {
                            g.samples[i].push(val.clone());
                        }
                    }
                })
                .or_insert_with(|| PatternGroup {
                    template,
                    count: 1,
                    samples: var_values.into_iter().map(|v| vec![v]).collect(),
                    var_types,
                });
        }

        all_groups.extend(groups.into_values());
    }

    // Step 3: Merge similar templates (Jaccard similarity > 0.6)
    all_groups = merge_similar_templates(all_groups, 0.6);

    // Sort groups by count (descending)
    all_groups.sort_by(|a, b| b.count.cmp(&a.count));

    // Format output with reduced verbosity
    let mut output = Vec::new();
    for group in all_groups {
        output.push(format!("[{}x] {}", group.count, group.template));

        // Reduce sample verbosity based on count
        let max_samples = if group.count > 100 {
            1
        } else if group.count > 10 {
            2
        } else {
            3
        };

        if !group.samples.is_empty() && group.samples.iter().any(|s| !s.is_empty())
        {
            let samples_str: Vec<String> = group
                .samples
                .iter()
                .enumerate()
                .filter(|(_, s)| !s.is_empty())
                .map(|(i, s)| {
                    let limited: Vec<_> = s.iter().take(max_samples).cloned().collect();
                    format!("<{}>: {}", i, limited.join(", "))
                })
                .collect();
            if !samples_str.is_empty() {
                output.push(format!("     {}", samples_str.join(" | ")));
            }
        }
    }

    output.join("\n")
}

/// Merge templates with high Jaccard similarity
fn merge_similar_templates(mut groups: Vec<PatternGroup>, threshold: f64) -> Vec<PatternGroup> {
    if groups.len() < 2 {
        return groups;
    }

    let mut merged = true;
    while merged {
        merged = false;

        // Parse templates into token vectors for comparison
        let templates: Vec<Vec<String>> = groups
            .iter()
            .map(|g| g.template.split_whitespace().map(String::from).collect())
            .collect();

        // Find pairs to merge
        let mut merge_pair: Option<(usize, usize)> = None;

        'outer: for i in 0..groups.len() {
            for j in (i + 1)..groups.len() {
                // Only merge templates with same token count
                if templates[i].len() != templates[j].len() {
                    continue;
                }

                let similarity = jaccard_similarity(&templates[i], &templates[j]);
                if similarity >= threshold {
                    merge_pair = Some((i, j));
                    break 'outer;
                }
            }
        }

        if let Some((i, j)) = merge_pair {
            // Merge group j into group i
            let group_j = groups.remove(j);
            let group_i = &mut groups[i];

            // Create merged template: positions that differ become variables
            let tokens_i: Vec<String> = group_i
                .template
                .split_whitespace()
                .map(String::from)
                .collect();
            let tokens_j: Vec<String> = group_j
                .template
                .split_whitespace()
                .map(String::from)
                .collect();

            let mut new_template_parts = Vec::new();
            let mut new_samples: Vec<Vec<String>> = Vec::new();
            let mut new_var_types: Vec<String> = Vec::new();

            // Map old variable indices to their samples
            let mut var_idx_i = 0;
            let mut var_idx_j = 0;
            let mut new_var_idx = 0;

            for (ti, tj) in tokens_i.iter().zip(tokens_j.iter()) {
                let is_var_i = ti.starts_with('<') && ti.ends_with('>');
                let is_var_j = tj.starts_with('<') && tj.ends_with('>');

                if ti == tj {
                    // Same token (including matching placeholders)
                    if is_var_i {
                        new_template_parts.push(format!("<{}>", new_var_idx));

                        // Merge samples from both groups
                        let mut merged_samples: Vec<String> = Vec::new();
                        if var_idx_i < group_i.samples.len() {
                            merged_samples.extend(group_i.samples[var_idx_i].clone());
                        }
                        if var_idx_j < group_j.samples.len() {
                            for s in &group_j.samples[var_idx_j] {
                                if !merged_samples.contains(s) && merged_samples.len() < 3 {
                                    merged_samples.push(s.clone());
                                }
                            }
                        }
                        new_samples.push(merged_samples);
                        new_var_types.push(
                            group_i
                                .var_types
                                .get(var_idx_i)
                                .cloned()
                                .unwrap_or_default(),
                        );
                        new_var_idx += 1;
                        var_idx_i += 1;
                        var_idx_j += 1;
                    } else {
                        new_template_parts.push(ti.clone());
                    }
                } else {
                    // Different tokens - create a variable position
                    new_template_parts.push(format!("<{}>", new_var_idx));

                    let mut merged_samples: Vec<String> = Vec::new();

                    if is_var_i {
                        if var_idx_i < group_i.samples.len() {
                            merged_samples.extend(group_i.samples[var_idx_i].clone());
                        }
                        var_idx_i += 1;
                    } else {
                        merged_samples.push(ti.clone());
                    }

                    if is_var_j {
                        if var_idx_j < group_j.samples.len() {
                            for s in &group_j.samples[var_idx_j] {
                                if !merged_samples.contains(s) && merged_samples.len() < 3 {
                                    merged_samples.push(s.clone());
                                }
                            }
                        }
                        var_idx_j += 1;
                    } else if !merged_samples.contains(tj) && merged_samples.len() < 3 {
                        merged_samples.push(tj.clone());
                    }

                    new_samples.push(merged_samples);
                    new_var_types.push(String::new());
                    new_var_idx += 1;
                }
            }

            group_i.template = new_template_parts.join(" ");
            group_i.count += group_j.count;
            group_i.samples = new_samples;
            group_i.var_types = new_var_types;

            merged = true;
        }
    }

    groups
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

/// Determine entropy threshold using adaptive heuristics
fn determine_threshold(
    column_stats: &[HashMap<String, usize>],
    column_counts: &[usize],
    entropies: &[f64],
) -> f64 {
    // Filter out empty columns
    let valid_entropies: Vec<f64> = entropies
        .iter()
        .zip(column_counts.iter())
        .filter(|(_, count)| **count > 0)
        .map(|(e, _)| *e)
        .collect();

    if valid_entropies.is_empty() {
        return 1.0;
    }

    // Heuristic 1: Use median entropy as base
    let mut sorted = valid_entropies.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = sorted[sorted.len() / 2];

    // Heuristic 2: Consider uniqueness ratio
    // If a column has many unique values relative to total, it's likely noise
    let mut high_uniqueness_entropies: Vec<f64> = Vec::new();
    for (col, stats) in column_stats.iter().enumerate() {
        if column_counts[col] == 0 {
            continue;
        }
        let unique_ratio = stats.len() as f64 / column_counts[col] as f64;
        if unique_ratio > 0.5 {
            high_uniqueness_entropies.push(entropies[col]);
        }
    }

    // Use the minimum of high-uniqueness entropies as threshold
    // or fall back to median + small delta
    if let Some(min_high) = high_uniqueness_entropies
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
    {
        // Use slightly below this value to catch similar columns
        (*min_high * 0.9).max(median)
    } else {
        // No clearly noisy columns found, use a conservative threshold
        // Entropy > 2.0 bits typically indicates high variability
        median.max(2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_traces() {
        let input = r#"+                1744 ???  (in Live)  load address 0x104fc4000 + 0x114df74  [0x106111f74]
+                                         1744 ???  (in Live)  load address 0x104fc4000 + 0x115c9c0  [0x1061209c0]
+                                           1744 ???  (in Live)  load address 0x104fc4000 + 0x1e99770  [0x106e5d770]
+                                             1744 ???  (in Live)  load address 0x104fc4000 + 0x1d09b64  [0x106ccdb64]
+                                               1744 ???  (in Live)  load address 0x104fc4000 + 0x121a7e0  [0x1061de7e0]
+                                                 1744 ???  (in Live)  load address 0x104fc4000 + 0x2c21e8  [0x1052861e8]
+                                                   1744 ???  (in Live)  load address 0x104fc4000 + 0x29b458  [0x10525f458]
+                                                     1744 ???  (in Live)  load address 0x104fc4000 + 0x123c3e8  [0x1062003e8]
+                                                       1744 ???  (in Live)  load address 0x104fc4000 + 0x29975c  [0x10525d75c]"#;

        let expected = r#"[9x] + 1744 ??? (in Live) load address <0> + <1> <2>
     <0>: 0x104fc4000 | <1>: 0x114df74, 0x115c9c0, 0x1e99770 | <2>: [0x106111f74], [0x1061209c0], [0x106e5d770]"#;

        assert_eq!(process(input), expected);
    }

    #[test]
    fn test_sshd_auth_logs() {
        let input = r#"Dec 10 07:28:03 LabSZ sshd[24245]: Failed password for root from 112.95.230.3 port 54087 ssh2
Dec 10 07:28:05 LabSZ sshd[24245]: Failed password for root from 112.95.230.3 port 55618 ssh2
Dec 10 07:28:08 LabSZ sshd[24245]: Failed password for root from 112.95.230.3 port 57138 ssh2"#;

        let expected = r#"[3x] Dec 10 <0> LabSZ sshd[24245]: Failed password for root from 112.95.230.3 port <1> ssh2
     <0>: 07:28:03, 07:28:05, 07:28:08 | <1>: 54087, 55618, 57138"#;

        assert_eq!(process(input), expected);
    }

    #[test]
    fn test_mixed_syslog() {
        // With length-based grouping, different log types are processed separately
        // The sshd lines (same token count) are grouped together
        // The su lines (different token count) are in their own group
        let input = r#"Jun 15 02:04:59 combo sshd(pam_unix)[20892]: authentication failure; logname= uid=0 euid=0 tty=NODEVssh ruser= rhost=220-135-151-1.hinet-ip.hinet.net user=root
Jun 15 02:04:59 combo sshd(pam_unix)[20892]: authentication failure; logname= uid=0 euid=0 tty=NODEVssh ruser= rhost=220-135-151-1.hinet-ip.hinet.net user=root
Jun 15 02:04:59 combo sshd(pam_unix)[20892]: authentication failure; logname= uid=0 euid=0 tty=NODEVssh ruser= rhost=220-135-151-1.hinet-ip.hinet.net user=root
Jun 15 04:06:18 combo su(pam_unix)[21416]: session opened for user cyrus
Jun 15 04:06:19 combo su(pam_unix)[21416]: session closed for user cyrus"#;

        // The sshd lines are identical except timestamp is recognized as inherently variable
        // The su lines have variable timestamps and opened/closed
        let expected = r#"[3x] Jun 15 <0> combo sshd(pam_unix)[20892]: authentication failure; logname= uid=0 euid=0 tty=NODEVssh ruser= rhost=220-135-151-1.hinet-ip.hinet.net user=root
     <0>: 02:04:59
[2x] Jun 15 <0> combo su(pam_unix)[21416]: session <1> for user cyrus
     <0>: 04:06:18, 04:06:19 | <1>: opened, closed"#;

        assert_eq!(process(input), expected);
    }
}
