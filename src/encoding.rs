//! Encoding detection and transcoding module
//!
//! Provides automatic detection of file encodings and transcoding to UTF-8.

use chardetng::EncodingDetector;
use encoding_rs::Encoding;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

/// Result of encoding detection
#[derive(Debug, Clone)]
pub struct EncodingInfo {
    /// Detected encoding name
    pub name: &'static str,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f32,
    /// The encoding_rs Encoding reference
    pub encoding: &'static Encoding,
}

impl Default for EncodingInfo {
    fn default() -> Self {
        Self {
            name: "UTF-8",
            confidence: 1.0,
            encoding: encoding_rs::UTF_8,
        }
    }
}

/// Detect the encoding of a file by sampling its content
pub fn detect_encoding(path: &Path) -> anyhow::Result<EncodingInfo> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    
    // Read sample for detection (first 64KB should be enough)
    let mut sample = vec![0u8; 64 * 1024];
    let bytes_read = reader.read(&mut sample)?;
    sample.truncate(bytes_read);
    
    if bytes_read == 0 {
        return Ok(EncodingInfo::default());
    }
    
    // Check for BOM first
    if let Some(encoding) = detect_bom(&sample) {
        return Ok(EncodingInfo {
            name: encoding.name(),
            confidence: 1.0,
            encoding,
        });
    }
    
    // Try to detect encoding using chardetng
    let mut detector = EncodingDetector::new();
    detector.feed(&sample, true);
    
    let encoding = detector.guess(None, true);
    
    // Calculate a rough confidence based on whether the content is valid UTF-8
    let confidence = if encoding == encoding_rs::UTF_8 {
        if std::str::from_utf8(&sample).is_ok() {
            1.0
        } else {
            0.5
        }
    } else {
        0.8
    };
    
    Ok(EncodingInfo {
        name: encoding.name(),
        confidence,
        encoding,
    })
}

/// Detect BOM (Byte Order Mark) at the start of content
fn detect_bom(content: &[u8]) -> Option<&'static Encoding> {
    if content.len() >= 3 && content[0..3] == [0xEF, 0xBB, 0xBF] {
        return Some(encoding_rs::UTF_8);
    }
    if content.len() >= 2 {
        if content[0..2] == [0xFE, 0xFF] {
            return Some(encoding_rs::UTF_16BE);
        }
        if content[0..2] == [0xFF, 0xFE] {
            return Some(encoding_rs::UTF_16LE);
        }
    }
    None
}

/// Reader that automatically transcodes content to UTF-8
pub struct TranscodingReader<R: Read> {
    inner: R,
    encoding: &'static Encoding,
    buffer: Vec<u8>,
    output_buffer: String,
    position: usize,
}

impl<R: Read> TranscodingReader<R> {
    pub fn new(inner: R, encoding: &'static Encoding) -> Self {
        Self {
            inner,
            encoding,
            buffer: Vec::with_capacity(64 * 1024),
            output_buffer: String::new(),
            position: 0,
        }
    }
}

/// A line iterator that handles different encodings
pub struct EncodedLineIterator {
    reader: BufReader<File>,
    encoding: &'static Encoding,
    line_buffer: Vec<u8>,
}

impl EncodedLineIterator {
    /// Create a new line iterator for a file with automatic encoding detection
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        let encoding_info = detect_encoding(path)?;
        let file = File::open(path)?;
        
        Ok(Self {
            reader: BufReader::with_capacity(64 * 1024, file),
            encoding: encoding_info.encoding,
            line_buffer: Vec::with_capacity(4096),
        })
    }
    
    /// Create with a specific encoding
    pub fn with_encoding(path: &Path, encoding: &'static Encoding) -> anyhow::Result<Self> {
        let file = File::open(path)?;
        
        Ok(Self {
            reader: BufReader::with_capacity(64 * 1024, file),
            encoding,
            line_buffer: Vec::with_capacity(4096),
        })
    }
    
    /// Get the detected encoding
    pub fn encoding(&self) -> &'static Encoding {
        self.encoding
    }
}

impl Iterator for EncodedLineIterator {
    type Item = anyhow::Result<String>;
    
    fn next(&mut self) -> Option<Self::Item> {
        self.line_buffer.clear();
        
        match self.reader.read_until(b'\n', &mut self.line_buffer) {
            Ok(0) => None, // EOF
            Ok(_) => {
                // Remove trailing newline characters
                while self.line_buffer.last() == Some(&b'\n') 
                    || self.line_buffer.last() == Some(&b'\r') {
                    self.line_buffer.pop();
                }
                
                // Decode the line
                if self.encoding == encoding_rs::UTF_8 {
                    // Fast path for UTF-8
                    match String::from_utf8(self.line_buffer.clone()) {
                        Ok(s) => Some(Ok(s)),
                        Err(e) => {
                            // Try lossy conversion for invalid UTF-8
                            Some(Ok(String::from_utf8_lossy(e.as_bytes()).into_owned()))
                        }
                    }
                } else {
                    // Transcode from other encodings
                    let (decoded, _, had_errors) = self.encoding.decode(&self.line_buffer);
                    if had_errors {
                        // Log warning but continue
                        log::warn!("Encoding errors in line, using lossy conversion");
                    }
                    Some(Ok(decoded.into_owned()))
                }
            }
            Err(e) => Some(Err(e.into())),
        }
    }
}

/// Memory-mapped file reader for large files
pub struct MmapLineIterator {
    mmap: memmap2::Mmap,
    encoding: &'static Encoding,
    position: usize,
}

impl MmapLineIterator {
    /// Create a new memory-mapped line iterator
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        let encoding_info = detect_encoding(path)?;
        let file = File::open(path)?;
        let mmap = unsafe { memmap2::Mmap::map(&file)? };
        
        // Skip BOM if present
        let position = if mmap.len() >= 3 && mmap[0..3] == [0xEF, 0xBB, 0xBF] {
            3
        } else if mmap.len() >= 2 && (mmap[0..2] == [0xFE, 0xFF] || mmap[0..2] == [0xFF, 0xFE]) {
            2
        } else {
            0
        };
        
        Ok(Self {
            mmap,
            encoding: encoding_info.encoding,
            position,
        })
    }
    
    /// Get the total size of the file
    pub fn size(&self) -> usize {
        self.mmap.len()
    }
    
    /// Get current position
    pub fn position(&self) -> usize {
        self.position
    }
    
    /// Get the detected encoding
    pub fn encoding(&self) -> &'static Encoding {
        self.encoding
    }
}

impl Iterator for MmapLineIterator {
    type Item = anyhow::Result<String>;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.mmap.len() {
            return None;
        }
        
        // Find end of line
        let remaining = &self.mmap[self.position..];
        let line_end = memchr::memchr(b'\n', remaining)
            .map(|i| i + 1)
            .unwrap_or(remaining.len());
        
        let line_bytes = &remaining[..line_end];
        self.position += line_end;
        
        // Remove trailing newline/carriage return
        let line_bytes = line_bytes
            .strip_suffix(&[b'\n'])
            .unwrap_or(line_bytes);
        let line_bytes = line_bytes
            .strip_suffix(&[b'\r'])
            .unwrap_or(line_bytes);
        
        // Decode the line
        if self.encoding == encoding_rs::UTF_8 {
            match std::str::from_utf8(line_bytes) {
                Ok(s) => Some(Ok(s.to_string())),
                Err(_) => Some(Ok(String::from_utf8_lossy(line_bytes).into_owned())),
            }
        } else {
            let (decoded, _, _) = self.encoding.decode(line_bytes);
            Some(Ok(decoded.into_owned()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_utf8_detection() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Hello, World!").unwrap();
        writeln!(file, "Привет мир!").unwrap();
        
        let info = detect_encoding(file.path()).unwrap();
        assert_eq!(info.name, "UTF-8");
    }
    
    #[test]
    fn test_line_iterator() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "line1").unwrap();
        writeln!(file, "line2").unwrap();
        writeln!(file, "line3").unwrap();
        
        let iter = EncodedLineIterator::new(file.path()).unwrap();
        let lines: Vec<_> = iter.filter_map(|r| r.ok()).collect();
        
        assert_eq!(lines, vec!["line1", "line2", "line3"]);
    }
}
