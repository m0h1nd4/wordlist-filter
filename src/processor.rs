//! Core processing engine
//!
//! Handles parallel processing of large wordlist files with filtering and deduplication.

use crate::cli::Args;
use crate::dedup::{create_deduplicator, Deduplicator, NoOpDeduplicator, ShardedDeduplicator};
use crate::encoding::MmapLineIterator;
use crate::filter::{FilterConfig, MultiLengthRouter, SingleLengthFilter};
use crate::output::{ensure_output_dir, MultiOutputManager, OutputMode, SingleOutputManager};
use crate::progress::{create_bytes_progress_bar, print_bullet, print_error, print_header, print_info, print_success, print_warning, ProcessingStats};

use bytesize::ByteSize;
use colored::*;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use walkdir::WalkDir;

/// Processor configuration
pub struct ProcessorConfig {
    pub lengths: Option<Vec<usize>>,
    pub pattern: Option<String>,
    pub single_file: bool,
    pub output_dir: PathBuf,
    pub output_name: String,
    pub recursive: bool,
    pub no_dedup: bool,
    pub buffer_size: usize,
    pub extensions: Vec<String>,
    pub dry_run: bool,
    pub quiet: bool,
    pub verbose: bool,
    pub sort_output: bool,
}

impl ProcessorConfig {
    pub fn from_args(args: &Args) -> anyhow::Result<Self> {
        Ok(Self {
            lengths: args.parse_lengths()?,
            pattern: args.pattern.clone(),
            single_file: args.single_file,
            output_dir: args.get_output_dir(),
            output_name: args.output_name.clone(),
            recursive: args.recursive,
            no_dedup: args.no_dedup,
            buffer_size: args.parse_buffer_size()?,
            extensions: args.get_extensions(),
            dry_run: args.dry_run,
            quiet: args.quiet,
            verbose: args.verbose,
            sort_output: args.sort,
        })
    }
}

/// Main processor
pub struct Processor {
    config: ProcessorConfig,
    stats: Arc<ProcessingStats>,
}

impl Processor {
    pub fn new(config: ProcessorConfig) -> Self {
        Self {
            config,
            stats: Arc::new(ProcessingStats::new()),
        }
    }
    
    /// Process input (file or directory)
    pub fn process(&self, input: &Path) -> anyhow::Result<()> {
        if !self.config.quiet {
            print_header("Scanning input...");
        }
        
        // Collect files to process
        let files = self.collect_files(input)?;
        
        if files.is_empty() {
            print_warning("No files found to process!");
            return Ok(());
        }
        
        // Calculate total size
        let total_size: u64 = files.iter()
            .map(|(_, size)| *size)
            .sum();
        
        if !self.config.quiet {
            print_info(&format!("Found {} files ({} total)", 
                files.len(), 
                ByteSize(total_size)));
        }
        
        // Ensure output directory exists
        ensure_output_dir(&self.config.output_dir)?;
        
        if self.config.dry_run {
            self.dry_run_report(&files)?;
            return Ok(());
        }
        
        // Process based on mode
        if self.config.single_file {
            self.process_single_output(&files)?;
        } else if let Some(ref lengths) = self.config.lengths {
            if lengths.len() == 1 {
                self.process_single_length(&files, lengths[0])?;
            } else {
                self.process_multi_length(&files, lengths)?;
            }
        } else {
            // Pattern only, single output
            self.process_single_output(&files)?;
        }
        
        // Print statistics
        if !self.config.quiet {
            self.stats.print_summary();
        }
        
        Ok(())
    }
    
    /// Collect all files to process
    fn collect_files(&self, input: &Path) -> anyhow::Result<Vec<(PathBuf, u64)>> {
        let mut files = Vec::new();
        
        if input.is_file() {
            let size = fs::metadata(input)?.len();
            files.push((input.to_path_buf(), size));
            self.stats.add_file(size);
        } else if input.is_dir() {
            let walker = if self.config.recursive {
                WalkDir::new(input)
            } else {
                WalkDir::new(input).max_depth(1)
            };
            
            for entry in walker.into_iter().filter_map(|e| e.ok()) {
                let path = entry.path();
                
                if path.is_file() {
                    // Check extension
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if self.config.extensions.contains(&ext.to_lowercase()) {
                            let size = fs::metadata(path)?.len();
                            files.push((path.to_path_buf(), size));
                            self.stats.add_file(size);
                        }
                    }
                }
            }
        } else {
            anyhow::bail!("Input path does not exist: {:?}", input);
        }
        
        Ok(files)
    }
    
    /// Process files with single output file
    fn process_single_output(&self, files: &[(PathBuf, u64)]) -> anyhow::Result<()> {
        let output_path = self.config.output_dir.join(&self.config.output_name);
        
        if !self.config.quiet {
            print_header("Processing (single output mode)...");
            print_info(&format!("Output: {:?}", output_path));
        }
        
        let filter = FilterConfig::new(
            self.config.lengths.clone(),
            self.config.pattern.as_deref(),
        )?;
        
        let dedup: Box<dyn Deduplicator> = if self.config.no_dedup {
            Box::new(NoOpDeduplicator::new())
        } else {
            // Estimate unique words based on total size
            let total_size: u64 = files.iter().map(|(_, s)| *s).sum();
            let estimated_words = (total_size / 10) as usize; // ~10 bytes per word avg
            Box::new(ShardedDeduplicator::with_capacity(
                num_cpus::get() * 4,
                estimated_words / (num_cpus::get() * 4),
            ))
        };
        
        let output = SingleOutputManager::new(output_path.clone(), self.config.buffer_size)?;
        
        // Process files
        let total_bytes: u64 = files.iter().map(|(_, s)| *s).sum();
        let pb = if self.config.quiet {
            indicatif::ProgressBar::hidden()
        } else {
            create_bytes_progress_bar(total_bytes, "Processing...")
        };
        
        for (path, size) in files {
            if self.config.verbose {
                pb.set_message(format!("Processing {:?}...", path.file_name().unwrap_or_default()));
            }
            
            self.process_file(&path, &filter, &*dedup, |line| {
                output.write_line(line).ok();
            })?;
            
            pb.inc(*size);
            self.stats.complete_file(*size);
        }
        
        pb.finish_with_message("Complete".green().to_string());
        output.flush()?;
        
        if !self.config.quiet {
            print_success(&format!("Output written to: {:?}", output_path));
            print_info(&format!("Unique words: {}", output.lines_written()));
        }
        
        Ok(())
    }
    
    /// Process files with single length filter
    fn process_single_length(&self, files: &[(PathBuf, u64)], length: usize) -> anyhow::Result<()> {
        let output_name = format!("wordlist_len{}.txt", length);
        let output_path = self.config.output_dir.join(&output_name);
        
        if !self.config.quiet {
            print_header(&format!("Processing (length {} filter)...", length));
            print_info(&format!("Output: {:?}", output_path));
        }
        
        let filter = SingleLengthFilter::new(length, self.config.pattern.as_deref())?;
        
        let dedup: Box<dyn Deduplicator> = if self.config.no_dedup {
            Box::new(NoOpDeduplicator::new())
        } else {
            let total_size: u64 = files.iter().map(|(_, s)| *s).sum();
            let estimated_words = (total_size / 10) as usize;
            Box::new(ShardedDeduplicator::with_capacity(
                num_cpus::get() * 4,
                estimated_words / (num_cpus::get() * 4),
            ))
        };
        
        let output = SingleOutputManager::new(output_path.clone(), self.config.buffer_size)?;
        
        let total_bytes: u64 = files.iter().map(|(_, s)| *s).sum();
        let pb = if self.config.quiet {
            indicatif::ProgressBar::hidden()
        } else {
            create_bytes_progress_bar(total_bytes, "Processing...")
        };
        
        for (path, size) in files {
            if self.config.verbose {
                pb.set_message(format!("Processing {:?}...", path.file_name().unwrap_or_default()));
            }
            
            self.process_file_single_length(&path, &filter, &*dedup, |line| {
                output.write_line(line).ok();
            })?;
            
            pb.inc(*size);
            self.stats.complete_file(*size);
        }
        
        pb.finish_with_message("Complete".green().to_string());
        output.flush()?;
        
        if !self.config.quiet {
            print_success(&format!("Output written to: {:?}", output_path));
            print_info(&format!("Unique words: {}", output.lines_written()));
        }
        
        Ok(())
    }
    
    /// Process files with multiple length filters
    fn process_multi_length(&self, files: &[(PathBuf, u64)], lengths: &[usize]) -> anyhow::Result<()> {
        if !self.config.quiet {
            print_header(&format!("Processing (lengths {:?})...", lengths));
        }
        
        let router = MultiLengthRouter::new(lengths.to_vec(), self.config.pattern.as_deref())?;
        
        // Create deduplicator per length
        let dedups: Vec<Box<dyn Deduplicator>> = if self.config.no_dedup {
            lengths.iter().map(|_| Box::new(NoOpDeduplicator::new()) as Box<dyn Deduplicator>).collect()
        } else {
            let total_size: u64 = files.iter().map(|(_, s)| *s).sum();
            let estimated_per_length = (total_size / 10 / lengths.len() as u64) as usize;
            lengths.iter().map(|_| {
                Box::new(ShardedDeduplicator::with_capacity(
                    num_cpus::get() * 2,
                    estimated_per_length / (num_cpus::get() * 2),
                )) as Box<dyn Deduplicator>
            }).collect()
        };
        
        // Create output manager
        let mut output = MultiOutputManager::new(
            self.config.output_dir.clone(),
            "wordlist",
            self.config.buffer_size,
        );
        output.init_lengths(lengths)?;
        
        let total_bytes: u64 = files.iter().map(|(_, s)| *s).sum();
        let pb = if self.config.quiet {
            indicatif::ProgressBar::hidden()
        } else {
            create_bytes_progress_bar(total_bytes, "Processing...")
        };
        
        for (path, size) in files {
            if self.config.verbose {
                pb.set_message(format!("Processing {:?}...", path.file_name().unwrap_or_default()));
            }
            
            self.process_file_multi_length(&path, &router, &dedups, &output)?;
            
            pb.inc(*size);
            self.stats.complete_file(*size);
        }
        
        pb.finish_with_message("Complete".green().to_string());
        output.flush_all()?;
        
        if !self.config.quiet {
            print_success("Output files created:");
            for (len, path) in output.get_paths() {
                if let Some(writer_stats) = output.get_stats().iter().find(|(l, _, _)| *l == len) {
                    print_bullet(&format!("Length {}: {:?} ({} words)", len, path, writer_stats.1));
                }
            }
        }
        
        Ok(())
    }
    
    /// Process a single file with generic filter
    fn process_file<F>(&self, path: &Path, filter: &FilterConfig, dedup: &dyn Deduplicator, mut writer: F) -> anyhow::Result<()>
    where
        F: FnMut(&str),
    {
        let iter = MmapLineIterator::new(path)?;
        
        for line_result in iter {
            match line_result {
                Ok(line) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    
                    self.stats.add_line();
                    
                    if filter.matches(line) {
                        self.stats.add_match();
                        
                        if dedup.insert(line) {
                            writer(line);
                        } else {
                            self.stats.add_duplicate();
                        }
                    }
                }
                Err(_) => {
                    self.stats.add_error();
                }
            }
        }
        
        Ok(())
    }
    
    /// Process file with single length filter (optimized)
    fn process_file_single_length<F>(&self, path: &Path, filter: &SingleLengthFilter, dedup: &dyn Deduplicator, mut writer: F) -> anyhow::Result<()>
    where
        F: FnMut(&str),
    {
        let iter = MmapLineIterator::new(path)?;
        
        for line_result in iter {
            match line_result {
                Ok(line) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    
                    self.stats.add_line();
                    
                    if filter.matches(line) {
                        self.stats.add_match();
                        
                        if dedup.insert(line) {
                            writer(line);
                        } else {
                            self.stats.add_duplicate();
                        }
                    }
                }
                Err(_) => {
                    self.stats.add_error();
                }
            }
        }
        
        Ok(())
    }
    
    /// Process file with multi-length routing
    fn process_file_multi_length(
        &self,
        path: &Path,
        router: &MultiLengthRouter,
        dedups: &[Box<dyn Deduplicator>],
        output: &MultiOutputManager,
    ) -> anyhow::Result<()> {
        let iter = MmapLineIterator::new(path)?;
        
        for line_result in iter {
            match line_result {
                Ok(line) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    
                    self.stats.add_line();
                    
                    if let Some(idx) = router.route(line) {
                        self.stats.add_match();
                        
                        if dedups[idx].insert(line) {
                            let length = router.lengths()[idx];
                            output.write_line(line, length)?;
                        } else {
                            self.stats.add_duplicate();
                        }
                    }
                }
                Err(_) => {
                    self.stats.add_error();
                }
            }
        }
        
        Ok(())
    }
    
    /// Dry run report
    fn dry_run_report(&self, files: &[(PathBuf, u64)]) -> anyhow::Result<()> {
        print_header("DRY RUN - No files will be written");
        
        println!("\n  {} Files to process:", "▶".green());
        for (path, size) in files {
            print_bullet(&format!("{:?} ({})", path, ByteSize(*size)));
        }
        
        println!("\n  {} Output configuration:", "▶".green());
        print_bullet(&format!("Output directory: {:?}", self.config.output_dir));
        
        if self.config.single_file {
            print_bullet(&format!("Single output file: {}", self.config.output_name));
        } else if let Some(ref lengths) = self.config.lengths {
            for len in lengths {
                print_bullet(&format!("wordlist_len{}.txt", len));
            }
        }
        
        if let Some(ref pattern) = self.config.pattern {
            print_bullet(&format!("Regex pattern: {}", pattern));
        }
        
        print_bullet(&format!("Deduplication: {}", 
            if self.config.no_dedup { "disabled" } else { "enabled" }));
        
        Ok(())
    }
    
    /// Get processing statistics
    pub fn stats(&self) -> Arc<ProcessingStats> {
        Arc::clone(&self.stats)
    }
}
