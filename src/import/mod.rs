mod har;

pub use har::import_har;

use crate::cli::{ImportArgs, ImportFormat};
use std::path::Path;

pub fn run_import(args: &ImportArgs) -> Result<(), String> {
    // Check if output exists
    if args.output.exists() && !args.force {
        return Err(format!(
            "Output file '{}' already exists. Use --force to overwrite.",
            args.output.display()
        ));
    }

    // Auto-detect format from extension if not specified
    let format = args.format.unwrap_or_else(|| detect_format(&args.input));

    let filter_regex = args
        .filter
        .as_ref()
        .map(|f| regex_lite::Regex::new(f).map_err(|e| format!("Invalid filter regex: {}", e)))
        .transpose()?;

    // Import based on format
    let config = match format {
        ImportFormat::Har => import_har(&args.input, filter_regex.as_ref())?,
        ImportFormat::Postman => {
            return Err("Postman import not yet implemented. Coming soon!".to_string());
        }
        ImportFormat::Openapi => {
            return Err("OpenAPI import not yet implemented. Coming soon!".to_string());
        }
    };

    // Write output
    std::fs::write(&args.output, config)
        .map_err(|e| format!("Failed to write output file: {}", e))?;

    eprintln!("Imported to: {}", args.output.display());
    Ok(())
}

fn detect_format(path: &Path) -> ImportFormat {
    match path.extension().and_then(|e| e.to_str()) {
        Some("har") => ImportFormat::Har,
        Some("json") => {
            // Could be HAR or Postman - try to detect from content
            // For now, assume HAR for .json files
            ImportFormat::Har
        }
        Some("yaml") | Some("yml") => ImportFormat::Openapi,
        _ => ImportFormat::Har, // Default to HAR
    }
}
