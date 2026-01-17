use std::collections::HashMap;
use std::io::{self, Read};

/// Tokenize a line by whitespace
fn tokenize(line: &str) -> Vec<String> {
    line.split_whitespace().map(String::from).collect()
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
}

/// Process input and return compacted output
fn process(input: &str) -> String {
    let lines: Vec<Vec<String>> = input.lines().map(tokenize).collect();

    if lines.is_empty() {
        return String::new();
    }

    let max_cols = lines.iter().map(|l| l.len()).max().unwrap_or(0);

    let mut column_stats: Vec<HashMap<String, usize>> = vec![HashMap::new(); max_cols];
    let mut column_counts: Vec<usize> = vec![0; max_cols];

    for line in &lines {
        for (col, token) in line.iter().enumerate() {
            *column_stats[col].entry(token.clone()).or_insert(0) += 1;
            column_counts[col] += 1;
        }
    }

    let entropies: Vec<f64> = column_stats
        .iter()
        .zip(column_counts.iter())
        .map(|(stats, count)| compute_entropy(stats, *count))
        .collect();

    let threshold = determine_threshold(&column_stats, &column_counts, &entropies);

    // Identify which columns are variable
    let is_variable: Vec<bool> = (0..max_cols)
        .map(|col| entropies.get(col).map(|e| *e > threshold).unwrap_or(false))
        .collect();

    // Group lines by their template pattern
    let mut groups: HashMap<String, PatternGroup> = HashMap::new();

    for line in &lines {
        let mut template_parts = Vec::new();
        let mut var_values: Vec<String> = Vec::new();
        let mut var_idx = 0;

        for (col, token) in line.iter().enumerate() {
            if is_variable.get(col).copied().unwrap_or(false) {
                template_parts.push(format!("<{}>", var_idx));
                var_values.push(token.clone());
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
            });
    }

    // Sort groups by count (descending)
    let mut groups: Vec<_> = groups.into_values().collect();
    groups.sort_by(|a, b| b.count.cmp(&a.count));

    // Format output
    let mut output = Vec::new();
    for group in groups {
        output.push(format!("[{}x] {}", group.count, group.template));
        if !group.samples.is_empty() && group.samples.iter().any(|s| !s.is_empty()) {
            let samples_str: Vec<String> = group
                .samples
                .iter()
                .enumerate()
                .filter(|(_, s)| !s.is_empty())
                .map(|(i, s)| format!("<{}>: {}", i, s.join(", ")))
                .collect();
            if !samples_str.is_empty() {
                output.push(format!("     {}", samples_str.join(" | ")));
            }
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

        let output = process(input);

        // 9 lines should compress to 2 lines (template + samples)
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);
        // Should show count
        assert!(lines[0].starts_with("[9x]"));
        // Should preserve structure
        assert!(output.contains("1744"));
        assert!(output.contains("Live"));
        assert!(output.contains("load"));
        assert!(output.contains("address"));
        // Should show sample values
        assert!(output.contains("0x114df74"));
    }

    #[test]
    fn test_sshd_auth_logs() {
        // Use same PID to ensure grouping works
        let input = r#"Dec 10 07:28:03 LabSZ sshd[24245]: Failed password for root from 112.95.230.3 port 54087 ssh2
Dec 10 07:28:05 LabSZ sshd[24245]: Failed password for root from 112.95.230.3 port 55618 ssh2
Dec 10 07:28:08 LabSZ sshd[24245]: Failed password for root from 112.95.230.3 port 57138 ssh2"#;

        let output = process(input);

        // 3 identical lines -> 2 lines (template + samples)
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);
        // Should preserve structure keywords
        assert!(output.contains("LabSZ"));
        assert!(output.contains("Failed"));
        assert!(output.contains("password"));
        // Should show count
        assert!(output.contains("[3x]"));
        // Should show sample timestamps
        assert!(output.contains("07:28:"));
    }

    #[test]
    fn test_mixed_syslog() {
        // Identical auth failure lines (same PID pattern)
        let input = r#"Jun 15 02:04:59 combo sshd(pam_unix)[20892]: authentication failure; logname= uid=0 euid=0 tty=NODEVssh ruser= rhost=220-135-151-1.hinet-ip.hinet.net user=root
Jun 15 02:04:59 combo sshd(pam_unix)[20892]: authentication failure; logname= uid=0 euid=0 tty=NODEVssh ruser= rhost=220-135-151-1.hinet-ip.hinet.net user=root
Jun 15 02:04:59 combo sshd(pam_unix)[20892]: authentication failure; logname= uid=0 euid=0 tty=NODEVssh ruser= rhost=220-135-151-1.hinet-ip.hinet.net user=root
Jun 15 04:06:18 combo su(pam_unix)[21416]: session opened for user cyrus
Jun 15 04:06:19 combo su(pam_unix)[21416]: session closed for user cyrus"#;

        let output = process(input);

        // 5 lines -> 4 lines (3 auth=1 group + 2 session=2 groups, each with samples)
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines.len() <= 6); // 3 groups * 2 lines each max
        // Should preserve common structure
        assert!(output.contains("combo"));
        assert!(output.contains("authentication"));
        assert!(output.contains("session"));
        // Should have count for 3 repeated patterns
        assert!(output.contains("[3x]"));
    }
}
