//! Command-line interface definition for wordlist-filter
//!
//! Provides argument parsing and validation for the wordlist filtering tool.

use clap::{Parser, ValueEnum};
use std::path::PathBuf;

/// High-performance wordlist filter for penetration testing
///
/// Filter wordlists by length, regex patterns, and remove duplicates.
/// Optimized for processing very large files (100GB+).
#[derive(Parser, Debug, Clone)]
#[command(
    name = "wordlist-filter",
    author = "m0h1nd4",
    version,
    about = "High-performance wordlist filter for penetration testing",
    long_about = r#"
╔══════════════════════════════════════════════════════════════════════════════╗
║                          WORDLIST-FILTER v1.0.0                              ║
║                    High-Performance Wordlist Processing                       ║
║                         For Penetration Testing                               ║
╚══════════════════════════════════════════════════════════════════════════════╝

Filter large wordlists by length, regex patterns, and automatically remove 
duplicates. Optimized for processing files in the 100-400GB range.

EXAMPLES:
    # Filter for exact length 8
    wordlist-filter -i wordlist.txt -l 8

    # Filter for multiple lengths (creates separate files)
    wordlist-filter -i wordlist.txt -l 8,9,10

    # Filter for length range 8-12
    wordlist-filter -i wordlist.txt -l 8-12

    # All lengths in single file
    wordlist-filter -i wordlist.txt -l 8-12 --single-file

    # Process entire directory recursively
    wordlist-filter -i /wordlists/ -l 8 --recursive

    # Filter with regex pattern
    wordlist-filter -i wordlist.txt -p "^[a-z]{4}[0-9]{4}$"

REGEX PATTERN EXAMPLES:
    ^[a-z]+$           - Only lowercase letters
    ^[A-Z]+$           - Only uppercase letters
    ^[a-zA-Z]+$        - Only letters (mixed case)
    ^[0-9]+$           - Only digits
    ^[a-z]{4}[0-9]{4}$ - 4 lowercase + 4 digits (e.g., "pass1234")
    ^[A-Z][a-z]+[0-9]+ - Capital + lowercase + digits (e.g., "Password123")
    .*[!@#$%].*        - Contains special character
    ^(?=.*[a-z])(?=.*[A-Z])(?=.*[0-9]).{8,}$ - Complex password pattern
"#,
    after_help = "For more information, visit: https://github.com/m0h1nd4/wordlist-filter"
)]
pub struct Args {
    /// Input file or directory path
    #[arg(short, long, required = true, value_name = "PATH")]
    pub input: PathBuf,

    /// Output directory (default: current directory)
    #[arg(short, long, value_name = "DIR")]
    pub output: Option<PathBuf>,

    /// Filter by length: single (8), multiple (8,9,10), or range (8-12)
    #[arg(short, long, value_name = "LENGTH")]
    pub length: Option<String>,

    /// Filter by regex pattern
    #[arg(short, long, value_name = "PATTERN")]
    pub pattern: Option<String>,

    /// Combine all results into a single output file
    #[arg(long, default_value_t = false)]
    pub single_file: bool,

    /// Output filename for single-file mode (default: filtered_wordlist.txt)
    #[arg(long, value_name = "NAME", default_value = "filtered_wordlist.txt")]
    pub output_name: String,

    /// Process directories recursively
    #[arg(short, long, default_value_t = false)]
    pub recursive: bool,

    /// Number of threads (default: auto-detect)
    #[arg(short = 't', long, value_name = "NUM")]
    pub threads: Option<usize>,

    /// Deduplication strategy
    #[arg(long, value_enum, default_value_t = DedupStrategy::Memory)]
    pub dedup_strategy: DedupStrategy,

    /// Memory limit for in-memory deduplication (e.g., "8GB", "16GB")
    #[arg(long, value_name = "SIZE", default_value = "8GB")]
    pub memory_limit: String,

    /// Disable deduplication (faster but may contain duplicates)
    #[arg(long, default_value_t = false)]
    pub no_dedup: bool,

    /// Show detailed statistics
    #[arg(long, default_value_t = false)]
    pub stats: bool,

    /// Quiet mode - minimal output
    #[arg(short, long, default_value_t = false)]
    pub quiet: bool,

    /// Verbose mode - detailed logging
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,

    /// Dry run - show what would be done without writing files
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// Buffer size for file operations (default: 64MB)
    #[arg(long, value_name = "SIZE", default_value = "64MB")]
    pub buffer_size: String,

    /// File extensions to process (default: txt)
    #[arg(long, value_name = "EXT", default_value = "txt")]
    pub extensions: String,

    /// Preserve original line order (slower, more memory)
    #[arg(long, default_value_t = false)]
    pub preserve_order: bool,

    /// Sort output alphabetically
    #[arg(long, default_value_t = false)]
    pub sort: bool,
}

/// Deduplication strategy for handling large datasets
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DedupStrategy {
    /// In-memory HashSet (fastest, requires RAM)
    Memory,
    /// Streaming with bloom filter (fast, probabilistic)
    Bloom,
    /// Disk-based deduplication (slowest, unlimited size)
    #[cfg(feature = "disk-dedup")]
    Disk,
}

impl Args {
    /// Parse the length specification into a list of lengths
    pub fn parse_lengths(&self) -> anyhow::Result<Option<Vec<usize>>> {
        let Some(ref length_str) = self.length else {
            return Ok(None);
        };

        let mut lengths = Vec::new();

        for part in length_str.split(',') {
            let part = part.trim();
            
            if part.contains('-') {
                // Range: "8-12"
                let parts: Vec<&str> = part.split('-').collect();
                if parts.len() != 2 {
                    anyhow::bail!("Invalid length range format: '{}'. Use format: START-END (e.g., 8-12)", part);
                }
                
                let start: usize = parts[0].trim().parse()
                    .map_err(|_| anyhow::anyhow!("Invalid start value in range: '{}'", parts[0]))?;
                let end: usize = parts[1].trim().parse()
                    .map_err(|_| anyhow::anyhow!("Invalid end value in range: '{}'", parts[1]))?;
                
                if start > end {
                    anyhow::bail!("Invalid range: start ({}) must be <= end ({})", start, end);
                }
                
                for len in start..=end {
                    if !lengths.contains(&len) {
                        lengths.push(len);
                    }
                }
            } else {
                // Single value: "8"
                let len: usize = part.parse()
                    .map_err(|_| anyhow::anyhow!("Invalid length value: '{}'", part))?;
                if !lengths.contains(&len) {
                    lengths.push(len);
                }
            }
        }

        lengths.sort_unstable();
        Ok(Some(lengths))
    }

    /// Parse buffer size string to bytes
    pub fn parse_buffer_size(&self) -> anyhow::Result<usize> {
        parse_size(&self.buffer_size)
    }

    /// Parse memory limit string to bytes
    pub fn parse_memory_limit(&self) -> anyhow::Result<usize> {
        parse_size(&self.memory_limit)
    }

    /// Get output directory, defaulting to current directory
    pub fn get_output_dir(&self) -> PathBuf {
        self.output.clone().unwrap_or_else(|| PathBuf::from("."))
    }

    /// Parse file extensions to process
    pub fn get_extensions(&self) -> Vec<String> {
        self.extensions
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

/// Parse human-readable size string to bytes
fn parse_size(size_str: &str) -> anyhow::Result<usize> {
    let size_str = size_str.trim().to_uppercase();
    
    let (num_str, multiplier) = if size_str.ends_with("GB") {
        (&size_str[..size_str.len()-2], 1024 * 1024 * 1024)
    } else if size_str.ends_with("MB") {
        (&size_str[..size_str.len()-2], 1024 * 1024)
    } else if size_str.ends_with("KB") {
        (&size_str[..size_str.len()-2], 1024)
    } else if size_str.ends_with("B") {
        (&size_str[..size_str.len()-1], 1)
    } else {
        (size_str.as_str(), 1)
    };

    let num: usize = num_str.trim().parse()
        .map_err(|_| anyhow::anyhow!("Invalid size format: '{}'", size_str))?;
    
    Ok(num * multiplier)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_length() {
        let args = Args {
            input: PathBuf::from("test.txt"),
            output: None,
            length: Some("8".to_string()),
            pattern: None,
            single_file: false,
            output_name: "filtered_wordlist.txt".to_string(),
            recursive: false,
            threads: None,
            dedup_strategy: DedupStrategy::Memory,
            memory_limit: "8GB".to_string(),
            no_dedup: false,
            stats: false,
            quiet: false,
            verbose: false,
            dry_run: false,
            buffer_size: "64MB".to_string(),
            extensions: "txt".to_string(),
            preserve_order: false,
            sort: false,
        };
        
        let lengths = args.parse_lengths().unwrap().unwrap();
        assert_eq!(lengths, vec![8]);
    }

    #[test]
    fn test_parse_multiple_lengths() {
        let args = Args {
            input: PathBuf::from("test.txt"),
            output: None,
            length: Some("8,9,10".to_string()),
            pattern: None,
            single_file: false,
            output_name: "filtered_wordlist.txt".to_string(),
            recursive: false,
            threads: None,
            dedup_strategy: DedupStrategy::Memory,
            memory_limit: "8GB".to_string(),
            no_dedup: false,
            stats: false,
            quiet: false,
            verbose: false,
            dry_run: false,
            buffer_size: "64MB".to_string(),
            extensions: "txt".to_string(),
            preserve_order: false,
            sort: false,
        };
        
        let lengths = args.parse_lengths().unwrap().unwrap();
        assert_eq!(lengths, vec![8, 9, 10]);
    }

    #[test]
    fn test_parse_length_range() {
        let args = Args {
            input: PathBuf::from("test.txt"),
            output: None,
            length: Some("8-12".to_string()),
            pattern: None,
            single_file: false,
            output_name: "filtered_wordlist.txt".to_string(),
            recursive: false,
            threads: None,
            dedup_strategy: DedupStrategy::Memory,
            memory_limit: "8GB".to_string(),
            no_dedup: false,
            stats: false,
            quiet: false,
            verbose: false,
            dry_run: false,
            buffer_size: "64MB".to_string(),
            extensions: "txt".to_string(),
            preserve_order: false,
            sort: false,
        };
        
        let lengths = args.parse_lengths().unwrap().unwrap();
        assert_eq!(lengths, vec![8, 9, 10, 11, 12]);
    }

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("64MB").unwrap(), 64 * 1024 * 1024);
        assert_eq!(parse_size("8GB").unwrap(), 8 * 1024 * 1024 * 1024);
        assert_eq!(parse_size("1024KB").unwrap(), 1024 * 1024);
    }
}
