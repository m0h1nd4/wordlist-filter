//! Output management module
//!
//! Handles writing filtered words to output files with buffering for performance.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Default buffer size for file writing (64MB)
const DEFAULT_BUFFER_SIZE: usize = 64 * 1024 * 1024;

/// Output file writer with buffering
pub struct OutputWriter {
    writer: BufWriter<File>,
    path: PathBuf,
    lines_written: u64,
    bytes_written: u64,
}

impl OutputWriter {
    /// Create a new output writer
    pub fn new(path: PathBuf, buffer_size: usize) -> anyhow::Result<Self> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;
        
        let writer = BufWriter::with_capacity(buffer_size, file);
        
        Ok(Self {
            writer,
            path,
            lines_written: 0,
            bytes_written: 0,
        })
    }
    
    /// Write a line to the output
    pub fn write_line(&mut self, line: &str) -> anyhow::Result<()> {
        writeln!(self.writer, "{}", line)?;
        self.lines_written += 1;
        self.bytes_written += line.len() as u64 + 1; // +1 for newline
        Ok(())
    }
    
    /// Write a line without newline (for batch operations)
    pub fn write(&mut self, data: &str) -> anyhow::Result<()> {
        write!(self.writer, "{}", data)?;
        self.bytes_written += data.len() as u64;
        Ok(())
    }
    
    /// Flush the buffer to disk
    pub fn flush(&mut self) -> anyhow::Result<()> {
        self.writer.flush()?;
        Ok(())
    }
    
    /// Get the output path
    pub fn path(&self) -> &Path {
        &self.path
    }
    
    /// Get number of lines written
    pub fn lines_written(&self) -> u64 {
        self.lines_written
    }
    
    /// Get bytes written
    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }
}

impl Drop for OutputWriter {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

/// Thread-safe output writer
pub struct SyncOutputWriter {
    inner: Mutex<OutputWriter>,
}

impl SyncOutputWriter {
    pub fn new(path: PathBuf, buffer_size: usize) -> anyhow::Result<Self> {
        let writer = OutputWriter::new(path, buffer_size)?;
        Ok(Self {
            inner: Mutex::new(writer),
        })
    }
    
    pub fn write_line(&self, line: &str) -> anyhow::Result<()> {
        let mut writer = self.inner.lock().unwrap();
        writer.write_line(line)
    }
    
    pub fn flush(&self) -> anyhow::Result<()> {
        let mut writer = self.inner.lock().unwrap();
        writer.flush()
    }
    
    pub fn lines_written(&self) -> u64 {
        let writer = self.inner.lock().unwrap();
        writer.lines_written()
    }
    
    pub fn bytes_written(&self) -> u64 {
        let writer = self.inner.lock().unwrap();
        writer.bytes_written()
    }
    
    pub fn path(&self) -> PathBuf {
        let writer = self.inner.lock().unwrap();
        writer.path().to_path_buf()
    }
}

/// Manager for multiple output files (one per length)
pub struct MultiOutputManager {
    writers: HashMap<usize, SyncOutputWriter>,
    output_dir: PathBuf,
    prefix: String,
    buffer_size: usize,
}

impl MultiOutputManager {
    /// Create a new multi-output manager
    pub fn new(output_dir: PathBuf, prefix: &str, buffer_size: usize) -> Self {
        Self {
            writers: HashMap::new(),
            output_dir,
            prefix: prefix.to_string(),
            buffer_size,
        }
    }
    
    /// Initialize writers for specific lengths
    pub fn init_lengths(&mut self, lengths: &[usize]) -> anyhow::Result<()> {
        for &length in lengths {
            let path = self.output_dir.join(format!("{}_len{}.txt", self.prefix, length));
            let writer = SyncOutputWriter::new(path, self.buffer_size)?;
            self.writers.insert(length, writer);
        }
        Ok(())
    }
    
    /// Write a line to the appropriate length file
    pub fn write_line(&self, line: &str, length: usize) -> anyhow::Result<()> {
        if let Some(writer) = self.writers.get(&length) {
            writer.write_line(line)?;
        }
        Ok(())
    }
    
    /// Get or create a writer for a specific length
    pub fn get_or_create(&mut self, length: usize) -> anyhow::Result<&SyncOutputWriter> {
        if !self.writers.contains_key(&length) {
            let path = self.output_dir.join(format!("{}_len{}.txt", self.prefix, length));
            let writer = SyncOutputWriter::new(path, self.buffer_size)?;
            self.writers.insert(length, writer);
        }
        Ok(self.writers.get(&length).unwrap())
    }
    
    /// Flush all writers
    pub fn flush_all(&self) -> anyhow::Result<()> {
        for writer in self.writers.values() {
            writer.flush()?;
        }
        Ok(())
    }
    
    /// Get statistics for all outputs
    pub fn get_stats(&self) -> Vec<(usize, u64, u64)> {
        let mut stats: Vec<_> = self.writers.iter()
            .map(|(&len, w)| (len, w.lines_written(), w.bytes_written()))
            .collect();
        stats.sort_by_key(|(len, _, _)| *len);
        stats
    }
    
    /// Get output paths
    pub fn get_paths(&self) -> Vec<(usize, PathBuf)> {
        let mut paths: Vec<_> = self.writers.iter()
            .map(|(&len, w)| (len, w.path()))
            .collect();
        paths.sort_by_key(|(len, _)| *len);
        paths
    }
}

/// Single output manager for combined output
pub struct SingleOutputManager {
    writer: SyncOutputWriter,
}

impl SingleOutputManager {
    pub fn new(path: PathBuf, buffer_size: usize) -> anyhow::Result<Self> {
        let writer = SyncOutputWriter::new(path, buffer_size)?;
        Ok(Self { writer })
    }
    
    pub fn write_line(&self, line: &str) -> anyhow::Result<()> {
        self.writer.write_line(line)
    }
    
    pub fn flush(&self) -> anyhow::Result<()> {
        self.writer.flush()
    }
    
    pub fn lines_written(&self) -> u64 {
        self.writer.lines_written()
    }
    
    pub fn bytes_written(&self) -> u64 {
        self.writer.bytes_written()
    }
    
    pub fn path(&self) -> PathBuf {
        self.writer.path()
    }
}

/// Output mode for the processor
pub enum OutputMode {
    /// Single file for all output
    Single(SingleOutputManager),
    /// Multiple files by length
    Multi(MultiOutputManager),
}

impl OutputMode {
    /// Create single output mode
    pub fn single(path: PathBuf, buffer_size: usize) -> anyhow::Result<Self> {
        Ok(Self::Single(SingleOutputManager::new(path, buffer_size)?))
    }
    
    /// Create multi output mode
    pub fn multi(output_dir: PathBuf, prefix: &str, lengths: &[usize], buffer_size: usize) -> anyhow::Result<Self> {
        let mut manager = MultiOutputManager::new(output_dir, prefix, buffer_size);
        manager.init_lengths(lengths)?;
        Ok(Self::Multi(manager))
    }
    
    /// Write a line (routes to appropriate file in multi mode)
    pub fn write_line(&self, line: &str, length: usize) -> anyhow::Result<()> {
        match self {
            Self::Single(mgr) => mgr.write_line(line),
            Self::Multi(mgr) => mgr.write_line(line, length),
        }
    }
    
    /// Flush all outputs
    pub fn flush(&self) -> anyhow::Result<()> {
        match self {
            Self::Single(mgr) => mgr.flush(),
            Self::Multi(mgr) => mgr.flush_all(),
        }
    }
}

/// Generate output filename from input filename
pub fn generate_output_name(input: &Path, suffix: &str) -> String {
    let stem = input.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    
    format!("{}_{}.txt", stem, suffix)
}

/// Ensure output directory exists
pub fn ensure_output_dir(path: &Path) -> anyhow::Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_output_writer() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.txt");
        
        let mut writer = OutputWriter::new(path.clone(), 1024).unwrap();
        writer.write_line("hello").unwrap();
        writer.write_line("world").unwrap();
        writer.flush().unwrap();
        
        assert_eq!(writer.lines_written(), 2);
        
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "hello\nworld\n");
    }
    
    #[test]
    fn test_multi_output_manager() {
        let temp_dir = TempDir::new().unwrap();
        
        let mut manager = MultiOutputManager::new(
            temp_dir.path().to_path_buf(),
            "wordlist",
            1024
        );
        
        manager.init_lengths(&[8, 10]).unwrap();
        
        manager.write_line("password", 8).unwrap();
        manager.write_line("verysecret", 10).unwrap();
        manager.flush_all().unwrap();
        
        let stats = manager.get_stats();
        assert_eq!(stats.len(), 2);
    }
    
    #[test]
    fn test_generate_output_name() {
        let input = Path::new("/path/to/rockyou.txt");
        let name = generate_output_name(input, "len8");
        assert_eq!(name, "rockyou_len8.txt");
    }
}
