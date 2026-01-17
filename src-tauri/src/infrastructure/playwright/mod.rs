//! Playwright subprocess wrapper for web page screenshot capture.
//!
//! This module provides a Rust wrapper around the Node.js Playwright script
//! to capture full-page screenshots suitable for OCR processing.

use crate::domain::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::sync::mpsc;

/// Configuration for Playwright capture
#[derive(Debug, Clone)]
pub struct PlaywrightConfig {
    /// Path to the playwright-capture.js script
    pub script_path: PathBuf,
    /// Viewport width (default: 1280)
    pub viewport_width: u32,
    /// Viewport height (default: 2000)
    pub viewport_height: u32,
    /// Device scale factor for high-DPI captures (default: 2)
    pub device_scale: f32,
}

impl Default for PlaywrightConfig {
    fn default() -> Self {
        Self {
            script_path: PathBuf::new(),
            viewport_width: 1280,
            viewport_height: 2000,
            device_scale: 2.0,
        }
    }
}

/// Tile information from capture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedTile {
    pub index: usize,
    pub path: String,
    #[serde(rename = "yOffset")]
    pub y_offset: u32,
    pub width: u32,
    pub height: u32,
}

/// Capture manifest containing all metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureManifest {
    pub engine: String,
    pub version: String,
    pub timestamp: String,
    pub url: String,
    #[serde(rename = "originalUrl")]
    pub original_url: String,
    pub title: String,
    pub viewport: ViewportConfig,
    #[serde(rename = "pageSize")]
    pub page_size: PageSize,
    #[serde(rename = "tileOverlap")]
    pub tile_overlap: u32,
    pub tiles: Vec<CapturedTile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewportConfig {
    pub width: u32,
    pub height: u32,
    #[serde(rename = "deviceScaleFactor")]
    pub device_scale_factor: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSize {
    pub width: u32,
    pub height: u32,
}

/// Progress status from capture process
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum CaptureProgress {
    #[serde(rename = "navigating")]
    Navigating { url: String },
    #[serde(rename = "retrying")]
    Retrying { reason: String },
    #[serde(rename = "dimensions")]
    Dimensions { width: u32, height: u32 },
    #[serde(rename = "capturing")]
    Capturing {
        #[serde(rename = "numTiles")]
        num_tiles: usize,
    },
    #[serde(rename = "tile_captured")]
    TileCaptured { index: usize, total: usize },
    #[serde(rename = "complete")]
    Complete {
        #[serde(rename = "tilesCount")]
        tiles_count: usize,
        #[serde(rename = "manifestPath")]
        manifest_path: String,
    },
}

/// Playwright capture service
pub struct PlaywrightCapture {
    config: PlaywrightConfig,
}

impl PlaywrightCapture {
    /// Create a new PlaywrightCapture with the given script path
    pub fn new(script_path: PathBuf) -> Self {
        Self {
            config: PlaywrightConfig {
                script_path,
                ..Default::default()
            },
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: PlaywrightConfig) -> Self {
        Self { config }
    }

    /// Check if Node.js is available
    pub fn check_nodejs() -> Result<String> {
        let output = Command::new("node")
            .arg("--version")
            .output()
            .map_err(|e| {
                AppError::Internal(format!(
                    "Node.js not found. Please install Node.js to use web OCR capture: {}",
                    e
                ))
            })?;

        if !output.status.success() {
            return Err(AppError::Internal(
                "Failed to get Node.js version".to_string(),
            ));
        }

        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(version)
    }

    /// Check if Playwright is installed, install if needed
    pub async fn ensure_playwright_installed() -> Result<()> {
        // Check if playwright is available
        let check = Command::new("npx")
            .args(["playwright", "--version"])
            .output();

        match check {
            Ok(output) if output.status.success() => {
                // Playwright is installed
                Ok(())
            }
            _ => {
                // Try to install playwright
                let install = Command::new("npx")
                    .args(["playwright", "install", "chromium"])
                    .output()
                    .map_err(|e| {
                        AppError::Internal(format!("Failed to install Playwright: {}", e))
                    })?;

                if !install.status.success() {
                    let stderr = String::from_utf8_lossy(&install.stderr);
                    return Err(AppError::Internal(format!(
                        "Failed to install Playwright chromium: {}",
                        stderr
                    )));
                }

                Ok(())
            }
        }
    }

    /// Capture a URL and return tile paths
    pub async fn capture_url(
        &self,
        url: &str,
        output_dir: &Path,
        progress_tx: Option<mpsc::Sender<CaptureProgress>>,
    ) -> Result<CaptureManifest> {
        // Verify script exists
        if !self.config.script_path.exists() {
            return Err(AppError::Internal(format!(
                "Playwright capture script not found at: {}",
                self.config.script_path.display()
            )));
        }

        // Ensure output directory exists
        std::fs::create_dir_all(output_dir).map_err(|e| {
            AppError::Internal(format!("Failed to create output directory: {}", e))
        })?;

        // Run the capture script
        let output = Command::new("node")
            .arg(&self.config.script_path)
            .arg(url)
            .arg(output_dir)
            .output()
            .map_err(|e| AppError::Internal(format!("Failed to run playwright capture: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Parse progress messages and send them
        if let Some(tx) = progress_tx {
            for line in stdout.lines() {
                if line.starts_with('{') && !line.contains("---RESULT---") {
                    if let Ok(progress) = serde_json::from_str::<CaptureProgress>(line) {
                        let _ = tx.send(progress).await;
                    }
                }
            }
        }

        if !output.status.success() {
            // Try to extract error from output
            let error_msg = if !stderr.is_empty() {
                stderr.to_string()
            } else {
                stdout.to_string()
            };
            return Err(AppError::Internal(format!(
                "Playwright capture failed: {}",
                error_msg
            )));
        }

        // Find the result JSON after ---RESULT--- marker
        let result_json = stdout
            .split("---RESULT---")
            .nth(1)
            .map(|s| s.trim())
            .ok_or_else(|| {
                AppError::Internal("No result JSON found in playwright output".to_string())
            })?;

        let manifest: CaptureManifest = serde_json::from_str(result_json).map_err(|e| {
            AppError::Internal(format!("Failed to parse capture manifest: {}", e))
        })?;

        Ok(manifest)
    }

    /// Get full paths to all captured tiles
    pub fn get_tile_paths(&self, output_dir: &Path, manifest: &CaptureManifest) -> Vec<PathBuf> {
        manifest
            .tiles
            .iter()
            .map(|tile| output_dir.join(&tile.path))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PlaywrightConfig::default();
        assert_eq!(config.viewport_width, 1280);
        assert_eq!(config.viewport_height, 2000);
        assert_eq!(config.device_scale, 2.0);
    }
}
