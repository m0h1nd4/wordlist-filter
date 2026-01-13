//! Progress display module
//!
//! Provides styled progress bars and statistics display for the pentesting aesthetic.

use bytesize::ByteSize;
use colored::*;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Color theme for the tool
pub mod theme {
    use colored::Color;
    
    pub const PRIMARY: Color = Color::Green;
    pub const SECONDARY: Color = Color::BrightGreen;
    pub const ACCENT: Color = Color::Cyan;
    pub const WARNING: Color = Color::Yellow;
    pub const ERROR: Color = Color::Red;
    pub const MUTED: Color = Color::BrightBlack;
}

/// Print the application banner
pub fn print_banner() {
    let banner = r#"
╔══════════════════════════════════════════════════════════════════════════════╗
║                                                                              ║
║   ██╗    ██╗ ██████╗ ██████╗ ██████╗ ██╗     ██╗███████╗████████╗           ║
║   ██║    ██║██╔═══██╗██╔══██╗██╔══██╗██║     ██║██╔════╝╚══██╔══╝           ║
║   ██║ █╗ ██║██║   ██║██████╔╝██║  ██║██║     ██║███████╗   ██║              ║
║   ██║███╗██║██║   ██║██╔══██╗██║  ██║██║     ██║╚════██║   ██║              ║
║   ╚███╔███╔╝╚██████╔╝██║  ██║██████╔╝███████╗██║███████║   ██║              ║
║    ╚══╝╚══╝  ╚═════╝ ╚═╝  ╚═╝╚═════╝ ╚══════╝╚═╝╚══════╝   ╚═╝              ║
║                                                                              ║
║   ███████╗██╗██╗  ████████╗███████╗██████╗                                  ║
║   ██╔════╝██║██║  ╚══██╔══╝██╔════╝██╔══██╗                                 ║
║   █████╗  ██║██║     ██║   █████╗  ██████╔╝                                 ║
║   ██╔══╝  ██║██║     ██║   ██╔══╝  ██╔══██╗                                 ║
║   ██║     ██║███████╗██║   ███████╗██║  ██║                                 ║
║   ╚═╝     ╚═╝╚══════╝╚═╝   ╚══════╝╚═╝  ╚═╝                                 ║
║                                                                              ║
║                    High-Performance Wordlist Processing                       ║
║                         For Penetration Testing                               ║
║                                                              v1.0.0          ║
╚══════════════════════════════════════════════════════════════════════════════╝
"#;
    
    println!("{}", banner.green());
}

/// Print a section header
pub fn print_header(text: &str) {
    println!("\n{} {}", "▶".green(), text.green().bold());
}

/// Print an info message
pub fn print_info(text: &str) {
    println!("  {} {}", "ℹ".cyan(), text);
}

/// Print a success message
pub fn print_success(text: &str) {
    println!("  {} {}", "✔".green(), text.green());
}

/// Print a warning message
pub fn print_warning(text: &str) {
    println!("  {} {}", "⚠".yellow(), text.yellow());
}

/// Print an error message
pub fn print_error(text: &str) {
    eprintln!("  {} {}", "✖".red(), text.red());
}

/// Print a bullet point
pub fn print_bullet(text: &str) {
    println!("  {} {}", "•".green(), text);
}

/// Create a styled progress bar
pub fn create_progress_bar(total: u64, msg: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.green/dim}] {pos}/{len} ({percent}%) {msg}")
            .unwrap()
            .progress_chars("█▓░")
    );
    
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    
    pb
}

/// Create a styled spinner for indeterminate progress
pub fn create_spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} [{elapsed_precise}] {msg}")
            .unwrap()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
    );
    
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    
    pb
}

/// Create a bytes-based progress bar
pub fn create_bytes_progress_bar(total_bytes: u64, msg: &str) -> ProgressBar {
    let pb = ProgressBar::new(total_bytes);
    
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.green/dim}] {bytes}/{total_bytes} ({bytes_per_sec}) {msg}")
            .unwrap()
            .progress_chars("█▓░")
    );
    
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    
    pb
}

/// Processing statistics
#[derive(Debug)]
pub struct ProcessingStats {
    pub total_files: AtomicU64,
    pub processed_files: AtomicU64,
    pub total_bytes: AtomicU64,
    pub processed_bytes: AtomicU64,
    pub total_lines: AtomicU64,
    pub matched_lines: AtomicU64,
    pub duplicate_lines: AtomicU64,
    pub error_lines: AtomicU64,
    pub start_time: Instant,
}

impl ProcessingStats {
    pub fn new() -> Self {
        Self {
            total_files: AtomicU64::new(0),
            processed_files: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
            processed_bytes: AtomicU64::new(0),
            total_lines: AtomicU64::new(0),
            matched_lines: AtomicU64::new(0),
            duplicate_lines: AtomicU64::new(0),
            error_lines: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }
    
    pub fn add_file(&self, size: u64) {
        self.total_files.fetch_add(1, Ordering::Relaxed);
        self.total_bytes.fetch_add(size, Ordering::Relaxed);
    }
    
    pub fn complete_file(&self, size: u64) {
        self.processed_files.fetch_add(1, Ordering::Relaxed);
        self.processed_bytes.fetch_add(size, Ordering::Relaxed);
    }
    
    pub fn add_line(&self) {
        self.total_lines.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn add_lines(&self, count: u64) {
        self.total_lines.fetch_add(count, Ordering::Relaxed);
    }
    
    pub fn add_match(&self) {
        self.matched_lines.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn add_matches(&self, count: u64) {
        self.matched_lines.fetch_add(count, Ordering::Relaxed);
    }
    
    pub fn add_duplicate(&self) {
        self.duplicate_lines.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn add_duplicates(&self, count: u64) {
        self.duplicate_lines.fetch_add(count, Ordering::Relaxed);
    }
    
    pub fn add_error(&self) {
        self.error_lines.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn get_total_files(&self) -> u64 {
        self.total_files.load(Ordering::Relaxed)
    }
    
    pub fn get_processed_files(&self) -> u64 {
        self.processed_files.load(Ordering::Relaxed)
    }
    
    pub fn get_total_bytes(&self) -> u64 {
        self.total_bytes.load(Ordering::Relaxed)
    }
    
    pub fn get_processed_bytes(&self) -> u64 {
        self.processed_bytes.load(Ordering::Relaxed)
    }
    
    pub fn get_total_lines(&self) -> u64 {
        self.total_lines.load(Ordering::Relaxed)
    }
    
    pub fn get_matched_lines(&self) -> u64 {
        self.matched_lines.load(Ordering::Relaxed)
    }
    
    pub fn get_duplicate_lines(&self) -> u64 {
        self.duplicate_lines.load(Ordering::Relaxed)
    }
    
    pub fn get_error_lines(&self) -> u64 {
        self.error_lines.load(Ordering::Relaxed)
    }
    
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
    
    pub fn lines_per_second(&self) -> f64 {
        let elapsed = self.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.get_total_lines() as f64 / elapsed
        } else {
            0.0
        }
    }
    
    pub fn bytes_per_second(&self) -> f64 {
        let elapsed = self.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.get_processed_bytes() as f64 / elapsed
        } else {
            0.0
        }
    }
    
    /// Print final statistics
    pub fn print_summary(&self) {
        let elapsed = self.elapsed();
        let total_lines = self.get_total_lines();
        let matched = self.get_matched_lines();
        let duplicates = self.get_duplicate_lines();
        let unique = matched.saturating_sub(duplicates);
        let errors = self.get_error_lines();
        
        println!();
        println!("{}", "═".repeat(60).green());
        println!("{}", "                    PROCESSING COMPLETE".green().bold());
        println!("{}", "═".repeat(60).green());
        println!();
        
        println!("  {} {}", "Files processed:".green(), 
            format!("{}/{}", self.get_processed_files(), self.get_total_files()));
        println!("  {} {}", "Data processed: ".green(), 
            format!("{} / {}", 
                ByteSize(self.get_processed_bytes()),
                ByteSize(self.get_total_bytes())));
        println!();
        
        println!("  {} {}", "Total lines:    ".green(), 
            format_number(total_lines));
        println!("  {} {}", "Matched lines:  ".green(), 
            format_number(matched));
        println!("  {} {}", "Duplicates:     ".yellow(), 
            format_number(duplicates));
        println!("  {} {}", "Unique output:  ".green().bold(), 
            format_number(unique).green().bold());
        
        if errors > 0 {
            println!("  {} {}", "Errors:         ".red(), 
                format_number(errors).red());
        }
        
        println!();
        println!("  {} {:?}", "Duration:       ".green(), elapsed);
        println!("  {} {:.2} lines/sec", "Throughput:     ".green(), 
            self.lines_per_second());
        println!("  {} {}/sec", "Speed:          ".green(), 
            ByteSize(self.bytes_per_second() as u64));
        println!();
        println!("{}", "═".repeat(60).green());
    }
}

impl Default for ProcessingStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Format a number with thousand separators
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }
    
    result
}

/// Progress manager for multi-file processing
pub struct ProgressManager {
    multi: MultiProgress,
    main_bar: ProgressBar,
    stats: Arc<ProcessingStats>,
    quiet: bool,
}

impl ProgressManager {
    pub fn new(total_bytes: u64, quiet: bool) -> Self {
        let multi = MultiProgress::new();
        let stats = Arc::new(ProcessingStats::new());
        
        let main_bar = if quiet {
            ProgressBar::hidden()
        } else {
            let pb = multi.add(create_bytes_progress_bar(total_bytes, "Processing..."));
            pb
        };
        
        Self {
            multi,
            main_bar,
            stats,
            quiet,
        }
    }
    
    pub fn stats(&self) -> Arc<ProcessingStats> {
        Arc::clone(&self.stats)
    }
    
    pub fn update_bytes(&self, bytes: u64) {
        self.main_bar.inc(bytes);
    }
    
    pub fn set_message(&self, msg: &str) {
        self.main_bar.set_message(msg.to_string());
    }
    
    pub fn add_sub_progress(&self, total: u64, msg: &str) -> ProgressBar {
        if self.quiet {
            ProgressBar::hidden()
        } else {
            let pb = self.multi.add(create_progress_bar(total, msg));
            pb
        }
    }
    
    pub fn finish(&self) {
        self.main_bar.finish_with_message("Complete".green().to_string());
    }
    
    pub fn finish_and_clear(&self) {
        self.main_bar.finish_and_clear();
    }
}

/// Format duration as human-readable string
pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    
    if secs < 60 {
        format!("{:.1}s", duration.as_secs_f64())
    } else if secs < 3600 {
        let mins = secs / 60;
        let secs = secs % 60;
        format!("{}m {}s", mins, secs)
    } else {
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        format!("{}h {}m", hours, mins)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(123), "123");
        assert_eq!(format_number(1234), "1,234");
        assert_eq!(format_number(1234567), "1,234,567");
    }
    
    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(30)), "30.0s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(Duration::from_secs(3661)), "1h 1m");
    }
    
    #[test]
    fn test_stats() {
        let stats = ProcessingStats::new();
        
        stats.add_lines(100);
        stats.add_matches(50);
        stats.add_duplicates(10);
        
        assert_eq!(stats.get_total_lines(), 100);
        assert_eq!(stats.get_matched_lines(), 50);
        assert_eq!(stats.get_duplicate_lines(), 10);
    }
}
