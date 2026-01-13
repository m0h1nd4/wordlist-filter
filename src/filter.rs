//! Word filtering module
//!
//! Provides filtering capabilities for words based on length and regex patterns.

use regex::Regex;
use std::collections::HashSet;

/// Filter configuration
#[derive(Debug, Clone)]
pub struct FilterConfig {
    /// Lengths to include (None means no length filter)
    pub lengths: Option<HashSet<usize>>,
    /// Regex pattern to match (None means no pattern filter)
    pub pattern: Option<Regex>,
}

impl FilterConfig {
    /// Create a new filter configuration
    pub fn new(lengths: Option<Vec<usize>>, pattern: Option<&str>) -> anyhow::Result<Self> {
        let lengths = lengths.map(|l| l.into_iter().collect());
        
        let pattern = match pattern {
            Some(p) if !p.is_empty() => {
                let regex = Regex::new(p)
                    .map_err(|e| anyhow::anyhow!("Invalid regex pattern '{}': {}", p, e))?;
                Some(regex)
            }
            _ => None,
        };
        
        Ok(Self { lengths, pattern })
    }
    
    /// Check if a word matches the filter criteria
    #[inline]
    pub fn matches(&self, word: &str) -> bool {
        // Check length filter
        if let Some(ref lengths) = self.lengths {
            if !lengths.contains(&word.chars().count()) {
                return false;
            }
        }
        
        // Check pattern filter
        if let Some(ref pattern) = self.pattern {
            if !pattern.is_match(word) {
                return false;
            }
        }
        
        true
    }
    
    /// Check if a word matches a specific length
    #[inline]
    pub fn matches_length(&self, word: &str, length: usize) -> bool {
        let word_len = word.chars().count();
        
        if word_len != length {
            return false;
        }
        
        // Check pattern filter
        if let Some(ref pattern) = self.pattern {
            if !pattern.is_match(word) {
                return false;
            }
        }
        
        true
    }
    
    /// Get the length category for a word (for routing to correct output file)
    #[inline]
    pub fn get_length(&self, word: &str) -> usize {
        word.chars().count()
    }
    
    /// Check if we have any filters active
    pub fn has_filters(&self) -> bool {
        self.lengths.is_some() || self.pattern.is_some()
    }
    
    /// Check if we have a length filter
    pub fn has_length_filter(&self) -> bool {
        self.lengths.is_some()
    }
    
    /// Check if we have a pattern filter
    pub fn has_pattern_filter(&self) -> bool {
        self.pattern.is_some()
    }
    
    /// Get the configured lengths
    pub fn get_lengths(&self) -> Option<Vec<usize>> {
        self.lengths.as_ref().map(|l| {
            let mut v: Vec<_> = l.iter().copied().collect();
            v.sort_unstable();
            v
        })
    }
}

/// Optimized filter for single-length filtering
#[derive(Debug)]
pub struct SingleLengthFilter {
    length: usize,
    pattern: Option<Regex>,
}

impl SingleLengthFilter {
    pub fn new(length: usize, pattern: Option<&str>) -> anyhow::Result<Self> {
        let pattern = match pattern {
            Some(p) if !p.is_empty() => {
                let regex = Regex::new(p)
                    .map_err(|e| anyhow::anyhow!("Invalid regex pattern '{}': {}", p, e))?;
                Some(regex)
            }
            _ => None,
        };
        
        Ok(Self { length, pattern })
    }
    
    #[inline]
    pub fn matches(&self, word: &str) -> bool {
        // Fast byte-length check first for ASCII-only optimization
        if word.is_ascii() {
            if word.len() != self.length {
                return false;
            }
        } else {
            if word.chars().count() != self.length {
                return false;
            }
        }
        
        // Check pattern filter
        if let Some(ref pattern) = self.pattern {
            if !pattern.is_match(word) {
                return false;
            }
        }
        
        true
    }
}

/// Multi-output filter that routes words to different length buckets
pub struct MultiLengthRouter {
    lengths: Vec<usize>,
    pattern: Option<Regex>,
}

impl MultiLengthRouter {
    pub fn new(lengths: Vec<usize>, pattern: Option<&str>) -> anyhow::Result<Self> {
        let pattern = match pattern {
            Some(p) if !p.is_empty() => {
                let regex = Regex::new(p)
                    .map_err(|e| anyhow::anyhow!("Invalid regex pattern '{}': {}", p, e))?;
                Some(regex)
            }
            _ => None,
        };
        
        Ok(Self { lengths, pattern })
    }
    
    /// Get the index of the length bucket this word belongs to, if any
    #[inline]
    pub fn route(&self, word: &str) -> Option<usize> {
        let word_len = if word.is_ascii() {
            word.len()
        } else {
            word.chars().count()
        };
        
        // Check pattern first if present
        if let Some(ref pattern) = self.pattern {
            if !pattern.is_match(word) {
                return None;
            }
        }
        
        // Find matching length bucket
        self.lengths.iter().position(|&l| l == word_len)
    }
    
    /// Get all configured lengths
    pub fn lengths(&self) -> &[usize] {
        &self.lengths
    }
}

/// Pattern-only filter (no length restriction)
pub struct PatternFilter {
    pattern: Regex,
}

impl PatternFilter {
    pub fn new(pattern: &str) -> anyhow::Result<Self> {
        let regex = Regex::new(pattern)
            .map_err(|e| anyhow::anyhow!("Invalid regex pattern '{}': {}", pattern, e))?;
        
        Ok(Self { pattern: regex })
    }
    
    #[inline]
    pub fn matches(&self, word: &str) -> bool {
        self.pattern.is_match(word)
    }
    
    pub fn pattern_str(&self) -> &str {
        self.pattern.as_str()
    }
}

/// Helper to validate a regex pattern before use
pub fn validate_pattern(pattern: &str) -> anyhow::Result<()> {
    Regex::new(pattern)
        .map_err(|e| anyhow::anyhow!("Invalid regex pattern '{}': {}", pattern, e))?;
    Ok(())
}

/// Common regex patterns for wordlist filtering
pub mod patterns {
    /// Only lowercase letters
    pub const LOWERCASE_ONLY: &str = r"^[a-z]+$";
    
    /// Only uppercase letters
    pub const UPPERCASE_ONLY: &str = r"^[A-Z]+$";
    
    /// Only letters (any case)
    pub const LETTERS_ONLY: &str = r"^[a-zA-Z]+$";
    
    /// Only digits
    pub const DIGITS_ONLY: &str = r"^[0-9]+$";
    
    /// Alphanumeric only
    pub const ALPHANUMERIC: &str = r"^[a-zA-Z0-9]+$";
    
    /// Contains at least one special character
    pub const HAS_SPECIAL: &str = r".*[!@#$%^&*()_+\-=\[\]{};':\"\\|,.<>\/?].*";
    
    /// Starts with letter, ends with digit
    pub const LETTER_START_DIGIT_END: &str = r"^[a-zA-Z].*[0-9]$";
    
    /// Common password pattern (letter + digits)
    pub const COMMON_PASSWORD: &str = r"^[a-zA-Z]+[0-9]+$";
    
    /// Complex password (upper, lower, digit)
    pub const COMPLEX_PASSWORD: &str = r"^(?=.*[a-z])(?=.*[A-Z])(?=.*[0-9]).+$";
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_filter_config_length() {
        let config = FilterConfig::new(Some(vec![8]), None).unwrap();
        
        assert!(config.matches("password"));  // 8 chars
        assert!(!config.matches("pass"));     // 4 chars
        assert!(!config.matches("verylongpassword")); // 16 chars
    }
    
    #[test]
    fn test_filter_config_pattern() {
        let config = FilterConfig::new(None, Some(r"^[a-z]+$")).unwrap();
        
        assert!(config.matches("password"));
        assert!(!config.matches("Password")); // Has uppercase
        assert!(!config.matches("pass123"));  // Has digits
    }
    
    #[test]
    fn test_filter_config_combined() {
        let config = FilterConfig::new(Some(vec![8]), Some(r"^[a-z]+$")).unwrap();
        
        assert!(config.matches("password"));  // 8 chars, lowercase
        assert!(!config.matches("PASSWORD")); // 8 chars, uppercase
        assert!(!config.matches("pass"));     // 4 chars, lowercase
    }
    
    #[test]
    fn test_single_length_filter() {
        let filter = SingleLengthFilter::new(8, None).unwrap();
        
        assert!(filter.matches("password"));
        assert!(!filter.matches("pass"));
    }
    
    #[test]
    fn test_multi_length_router() {
        let router = MultiLengthRouter::new(vec![6, 8, 10], None).unwrap();
        
        assert_eq!(router.route("secret"), Some(0));   // 6 chars -> index 0
        assert_eq!(router.route("password"), Some(1)); // 8 chars -> index 1
        assert_eq!(router.route("verysecret"), Some(2)); // 10 chars -> index 2
        assert_eq!(router.route("pass"), None);        // 4 chars -> no match
    }
    
    #[test]
    fn test_unicode_length() {
        let config = FilterConfig::new(Some(vec![5]), None).unwrap();
        
        assert!(config.matches("hëllo")); // 5 unicode chars
        assert!(config.matches("hello")); // 5 ascii chars
    }
    
    #[test]
    fn test_pattern_filter() {
        let filter = PatternFilter::new(r"^[a-z]{4}[0-9]{4}$").unwrap();
        
        assert!(filter.matches("pass1234"));
        assert!(!filter.matches("password"));
        assert!(!filter.matches("PASS1234"));
    }
}
