use crate::compare::CompareResult;

pub fn print_comparison(result: &CompareResult, serious: bool) {
    let title = if serious {
        "Comparison Results"
    } else {
        "FUSION"
    };

    println!();
    println!("┌{:─^70}┐", "");
    println!(
        "│{:^70}│",
        format!(
            "{}: {} vs {}",
            title,
            truncate(&result.baseline_file, 25),
            truncate(&result.current_file, 25)
        )
    );
    println!("├{:─^70}┤", "");

    // Header
    println!(
        "│ {:20} {:>12} {:>12} {:>10} {:>10} │",
        "Metric", "Baseline", "Current", "Delta", "Status"
    );
    println!("│{:─^70}│", "");

    // Metrics
    for m in &result.metrics {
        let delta_str = if m.delta_pct.abs() < 0.01 {
            "—".to_string()
        } else {
            format!("{:+.1}%", m.delta_pct)
        };

        let status = if m.improved && m.delta_pct.abs() > 1.0 {
            if serious { "↑ BETTER" } else { "↑ POWER" }
        } else if m.regressed && m.delta_pct.abs() > 1.0 {
            if serious { "↓ WORSE" } else { "↓ DRAIN" }
        } else {
            "—"
        };

        let baseline_str = format_value(m.baseline, &m.unit);
        let current_str = format_value(m.current, &m.unit);

        println!(
            "│ {:20} {:>12} {:>12} {:>10} {:>10} │",
            truncate(&m.name, 20),
            baseline_str,
            current_str,
            delta_str,
            status
        );
    }

    // Warnings
    if !result.warnings.is_empty() {
        println!("├{:─^70}┤", "");
        println!("│{:^70}│", "⚠️  WARNINGS");
        println!("│{:70}│", "");
        for warning in &result.warnings {
            println!("│  • {:66}│", truncate(warning, 66));
        }
    }

    // Regressions
    if !result.regressions.is_empty() {
        println!("├{:─^70}┤", "");
        let reg_title = if serious {
            "REGRESSIONS DETECTED"
        } else {
            "⚠️  REGRESSIONS DETECTED"
        };
        println!("│{:^70}│", reg_title);
        println!("│{:70}│", "");
        for reg in &result.regressions {
            println!(
                "│  • {:66}│",
                truncate(
                    &format!(
                        "{}: {:.1}% worse (threshold: {:.1}%)",
                        reg.metric,
                        reg.delta_pct.abs(),
                        reg.threshold_pct
                    ),
                    66
                )
            );
        }
    }

    println!("└{:─^70}┘", "");

    // Summary
    println!();
    if result.has_regressions {
        let msg = if serious {
            "RESULT: Regressions detected. Exiting with code 3."
        } else {
            "RESULT: Power level decreased! Senzu bean required. Exit code 3."
        };
        println!("{}", msg);
    } else {
        let msg = if serious {
            "RESULT: No regressions detected."
        } else {
            "RESULT: Power levels stable. You may proceed."
        };
        println!("{}", msg);
    }
    println!();
}

pub fn print_comparison_json(result: &CompareResult) -> Result<(), String> {
    serde_json::to_writer_pretty(std::io::stdout(), result)
        .map_err(|e| format!("Failed to write JSON: {}", e))?;
    println!();
    Ok(())
}

fn format_value(value: f64, unit: &str) -> String {
    if value >= 1_000_000.0 {
        format!("{:.2}M{}", value / 1_000_000.0, unit)
    } else if value >= 1_000.0 {
        format!("{:.2}K{}", value / 1_000.0, unit)
    } else if value >= 100.0 {
        format!("{:.0}{}", value, unit)
    } else if value >= 1.0 {
        format!("{:.2}{}", value, unit)
    } else if value > 0.0 {
        format!("{:.3}{}", value, unit)
    } else {
        format!("0{}", unit)
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
