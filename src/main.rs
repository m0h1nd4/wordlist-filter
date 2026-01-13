//! Wordlist Filter - High-performance wordlist processing for penetration testing
//!
//! Main entry point for the command-line application.

use clap::Parser;
use colored::*;
use std::process;

use wordlist_filter::cli::Args;
use wordlist_filter::processor::{Processor, ProcessorConfig};
use wordlist_filter::progress::{print_banner, print_error, print_header, print_info};

fn main() {
    // Parse command-line arguments
    let args = Args::parse();
    
    // Set up logging
    if args.verbose {
        std::env::set_var("RUST_LOG", "debug");
    } else if !args.quiet {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
    
    // Configure thread pool
    if let Some(threads) = args.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .ok();
    }
    
    // Run the application
    if let Err(e) = run(args) {
        print_error(&format!("{}", e));
        
        // Print chain of errors
        let mut source = e.source();
        while let Some(err) = source {
            print_error(&format!("  Caused by: {}", err));
            source = err.source();
        }
        
        process::exit(1);
    }
}

fn run(args: Args) -> anyhow::Result<()> {
    // Print banner unless quiet mode
    if !args.quiet {
        print_banner();
    }
    
    // Validate arguments
    validate_args(&args)?;
    
    // Create processor configuration
    let config = ProcessorConfig::from_args(&args)?;
    
    // Show configuration
    if !args.quiet && args.verbose {
        print_config(&args, &config);
    }
    
    // Create and run processor
    let processor = Processor::new(config);
    processor.process(&args.input)?;
    
    Ok(())
}

/// Validate command-line arguments
fn validate_args(args: &Args) -> anyhow::Result<()> {
    // Check that input exists
    if !args.input.exists() {
        anyhow::bail!("Input path does not exist: {:?}", args.input);
    }
    
    // Check that we have at least one filter
    if args.length.is_none() && args.pattern.is_none() {
        anyhow::bail!("At least one filter must be specified: --length or --pattern");
    }
    
    // Validate regex pattern if provided
    if let Some(ref pattern) = args.pattern {
        wordlist_filter::filter::validate_pattern(pattern)?;
    }
    
    // Validate length specification
    if let Some(ref length) = args.length {
        args.parse_lengths()?;
    }
    
    Ok(())
}

/// Print configuration summary
fn print_config(args: &Args, config: &ProcessorConfig) {
    print_header("Configuration");
    
    print_info(&format!("Input:        {:?}", args.input));
    print_info(&format!("Output dir:   {:?}", config.output_dir));
    
    if let Some(ref lengths) = config.lengths {
        print_info(&format!("Lengths:      {:?}", lengths));
    }
    
    if let Some(ref pattern) = config.pattern {
        print_info(&format!("Pattern:      {}", pattern));
    }
    
    print_info(&format!("Single file:  {}", config.single_file));
    print_info(&format!("Recursive:    {}", config.recursive));
    print_info(&format!("Dedup:        {}", !config.no_dedup));
    print_info(&format!("Extensions:   {:?}", config.extensions));
    print_info(&format!("Buffer size:  {} MB", config.buffer_size / (1024 * 1024)));
    print_info(&format!("Threads:      {}", args.threads.unwrap_or_else(num_cpus::get)));
}
