// src/visualization/streaming.rs
//! Streaming visualization output for real-time monitoring.
//!
//! Provides output streams that can be consumed by external tools:
//! - JSON Lines (JSONL) streaming for log aggregators
//! - CSV streaming for spreadsheet tools
//! - Custom formatters for specialized output
//! - WebSocket-compatible message formatting

use std::io::{Write, BufWriter};
use std::fs::File;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// Output format for streaming
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamFormat {
    /// JSON Lines format (one JSON object per line)
    JsonLines,
    /// CSV format with headers
    Csv,
    /// Plain text with custom formatting
    PlainText,
    /// Compact single-line format
    Compact,
}

/// A streaming output destination
pub trait StreamOutput: Send + Sync {
    /// Write a line to the output
    fn write_line(&self, line: &str) -> std::io::Result<()>;

    /// Flush the output
    fn flush(&self) -> std::io::Result<()>;
}

/// Console output stream
pub struct ConsoleOutput;

impl StreamOutput for ConsoleOutput {
    fn write_line(&self, line: &str) -> std::io::Result<()> {
        println!("{}", line);
        Ok(())
    }

    fn flush(&self) -> std::io::Result<()> {
        std::io::stdout().flush()
    }
}

/// File output stream
pub struct FileOutput {
    writer: Mutex<BufWriter<File>>,
}

impl FileOutput {
    pub fn new(path: &str) -> std::io::Result<Self> {
        let file = File::create(path)?;
        Ok(Self {
            writer: Mutex::new(BufWriter::new(file)),
        })
    }
}

impl StreamOutput for FileOutput {
    fn write_line(&self, line: &str) -> std::io::Result<()> {
        let mut writer = self.writer.lock().unwrap();
        writeln!(writer, "{}", line)
    }

    fn flush(&self) -> std::io::Result<()> {
        self.writer.lock().unwrap().flush()
    }
}

/// In-memory buffer output (for testing or buffering)
pub struct BufferOutput {
    lines: Mutex<Vec<String>>,
    max_lines: usize,
}

impl BufferOutput {
    pub fn new(max_lines: usize) -> Self {
        Self {
            lines: Mutex::new(Vec::new()),
            max_lines,
        }
    }

    pub fn get_lines(&self) -> Vec<String> {
        self.lines.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.lines.lock().unwrap().clear();
    }
}

impl StreamOutput for BufferOutput {
    fn write_line(&self, line: &str) -> std::io::Result<()> {
        let mut lines = self.lines.lock().unwrap();
        if lines.len() >= self.max_lines {
            lines.remove(0);
        }
        lines.push(line.to_string());
        Ok(())
    }

    fn flush(&self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Multi-output that writes to multiple destinations
pub struct MultiOutput {
    outputs: Vec<Arc<dyn StreamOutput>>,
}

impl MultiOutput {
    pub fn new() -> Self {
        Self { outputs: Vec::new() }
    }

    pub fn add<O: StreamOutput + 'static>(&mut self, output: O) {
        self.outputs.push(Arc::new(output));
    }
}

impl StreamOutput for MultiOutput {
    fn write_line(&self, line: &str) -> std::io::Result<()> {
        for output in &self.outputs {
            output.write_line(line)?;
        }
        Ok(())
    }

    fn flush(&self) -> std::io::Result<()> {
        for output in &self.outputs {
            output.flush()?;
        }
        Ok(())
    }
}

impl Default for MultiOutput {
    fn default() -> Self {
        Self::new()
    }
}

/// Streaming event for real-time output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    /// Event type
    pub event_type: String,
    /// Tick number
    pub tick: u64,
    /// Timestamp (milliseconds)
    pub timestamp: u64,
    /// Event data
    pub data: serde_json::Value,
}

impl StreamEvent {
    pub fn new(event_type: &str, tick: u64, data: serde_json::Value) -> Self {
        Self {
            event_type: event_type.to_string(),
            tick,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            data,
        }
    }

    /// Create a tick event
    pub fn tick(tick: u64, population: usize, health: f32, happiness: f32) -> Self {
        Self::new(
            "tick",
            tick,
            serde_json::json!({
                "population": population,
                "average_health": health,
                "average_happiness": happiness,
            }),
        )
    }

    /// Create a birth event
    pub fn birth(tick: u64, agent_id: Uuid, position: (i32, i32, i32)) -> Self {
        Self::new(
            "birth",
            tick,
            serde_json::json!({
                "agent_id": agent_id.to_string(),
                "position": position,
            }),
        )
    }

    /// Create a death event
    pub fn death(tick: u64, agent_id: Uuid, cause: &str) -> Self {
        Self::new(
            "death",
            tick,
            serde_json::json!({
                "agent_id": agent_id.to_string(),
                "cause": cause,
            }),
        )
    }

    /// Create an emergence event
    pub fn emergence(tick: u64, pattern: &str, severity: f32) -> Self {
        Self::new(
            "emergence",
            tick,
            serde_json::json!({
                "pattern": pattern,
                "severity": severity,
            }),
        )
    }
}

/// Stream formatter for converting events to output format
pub struct StreamFormatter {
    format: StreamFormat,
    /// CSV headers (used for first line)
    csv_headers_written: bool,
}

impl StreamFormatter {
    pub fn new(format: StreamFormat) -> Self {
        Self {
            format,
            csv_headers_written: false,
        }
    }

    /// Format an event for output
    pub fn format(&mut self, event: &StreamEvent) -> String {
        match self.format {
            StreamFormat::JsonLines => {
                serde_json::to_string(event).unwrap_or_default()
            }
            StreamFormat::Csv => {
                self.format_csv(event)
            }
            StreamFormat::PlainText => {
                format!(
                    "[{}] Tick {}: {} - {:?}",
                    event.timestamp, event.tick, event.event_type, event.data
                )
            }
            StreamFormat::Compact => {
                format!(
                    "{}:{}:{}",
                    event.tick, event.event_type,
                    serde_json::to_string(&event.data).unwrap_or_default()
                )
            }
        }
    }

    fn format_csv(&mut self, event: &StreamEvent) -> String {
        let mut result = String::new();

        if !self.csv_headers_written {
            result.push_str("timestamp,tick,event_type,data\n");
            self.csv_headers_written = true;
        }

        let data_str = serde_json::to_string(&event.data)
            .unwrap_or_default()
            .replace('"', "\"\""); // Escape quotes for CSV

        result.push_str(&format!(
            "{},{},{},\"{}\"\n",
            event.timestamp, event.tick, event.event_type, data_str
        ));

        result
    }
}

/// Streaming visualizer that outputs to a stream
pub struct StreamingVisualizer {
    output: Arc<dyn StreamOutput>,
    formatter: StreamFormatter,
    /// Filter event types (empty = all)
    event_filter: Vec<String>,
    /// Minimum tick interval between outputs (0 = every tick)
    tick_interval: u64,
    last_output_tick: u64,
}

impl StreamingVisualizer {
    pub fn new(output: Arc<dyn StreamOutput>, format: StreamFormat) -> Self {
        Self {
            output,
            formatter: StreamFormatter::new(format),
            event_filter: Vec::new(),
            tick_interval: 0,
            last_output_tick: 0,
        }
    }

    /// Create with console output
    pub fn console(format: StreamFormat) -> Self {
        Self::new(Arc::new(ConsoleOutput), format)
    }

    /// Create with file output
    pub fn file(path: &str, format: StreamFormat) -> std::io::Result<Self> {
        Ok(Self::new(Arc::new(FileOutput::new(path)?), format))
    }

    /// Set event type filter
    pub fn with_filter(mut self, event_types: Vec<String>) -> Self {
        self.event_filter = event_types;
        self
    }


    /// Emit an event
    pub fn emit(&mut self, event: StreamEvent) -> std::io::Result<()> {
        // Check filter
        if !self.event_filter.is_empty() && !self.event_filter.contains(&event.event_type) {
            return Ok(());
        }

        // Check interval for tick events
        if event.event_type == "tick" && self.tick_interval > 0 {
            if event.tick < self.last_output_tick + self.tick_interval {
                return Ok(());
            }
            self.last_output_tick = event.tick;
        }

        let line = self.formatter.format(&event);
        self.output.write_line(&line)
    }

    /// Flush output
    pub fn flush(&self) -> std::io::Result<()> {
        self.output.flush()
    }
}

/// Configuration for visualization streaming
#[derive(Debug, Clone)]
pub struct StreamConfig {
    pub format: StreamFormat,
    pub output_path: Option<String>,
    pub event_filter: Vec<String>,
    pub tick_interval: u64,
    pub buffer_size: usize,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            format: StreamFormat::JsonLines,
            output_path: None,
            event_filter: Vec::new(),
            tick_interval: 1,
            buffer_size: 1000,
        }
    }
}

/// Custom display widget trait
pub trait DisplayWidget: Send + Sync {
    /// Get widget name
    fn name(&self) -> &str;

    /// Get widget width (characters)
    fn width(&self) -> usize;

    /// Get widget height (lines)
    fn height(&self) -> usize;

    /// Render the widget to lines
    fn render(&self, data: &WidgetData) -> Vec<String>;
}

/// Data passed to widgets for rendering
#[derive(Debug, Clone, Default)]
pub struct WidgetData {
    pub tick: u64,
    pub population_size: usize,
    pub average_health: f32,
    pub average_happiness: f32,
    pub births: u64,
    pub deaths: u64,
    pub custom: std::collections::BTreeMap<String, String>,
}

/// Simple text widget
pub struct TextWidget {
    name: String,
    width: usize,
    height: usize,
    template: String,
}

impl TextWidget {
    pub fn new(name: &str, width: usize, height: usize, template: &str) -> Self {
        Self {
            name: name.to_string(),
            width,
            height,
            template: template.to_string(),
        }
    }
}

impl DisplayWidget for TextWidget {
    fn name(&self) -> &str {
        &self.name
    }

    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn render(&self, data: &WidgetData) -> Vec<String> {
        let text = self.template
            .replace("{tick}", &data.tick.to_string())
            .replace("{population}", &data.population_size.to_string())
            .replace("{health}", &format!("{:.1}", data.average_health))
            .replace("{happiness}", &format!("{:.2}", data.average_happiness))
            .replace("{births}", &data.births.to_string())
            .replace("{deaths}", &data.deaths.to_string());

        text.lines()
            .take(self.height)
            .map(|s| {
                let s = s.to_string();
                if s.len() < self.width {
                    format!("{:width$}", s, width = self.width)
                } else {
                    s[..self.width].to_string()
                }
            })
            .collect()
    }
}

/// Progress bar widget
pub struct ProgressWidget {
    name: String,
    width: usize,
    label: String,
    value_key: String,
    max_value: f32,
}

impl ProgressWidget {
    pub fn new(name: &str, width: usize, label: &str, value_key: &str, max_value: f32) -> Self {
        Self {
            name: name.to_string(),
            width,
            label: label.to_string(),
            value_key: value_key.to_string(),
            max_value,
        }
    }
}

impl DisplayWidget for ProgressWidget {
    fn name(&self) -> &str {
        &self.name
    }

    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        1
    }

    fn render(&self, data: &WidgetData) -> Vec<String> {
        let value = data.custom.get(&self.value_key)
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.0);

        let ratio = (value / self.max_value).clamp(0.0, 1.0);
        let bar_width = self.width.saturating_sub(self.label.len() + 7); // label + space + [###] + space
        let filled = (ratio * bar_width as f32) as usize;
        let empty = bar_width - filled;

        vec![format!(
            "{} [{}{}] {:>3.0}%",
            self.label,
            "█".repeat(filled),
            "░".repeat(empty),
            ratio * 100.0
        )]
    }
}

/// Widget container for custom dashboards
pub struct WidgetDashboard {
    widgets: Vec<Box<dyn DisplayWidget>>,
    #[allow(dead_code)]
    width: usize,
}

impl WidgetDashboard {
    pub fn new(width: usize) -> Self {
        Self {
            widgets: Vec::new(),
            width,
        }
    }


    pub fn render(&self, data: &WidgetData) -> String {
        let mut output = String::new();

        for widget in &self.widgets {
            let lines = widget.render(data);
            for line in lines {
                output.push_str(&line);
                output.push('\n');
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_event_creation() {
        let event = StreamEvent::tick(100, 50, 75.0, 0.6);
        assert_eq!(event.event_type, "tick");
        assert_eq!(event.tick, 100);
    }

    #[test]
    fn test_formatter_jsonl() {
        let mut formatter = StreamFormatter::new(StreamFormat::JsonLines);
        let event = StreamEvent::tick(100, 50, 75.0, 0.6);
        let output = formatter.format(&event);

        assert!(output.contains("\"tick\":100"));
        assert!(output.contains("\"event_type\":\"tick\""));
    }

    #[test]
    fn test_formatter_csv() {
        let mut formatter = StreamFormatter::new(StreamFormat::Csv);
        let event = StreamEvent::tick(100, 50, 75.0, 0.6);
        let output = formatter.format(&event);

        assert!(output.contains("timestamp,tick,event_type,data"));
        assert!(output.contains("100,tick,"));
    }

    #[test]
    fn test_buffer_output() {
        let buffer = BufferOutput::new(5);
        buffer.write_line("line1").unwrap();
        buffer.write_line("line2").unwrap();
        buffer.write_line("line3").unwrap();

        let lines = buffer.get_lines();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "line1");
    }

    #[test]
    fn test_text_widget() {
        let widget = TextWidget::new("test", 30, 2, "Tick: {tick}\nPop: {population}");
        let data = WidgetData {
            tick: 100,
            population_size: 50,
            ..Default::default()
        };

        let lines = widget.render(&data);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("100"));
        assert!(lines[1].contains("50"));
    }

    #[test]
    fn test_streaming_visualizer_filter() {
        let buffer = Arc::new(BufferOutput::new(100));
        let mut viz = StreamingVisualizer::new(buffer.clone(), StreamFormat::JsonLines)
            .with_filter(vec!["birth".to_string(), "death".to_string()]);

        // This should be filtered out
        viz.emit(StreamEvent::tick(1, 50, 75.0, 0.6)).unwrap();

        // This should pass through
        viz.emit(StreamEvent::birth(1, crate::core::dice::name(), (0, 0, 0))).unwrap();

        let lines = buffer.get_lines();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("birth"));
    }
}
