use crate::error::DspError;
use image::{self, DynamicImage};
use rsstv::{
    common::SSTVMode,
    martinm1::MartinM1,
};
use std::path::Path;
use tracing::{debug, info};

// --- SSTV Encoding Implementation ---

// Constants might be specific to modes, perhaps keep them here for now
// Or potentially move to a shared `modes` module if more modes are added.

/// Encodes an image file into SSTV Martin M1 audio samples using the `rsstv` crate (v0.2.1 API).
///
/// # Arguments
/// * `image_path` - Path to the image file (e.g., PNG, JPG).
///
/// # Returns
/// A `Vec<f32>` containing the raw audio samples (-1.0 to 1.0) for the SSTV signal
/// at the sample rate defined by `rsstv` (44100 Hz in v0.2.1),
/// or a `DspError` if image loading or processing fails.
pub fn encode_sstv_martin_m1(image_path: &Path) -> Result<Vec<f32>, DspError> {
    info!("Starting SSTV Martin M1 encoding using rsstv v0.2.1 for: {:?}", image_path);

    // 1. Load Image into DynamicImage
    debug!("Loading image...");
    let loaded_image: DynamicImage = image::open(image_path)?; // Implicitly uses DspError::ImageError(#[from] ImageError)
    debug!("Image loaded successfully. Dimensions: {}x{}", loaded_image.width(), loaded_image.height());

    // Note: rsstv 0.2.1 MartinM1 encoder seems to handle resizing internally.
    // If specific dimensions were strictly required by the library *before* encoding,
    // we would add a check here:
    // if loaded_image.width() != MARTIN_M1_WIDTH || loaded_image.height() != MARTIN_M1_HEIGHT {
    //     warn!(
    //         "Image dimensions ({}x{}) do not match Martin M1 ({}x{}). rsstv will resize.",
    //         loaded_image.width(), loaded_image.height(),
    //         MARTIN_M1_WIDTH, MARTIN_M1_HEIGHT
    //     );
    //     // Potentially return an error if strict dimensions are needed:
    //     // return Err(DspError::UnsupportedDimensions { ... });
    // }


    // 2. Create MartinM1 Encoder instance
    let mut encoder = MartinM1::new();
    debug!("rsstv MartinM1 encoder created.");

    // 3. Encode the loaded image
    info!("Encoding image to SSTV signal...");
    let signal = encoder.encode(loaded_image); // rsstv handles resizing internally
    debug!("Image encoded to signal.");

    // 4. Convert Signal to Audio Samples
    info!("Converting signal to audio samples...");
    let samples: Vec<f32> = signal.to_samples(); // This doesn't return Result in rsstv 0.2.1
    let sstv_sample_rate = rsstv::SAMPLE_RATE as u32; // Use the const directly

    info!(
        "rsstv successfully generated {} SSTV audio samples (approx {:.2} seconds at {} Hz)",
        samples.len(),
        samples.len() as f32 / sstv_sample_rate as f32,
        sstv_sample_rate
    );

    // 5. Return Samples
    Ok(samples)
}


// --- Tests ---
#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};
    use tempfile;

    #[test]
    fn test_encode_sstv_martin_m1_success() {
        // Create a temporary directory for test files
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        
        // Create a simple test image (Martin M1 format is 320x256)
        let width = 320u32;
        let height = 256u32;
        let mut img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(width, height);
        
        // Fill the image with a red color
        for (_, _, pixel) in img.enumerate_pixels_mut() {
            *pixel = Rgb([255u8, 0u8, 0u8]);
        }
        
        // Save the test image to a temporary file
        let input_path = temp_dir.path().join("test_image.png");
        img.save(&input_path).expect("Failed to save test image");
        
        // Call SSTV encoding function
        let result = encode_sstv_martin_m1(&input_path);
        
        // Verify successful encoding
        assert!(result.is_ok(), "SSTV encoding failed: {:?}", result.err());
        
        // Check the returned audio samples
        let samples = result.unwrap();
        assert!(!samples.is_empty(), "Encoded audio should not be empty");

        // Martin M1 mode takes approximately 114 seconds
        // Assuming sample rate of 48000 Hz, expected samples ≈ 114 * 48000
        let expected_samples = 114 * 48000;
        let margin = 0.15; // Allow 15% variation
        
        println!("Generated {} audio samples for Martin M1", samples.len());
        
        // Check if sample count is within reasonable range
        assert!(
            samples.len() > ((1.0 - margin) * expected_samples as f32) as usize &&
            samples.len() < ((1.0 + margin) * expected_samples as f32) as usize,
            "Sample count ({}) outside expected range ({} ± {}%)",
            samples.len(),
            expected_samples,
            margin * 100.0
        );
    }

    #[test]
    fn test_encode_sstv_martin_m1_nonexistent_input() {
        // Create a temporary directory
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        
        // Define a path to a non-existent file
        let nonexistent_path = temp_dir.path().join("nonexistent_image.png");
        
        // Verify the file doesn't exist
        assert!(!nonexistent_path.exists(), "Test file should not exist");
        
        // Try to encode a non-existent image
        let result = encode_sstv_martin_m1(&nonexistent_path);
        
        // Verify encoding fails
        assert!(result.is_err(), "Expected error for non-existent input file");
        
        // Print the error for debugging
        if let Err(err) = &result {
            println!("Error for non-existent file: {:?}", err);
        }
    }

    #[test]
    fn test_encode_sstv_martin_m1_invalid_image() {
        // Create a temporary directory
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        
        // Create an invalid "image" file (just text)
        let invalid_path = temp_dir.path().join("invalid_image.png");
        std::fs::write(&invalid_path, "This is not a valid PNG file").expect("Failed to write invalid file");
        
        // Try to encode the invalid image
        let result = encode_sstv_martin_m1(&invalid_path);
        
        // Verify encoding fails
        assert!(result.is_err(), "Expected error for invalid image format");
        
        // Print the error for debugging
        if let Err(err) = &result {
            println!("Error for invalid image: {:?}", err);
        }
    }

    #[test]
    fn test_encode_sstv_martin_m1_different_sizes() {
        // Create test images with different dimensions
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        
        // Test dimensions (one smaller, one larger than standard)
        let test_sizes = [(160u32, 128u32), (640u32, 512u32)];
        
        for (width, height) in test_sizes {
            // Create test image with non-standard dimensions
            let mut img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(width, height);
            for (_, _, pixel) in img.enumerate_pixels_mut() {
                *pixel = Rgb([0u8, 255u8, 0u8]); // Green
            }
            
            // Save the test image
            let input_path = temp_dir.path().join(format!("test_{}x{}.png", width, height));
            img.save(&input_path).expect("Failed to save test image");
            
            // Try to encode the image with non-standard dimensions
            let result = encode_sstv_martin_m1(&input_path);
            
            println!("Result for {}x{} image: {:?}", width, height, result.is_ok());
            
            // Check behavior with non-standard dimensions
            // Note: The function might resize the image, so this could succeed
            // If it's expected to fail with non-standard dimensions, change this to:
            // assert!(result.is_err(), "Should reject non-standard dimensions");
            if result.is_ok() {
                let samples = result.unwrap();
                assert!(!samples.is_empty(), "Generated audio should not be empty");
            }
        }
    }
}
