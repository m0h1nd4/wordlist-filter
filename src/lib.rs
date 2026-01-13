//! # Wordlist Filter
//!
//! High-performance wordlist processing tool for penetration testing.
//!
//! ## Features
//!
//! - **Length filtering**: Filter by exact length, multiple lengths, or ranges
//! - **Regex patterns**: Filter by custom regex patterns
//! - **Deduplication**: Remove duplicate words (case-sensitive)
//! - **Large file support**: Optimized for 100GB+ files using memory-mapped I/O
//! - **Encoding detection**: Automatic detection and transcoding of various encodings
//! - **Parallel processing**: Multi-threaded processing for maximum performance
//!
//! ## Usage
//!
//! ```bash
//! # Filter for exact length 8
//! wordlist-filter -i wordlist.txt -l 8
//!
//! # Filter for multiple lengths (creates separate files)
//! wordlist-filter -i wordlist.txt -l 8,9,10
//!
//! # Filter with regex pattern
//! wordlist-filter -i wordlist.txt -p "^[a-z]{4}[0-9]{4}$"
//! ```
//!
//! ## Example
//!
//! ```rust,no_run
//! use wordlist_filter::processor::{Processor, ProcessorConfig};
//! use std::path::PathBuf;
//!
//! let config = ProcessorConfig {
//!     lengths: Some(vec![8, 10, 12]),
//!     pattern: None,
//!     single_file: false,
//!     output_dir: PathBuf::from("./output"),
//!     output_name: "filtered.txt".to_string(),
//!     recursive: false,
//!     no_dedup: false,
//!     buffer_size: 64 * 1024 * 1024,
//!     extensions: vec!["txt".to_string()],
//!     dry_run: false,
//!     quiet: false,
//!     verbose: false,
//!     sort_output: false,
//! };
//!
//! let processor = Processor::new(config);
//! // processor.process(&PathBuf::from("wordlist.txt")).unwrap();
//! ```

pub mod cli;
pub mod dedup;
pub mod encoding;
pub mod filter;
pub mod output;
pub mod processor;
pub mod progress;

pub use cli::Args;
pub use processor::{Processor, ProcessorConfig};
