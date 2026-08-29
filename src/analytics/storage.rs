// src/analytics/storage.rs
//! Persistent storage backend for simulation metrics and data.
//!
//! Provides file-based storage for:
//! - Metrics time-series data
//! - Emergence patterns
//! - Session recordings
//! - Configuration snapshots

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};

/// Storage backend configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// Base directory for all storage
    pub base_path: PathBuf,
    /// Whether to auto-create directories
    pub auto_create_dirs: bool,
    /// Maximum file size before rotation (bytes)
    pub max_file_size: usize,
    /// Number of backup files to keep
    pub max_backups: usize,
    /// Whether to compress stored data
    pub compress: bool,
    /// Flush interval (write every N entries)
    pub flush_interval: usize,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            base_path: PathBuf::from("./simulation_data"),
            auto_create_dirs: true,
            max_file_size: 100 * 1024 * 1024, // 100 MB
            max_backups: 5,
            compress: false,
            flush_interval: 100,
        }
    }
}

impl StorageConfig {
    /// Create config for a specific directory
    pub fn with_path<P: AsRef<Path>>(path: P) -> Self {
        Self {
            base_path: path.as_ref().to_path_buf(),
            ..Default::default()
        }
    }
}

/// Storage result type
pub type StorageResult<T> = Result<T, StorageError>;

/// Storage errors
#[derive(Debug)]
pub enum StorageError {
    Io(std::io::Error),
    Serialization(String),
    NotFound(String),
    InvalidData(String),
}

impl From<std::io::Error> for StorageError {
    fn from(e: std::io::Error) -> Self {
        StorageError::Io(e)
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(e: serde_json::Error) -> Self {
        StorageError::Serialization(e.to_string())
    }
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::Io(e) => write!(f, "IO error: {}", e),
            StorageError::Serialization(s) => write!(f, "Serialization error: {}", s),
            StorageError::NotFound(s) => write!(f, "Not found: {}", s),
            StorageError::InvalidData(s) => write!(f, "Invalid data: {}", s),
        }
    }
}

impl std::error::Error for StorageError {}

/// Time-series data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub tick: u64,
    pub timestamp: u64,
    pub values: HashMap<String, f64>,
}

impl DataPoint {
    pub fn new(tick: u64) -> Self {
        Self {
            tick,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            values: HashMap::new(),
        }
    }

    pub fn with_value(mut self, key: &str, value: f64) -> Self {
        self.values.insert(key.to_string(), value);
        self
    }

    pub fn set(&mut self, key: &str, value: f64) {
        self.values.insert(key.to_string(), value);
    }

    pub fn get(&self, key: &str) -> Option<f64> {
        self.values.get(key).copied()
    }
}

/// Time-series data store
pub struct TimeSeriesStore {
    config: StorageConfig,
    series_path: PathBuf,
    buffer: Vec<DataPoint>,
    buffer_count: usize,
}

impl TimeSeriesStore {
    /// Create a new time-series store
    pub fn new(config: StorageConfig, series_name: &str) -> StorageResult<Self> {
        let series_path = config.base_path.join("timeseries").join(format!("{}.jsonl", series_name));

        if config.auto_create_dirs {
            if let Some(parent) = series_path.parent() {
                fs::create_dir_all(parent)?;
            }
        }

        Ok(Self {
            config,
            series_path,
            buffer: Vec::new(),
            buffer_count: 0,
        })
    }

    /// Append a data point
    pub fn append(&mut self, point: DataPoint) -> StorageResult<()> {
        self.buffer.push(point);
        self.buffer_count += 1;

        if self.buffer_count >= self.config.flush_interval {
            self.flush()?;
        }

        Ok(())
    }

    /// Flush buffer to disk
    pub fn flush(&mut self) -> StorageResult<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.series_path)?;

        let mut writer = BufWriter::new(file);

        for point in &self.buffer {
            let json = serde_json::to_string(point)?;
            writeln!(writer, "{}", json)?;
        }

        writer.flush()?;
        self.buffer.clear();
        self.buffer_count = 0;

        // Check for rotation
        self.check_rotation()?;

        Ok(())
    }

    /// Check if file needs rotation
    fn check_rotation(&self) -> StorageResult<()> {
        let metadata = fs::metadata(&self.series_path)?;
        if metadata.len() as usize > self.config.max_file_size {
            self.rotate()?;
        }
        Ok(())
    }

    /// Rotate log file
    fn rotate(&self) -> StorageResult<()> {
        // Remove oldest backup
        let oldest = format!("{}.{}", self.series_path.display(), self.config.max_backups);
        let _ = fs::remove_file(&oldest);

        // Shift existing backups
        for i in (1..self.config.max_backups).rev() {
            let from = format!("{}.{}", self.series_path.display(), i);
            let to = format!("{}.{}", self.series_path.display(), i + 1);
            let _ = fs::rename(&from, &to);
        }

        // Rename current to .1
        let backup = format!("{}.1", self.series_path.display());
        fs::rename(&self.series_path, &backup)?;

        Ok(())
    }

    /// Read all data points
    pub fn read_all(&self) -> StorageResult<Vec<DataPoint>> {
        if !self.series_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.series_path)?;
        let reader = BufReader::new(file);
        let mut points = Vec::new();

        for line in std::io::BufRead::lines(reader) {
            let line = line?;
            if !line.is_empty() {
                let point: DataPoint = serde_json::from_str(&line)?;
                points.push(point);
            }
        }

        Ok(points)
    }



    /// Clear all data
    pub fn clear(&mut self) -> StorageResult<()> {
        self.buffer.clear();
        self.buffer_count = 0;
        if self.series_path.exists() {
            fs::remove_file(&self.series_path)?;
        }
        Ok(())
    }

    /// Get total data point count (approximate, from file)
    pub fn count(&self) -> StorageResult<usize> {
        if !self.series_path.exists() {
            return Ok(self.buffer.len());
        }

        let file = File::open(&self.series_path)?;
        let reader = BufReader::new(file);
        let count = std::io::BufRead::lines(reader).count();
        Ok(count + self.buffer.len())
    }
}

impl Drop for TimeSeriesStore {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

/// Key-value document store
pub struct DocumentStore {
    #[allow(dead_code)]
    config: StorageConfig,
    store_path: PathBuf,
}

impl DocumentStore {
    /// Create a new document store
    pub fn new(config: StorageConfig, store_name: &str) -> StorageResult<Self> {
        let store_path = config.base_path.join("documents").join(store_name);

        if config.auto_create_dirs {
            fs::create_dir_all(&store_path)?;
        }

        Ok(Self { config, store_path })
    }

    /// Store a document
    pub fn put<T: Serialize>(&self, key: &str, value: &T) -> StorageResult<()> {
        let file_path = self.store_path.join(format!("{}.json", key));
        let file = File::create(file_path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, value)?;
        Ok(())
    }

    /// Retrieve a document
    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> StorageResult<T> {
        let file_path = self.store_path.join(format!("{}.json", key));
        if !file_path.exists() {
            return Err(StorageError::NotFound(key.to_string()));
        }

        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let value = serde_json::from_reader(reader)?;
        Ok(value)
    }

    /// Check if document exists
    pub fn exists(&self, key: &str) -> bool {
        self.store_path.join(format!("{}.json", key)).exists()
    }

    /// Delete a document
    pub fn delete(&self, key: &str) -> StorageResult<bool> {
        let file_path = self.store_path.join(format!("{}.json", key));
        if file_path.exists() {
            fs::remove_file(file_path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// List all document keys
    pub fn keys(&self) -> StorageResult<Vec<String>> {
        let mut keys = Vec::new();

        if !self.store_path.exists() {
            return Ok(keys);
        }

        for entry in fs::read_dir(&self.store_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Some(stem) = path.file_stem() {
                    if let Some(name) = stem.to_str() {
                        keys.push(name.to_string());
                    }
                }
            }
        }

        Ok(keys)
    }

    /// Clear all documents
    pub fn clear(&self) -> StorageResult<()> {
        if self.store_path.exists() {
            fs::remove_dir_all(&self.store_path)?;
            fs::create_dir_all(&self.store_path)?;
        }
        Ok(())
    }
}

/// Unified storage manager
pub struct StorageManager {
    config: StorageConfig,
    /// Time-series stores by name
    time_series: HashMap<String, TimeSeriesStore>,
    /// Document stores by name
    documents: HashMap<String, DocumentStore>,
}

impl StorageManager {
    /// Create a new storage manager
    pub fn new(config: StorageConfig) -> StorageResult<Self> {
        if config.auto_create_dirs {
            fs::create_dir_all(&config.base_path)?;
        }

        Ok(Self {
            config,
            time_series: HashMap::new(),
            documents: HashMap::new(),
        })
    }

    /// Get or create a time-series store
    pub fn time_series(&mut self, name: &str) -> StorageResult<&mut TimeSeriesStore> {
        if !self.time_series.contains_key(name) {
            let store = TimeSeriesStore::new(self.config.clone(), name)?;
            self.time_series.insert(name.to_string(), store);
        }
        Ok(self.time_series.get_mut(name).unwrap())
    }

    /// Get or create a document store
    pub fn documents(&mut self, name: &str) -> StorageResult<&mut DocumentStore> {
        if !self.documents.contains_key(name) {
            let store = DocumentStore::new(self.config.clone(), name)?;
            self.documents.insert(name.to_string(), store);
        }
        Ok(self.documents.get_mut(name).unwrap())
    }

    /// Flush all stores
    pub fn flush_all(&mut self) -> StorageResult<()> {
        for store in self.time_series.values_mut() {
            store.flush()?;
        }
        Ok(())
    }

    /// Get storage statistics
    pub fn stats(&self) -> StorageStats {
        let mut total_size = 0u64;
        let mut file_count = 0usize;

        if let Ok(entries) = fs::read_dir(&self.config.base_path) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if meta.is_file() {
                        total_size += meta.len();
                        file_count += 1;
                    }
                }
            }
        }

        StorageStats {
            base_path: self.config.base_path.clone(),
            total_size_bytes: total_size,
            file_count,
            time_series_count: self.time_series.len(),
            document_stores_count: self.documents.len(),
        }
    }

}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub base_path: PathBuf,
    pub total_size_bytes: u64,
    pub file_count: usize,
    pub time_series_count: usize,
    pub document_stores_count: usize,
}

impl StorageStats {
}


/// Convenience functions for quick storage operations
pub mod quick {
    use super::*;

    /// Save JSON data to file
    pub fn save_json<T: Serialize, P: AsRef<Path>>(path: P, data: &T) -> StorageResult<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, data)?;
        Ok(())
    }

    /// Load JSON data from file
    pub fn load_json<T: for<'de> Deserialize<'de>, P: AsRef<Path>>(path: P) -> StorageResult<T> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let data = serde_json::from_reader(reader)?;
        Ok(data)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_data_point() {
        let point = DataPoint::new(100)
            .with_value("population", 50.0)
            .with_value("health", 75.5);

        assert_eq!(point.tick, 100);
        assert_eq!(point.get("population"), Some(50.0));
        assert_eq!(point.get("health"), Some(75.5));
        assert_eq!(point.get("missing"), None);
    }

    #[test]
    fn test_time_series_store() {
        let dir = tempdir().unwrap();
        let config = StorageConfig {
            base_path: dir.path().to_path_buf(),
            flush_interval: 2,
            ..Default::default()
        };

        let mut store = TimeSeriesStore::new(config, "test_series").unwrap();

        // Add points
        store.append(DataPoint::new(1).with_value("val", 1.0)).unwrap();
        store.append(DataPoint::new(2).with_value("val", 2.0)).unwrap();
        store.append(DataPoint::new(3).with_value("val", 3.0)).unwrap();

        // Flush and read
        store.flush().unwrap();
        let points = store.read_all().unwrap();

        assert_eq!(points.len(), 3);
        assert_eq!(points[0].tick, 1);
        assert_eq!(points[2].get("val"), Some(3.0));
    }

    #[test]
    fn test_document_store() {
        let dir = tempdir().unwrap();
        let config = StorageConfig::with_path(dir.path());

        let store = DocumentStore::new(config, "test_docs").unwrap();

        // Store and retrieve
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct TestDoc {
            name: String,
            value: i32,
        }

        let doc = TestDoc {
            name: "test".to_string(),
            value: 42,
        };

        store.put("doc1", &doc).unwrap();
        assert!(store.exists("doc1"));

        let retrieved: TestDoc = store.get("doc1").unwrap();
        assert_eq!(retrieved, doc);

        // List keys
        store.put("doc2", &doc).unwrap();
        let keys = store.keys().unwrap();
        assert_eq!(keys.len(), 2);

        // Delete
        store.delete("doc1").unwrap();
        assert!(!store.exists("doc1"));
    }

    #[test]
    fn test_storage_manager() {
        let dir = tempdir().unwrap();
        let config = StorageConfig::with_path(dir.path());

        let mut manager = StorageManager::new(config).unwrap();

        // Use time series
        {
            let ts = manager.time_series("metrics").unwrap();
            ts.append(DataPoint::new(1).with_value("x", 1.0)).unwrap();
        }

        // Use documents
        {
            let docs = manager.documents("config").unwrap();
            docs.put("settings", &vec!["a", "b", "c"]).unwrap();
        }

        // Get stats
        manager.flush_all().unwrap();
        let stats = manager.stats();
        assert!(stats.time_series_count > 0);
    }

    #[test]
    fn test_quick_functions() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");

        let data = vec![1, 2, 3, 4, 5];
        quick::save_json(&path, &data).unwrap();

        let loaded: Vec<i32> = quick::load_json(&path).unwrap();
        assert_eq!(loaded, data);
    }
}
