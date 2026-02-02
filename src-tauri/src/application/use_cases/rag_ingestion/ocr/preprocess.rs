use image::{GrayImage, ImageBuffer, Luma};
use std::path::{Path, PathBuf};

use super::RagIngestionUseCase;

impl RagIngestionUseCase {
    /// Preprocess an image for better OCR results.
    /// Pipeline: grayscale -> contrast -> threshold.
    pub(super) fn preprocess_image_for_ocr(&self, image_path: &Path) -> Option<PathBuf> {
        let img = image::open(image_path).ok()?;

        let gray = img.to_luma8();
        let enhanced = self.enhance_contrast(&gray);
        let thresholded = self.adaptive_threshold(&enhanced);

        let preprocessed_path = image_path.with_extension("preprocessed.png");
        thresholded.save(&preprocessed_path).ok()?;
        Some(preprocessed_path)
    }

    pub(super) fn preprocess_image_for_mode(
        &self,
        image_path: &Path,
        mode: &str,
    ) -> Option<PathBuf> {
        let img = image::open(image_path).ok()?;

        let gray = img.to_luma8();
        let processed = match mode {
            "grayscale" => gray,
            "contrast" => self.enhance_contrast(&gray),
            "otsu" => self.adaptive_threshold(&self.enhance_contrast(&gray)),
            _ => self.adaptive_threshold(&self.enhance_contrast(&gray)),
        };

        let preprocessed_path = image_path.with_extension("preprocessed.png");
        processed.save(&preprocessed_path).ok()?;
        Some(preprocessed_path)
    }

    /// Enhance contrast using histogram stretching.
    pub(super) fn enhance_contrast(&self, img: &GrayImage) -> GrayImage {
        let (width, height) = img.dimensions();
        let mut result = ImageBuffer::new(width, height);

        let (min_val, max_val) = img.pixels().fold((255u8, 0u8), |(min_v, max_v), pixel| {
            let v = pixel[0];
            (min_v.min(v), max_v.max(v))
        });

        // Avoid division by zero.
        let range = if max_val > min_val {
            (max_val - min_val) as f32
        } else {
            1.0
        };

        for (x, y, pixel) in img.enumerate_pixels() {
            let val = pixel[0];
            let stretched = ((val as f32 - min_val as f32) / range * 255.0) as u8;
            result.put_pixel(x, y, Luma([stretched]));
        }

        result
    }

    /// Apply thresholding using a simplified Otsu method.
    pub(super) fn adaptive_threshold(&self, img: &GrayImage) -> GrayImage {
        let (width, height) = img.dimensions();
        let mut result = ImageBuffer::new(width, height);

        let mut histogram = [0u32; 256];
        for pixel in img.pixels() {
            histogram[pixel[0] as usize] += 1;
        }

        let total_pixels = (width * height) as f64;
        let sum_total: f64 = histogram
            .iter()
            .enumerate()
            .map(|(i, &count)| i as f64 * count as f64)
            .sum();

        let mut sum_b = 0.0;
        let mut weight_b = 0.0;
        let mut max_variance = 0.0;
        let mut threshold = 128u8;

        for (i, &count) in histogram.iter().enumerate() {
            weight_b += count as f64;
            if weight_b == 0.0 {
                continue;
            }

            let weight_f = total_pixels - weight_b;
            if weight_f == 0.0 {
                break;
            }

            sum_b += i as f64 * count as f64;
            let mean_b = sum_b / weight_b;
            let mean_f = (sum_total - sum_b) / weight_f;

            let variance = weight_b * weight_f * (mean_b - mean_f).powi(2);
            if variance > max_variance {
                max_variance = variance;
                threshold = i as u8;
            }
        }

        for (x, y, pixel) in img.enumerate_pixels() {
            let val = if pixel[0] > threshold { 255 } else { 0 };
            result.put_pixel(x, y, Luma([val]));
        }

        result
    }

    /// Heuristic: low std-dev ~= low contrast.
    pub(super) fn needs_preprocessing(&self, image_path: &Path) -> bool {
        let img = match image::open(image_path) {
            Ok(img) => img,
            Err(_) => return false,
        };

        let gray = img.to_luma8();
        let pixels: Vec<f64> = gray.pixels().map(|p| p[0] as f64).collect();
        if pixels.is_empty() {
            return false;
        }

        let mean = pixels.iter().sum::<f64>() / pixels.len() as f64;
        let variance =
            pixels.iter().map(|&p| (p - mean).powi(2)).sum::<f64>() / pixels.len() as f64;
        let std_dev = variance.sqrt();

        std_dev < 50.0
    }
}
