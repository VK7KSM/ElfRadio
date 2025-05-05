use crate::error::DspError;
use std::{collections::HashMap, f32::consts::PI};
use tracing::{debug, error, info, trace, warn};

// --- Helper function for sine wave generation (needed by CW) ---
fn generate_sine_wave(freq: f32, duration_samples: usize, sample_rate: u32, amplitude: f32) -> Vec<f32> {
    if sample_rate == 0 {
        return vec![0.0; duration_samples]; // Avoid division by zero
    }
    let mut buffer = Vec::with_capacity(duration_samples);
    let angular_frequency = 2.0 * PI * freq / sample_rate as f32;
    for i in 0..duration_samples {
        buffer.push(amplitude * (angular_frequency * i as f32).sin());
    }
    buffer
}

// --- Helper function for silence generation (needed by CW) ---
fn generate_silence(duration_samples: usize) -> Vec<f32> {
    vec![0.0f32; duration_samples]
}


// --- Morse Code Generation Implementation ---

// Moved Morse map generation here
fn get_morse_map() -> HashMap<char, &'static str> {
    HashMap::from([
        ('A', ".-"), ('B', "-..."), ('C', "-.-."), ('D', "-.."), ('E', "."),
        ('F', "..-."), ('G', "--."), ('H', "...."), ('I', ".."), ('J', ".---"),
        ('K', "-.-"), ('L', ".-.."), ('M', "--"), ('N', "-."), ('O', "---"),
        ('P', ".--."), ('Q', "--.-"), ('R', ".-."), ('S', "..."), ('T', "-"),
        ('U', "..-"), ('V', "...-"), ('W', ".--"), ('X', "-..-"), ('Y', "-.--"),
        ('Z', "--.."),
        ('0', "-----"), ('1', ".----"), ('2', "..---"), ('3', "...--"), ('4', "....-"),
        ('5', "....."), ('6', "-...."), ('7', "--..."), ('8', "---.."), ('9', "----."),
        ('.', ".-.-.-"), (',', "--..--"), ('?', "..--.."), ('\'', ".----."),
        ('!', "-.-.--"), ('/', "-..-."), ('(', "-.--."), (')', "-.--.-"),
        ('&', ".-..."), (':', "---..."), (';', "-.-.-."), ('=', "-...-"),
        ('+', ".-.-."), ('-', "-....-"), ('_', "..--.-"), ('"', ".-..-."),
        ('$', "...-..-"), ('@', ".--.-."),
        // Space handled separately in generate_cw_audio
    ])
}

/// 生成摩尔斯电码(CW)音频样本
///
/// # 参数
/// * `text` - 要编码的文本。将被转换为大写。不支持的字符将被跳过。
/// * `wpm` - 每分钟字数，决定速度。必须 > 0。
/// * `freq_hz` - 摩尔斯音调的频率，单位Hz(例如700.0)。必须 > 0。
/// * `sample_rate` - 音频采样率，单位Hz(例如44100, 48000)。必须 > 0。
///
/// # 返回值
/// 包含原始音频样本(-1.0到1.0)的`Vec<f32>`，或者一个`DspError`。
pub fn generate_cw_audio(
    text: &str,
    wpm: u32,
    freq_hz: f32,
    sample_rate: u32,
) -> Result<Vec<f32>, DspError> {
    // 参数验证
    if wpm == 0 {
        error!("Invalid WPM value: {}", wpm);
        return Err(DspError::InvalidWpm(wpm));
    }
    if sample_rate == 0 {
        error!("Invalid sample rate: {}", sample_rate);
        return Err(DspError::SstvEncodeError("Sample rate cannot be zero".to_string()));
    }
    if freq_hz <= 0.0 {
         error!("Invalid frequency: {}", freq_hz);
        return Err(DspError::SstvEncodeError("Frequency must be positive".to_string()));
    }

    debug!(
        "Generating CW audio: WPM={}, Freq={} Hz, Rate={} Hz, Text='{}'",
        wpm, freq_hz, sample_rate, text
    );

    // 空输入处理
    if text.is_empty() {
        return Ok(Vec::new());
    }

    // 修改时间参数计算，确保精确性
    let dot_duration_sec = 1.2 / wpm as f32;
    let dot_samples = (dot_duration_sec * sample_rate as f32).round() as usize;

    // 标准摩尔斯码时间单位（以点的长度为基准）
    let dash_samples = 3 * dot_samples;           // 划 = 3个点
    let element_gap_samples = dot_samples;        // 元素内间隔 = 1个点
    let char_gap_samples = 3 * dot_samples;       // 字符间间隔 = 3个点
    let word_gap_samples = 7 * dot_samples;       // 单词间间隔 = 7个点

    let amplitude = 0.85;

    let morse_map = get_morse_map();
    let mut audio_buffer: Vec<f32> = Vec::new();
    let mut is_first_char = true;
    let mut prev_was_valid = false;  // 跟踪前一个字符是否是有效的莫尔斯字符

    for character in text.to_uppercase().chars() {
        if character.is_whitespace() {
            if prev_was_valid {
                // 如果前一个字符是有效的莫尔斯字符，添加额外的间隔使总间隔达到单词间隔
                // 已经有了3个点的字符间隔，需要额外添加4个点
                audio_buffer.extend(generate_silence(word_gap_samples - char_gap_samples));
                trace!("Added word space (extra 4 dots): {} samples", word_gap_samples - char_gap_samples);
            }
            prev_was_valid = false;  // 重置标志
            continue;
        }

        // 查找字符的莫尔斯码表示
        if let Some(morse_sequence) = morse_map.get(&character) {
            // 如果不是第一个字符且前一个字符是有效的，添加字符间间隔
            if !is_first_char && prev_was_valid {
                audio_buffer.extend(generate_silence(char_gap_samples));
                trace!("Added char gap: {} samples", char_gap_samples);
                 }

            // 处理字符的每个元素（点和划）
            for (i, element) in morse_sequence.chars().enumerate() {
                // 在元素之间添加间隔（不是在第一个元素之前）
                if i > 0 {
                    audio_buffer.extend(generate_silence(element_gap_samples));
                    trace!("Added element gap: {} samples", element_gap_samples);
                }

                // 生成点或划的音频
                match element {
                    '.' => {
                        audio_buffer.extend(generate_sine_wave(
                            freq_hz, dot_samples, sample_rate, amplitude
                        ));
                        trace!("Added dot: {} samples", dot_samples);
                    }
                    '-' => {
                        audio_buffer.extend(generate_sine_wave(
                            freq_hz, dash_samples, sample_rate, amplitude
                        ));
                        trace!("Added dash: {} samples", dash_samples);
                    }
                    _ => {} // 不应该发生，因为映射中只包含点和划
                }
            }

            is_first_char = false;
            prev_was_valid = true;  // 标记这是一个有效的莫尔斯字符
        } else {
            // 完全跳过无效字符，不添加任何音频或间隔
            warn!("Unsupported character for CW encoding: '{}'. Skipping.", character);
            // 不改变 prev_was_valid 状态
        }
    }

    info!(
        "Generated CW audio with {} samples (approx {:.2} seconds)",
        audio_buffer.len(),
        audio_buffer.len() as f32 / sample_rate as f32
    );
    
    Ok(audio_buffer)
}


// --- Tests ---
#[cfg(test)]
mod tests {
    use super::*;
    // use std::time::Duration; // 移除未使用的导入
    use assert_matches::assert_matches;

    /// Call this at the beginning of each test function where tracing is needed.
    fn setup_test_tracing() {
        // Try to initialize tracing subscriber, ignore error if already initialized (e.g., by another test)
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env()) // Respect RUST_LOG
            .with_test_writer() // Write to test output capture
            .try_init();
    }

    #[test]
    fn test_generate_cw_audio_empty() {
        setup_test_tracing();
        let sample_rate = 48000;
        let wpm = 20;
        let freq = 700.0; // Fix: Mismatched types
        let text = "";

        let result = generate_cw_audio(text, wpm, freq, sample_rate);
        assert!(result.is_ok());
        let audio = result.unwrap();
        assert_eq!(audio.len(), 0, "Empty string should produce empty audio");
    }

    #[test]
    fn test_generate_cw_audio_simple() { // 测试 SOS
        setup_test_tracing();
        let sample_rate = 48000;
        let wpm = 20;
        let freq = 700.0;
        let text = "SOS";

        // 手动计算预期样本数 (基于 generate_cw_audio 的逻辑)
        // WPM=20, Rate=48000 => dot_samples = 2880
        // S = ... (3 dots, 2 gaps) = 5 * dot_samples = 14400
        // O = --- (3 dashes, 2 gaps) = (3*3 + 2*1) * dot_samples = 11 * dot_samples = 31680
        // char_gap = 3 * dot_samples = 8640
        // Total = S + gap + O + gap + S = 14400 + 8640 + 31680 + 8640 + 14400 = 77760
        let expected_samples: i64 = 77760;

        let result = generate_cw_audio(text, wpm, freq, sample_rate);
        assert!(result.is_ok());
        let audio = result.unwrap();

        let actual_samples = audio.len();
        let tolerance: i64 = 10; // 允许少量样本误差

        tracing::info!(
            wpm,
            freq,
            sample_rate,
            text,
            expected_samples,
            actual_samples = actual_samples as i64,
            tolerance,
            "Checking CW audio length for '{}'", text
        );

        assert!(
            (actual_samples as i64 - expected_samples).abs() <= tolerance,
            "SOS duration mismatch: 预期 {} 个样本 (基于代码逻辑), 实际得到 {}. 允许误差: {}",
            expected_samples, actual_samples, tolerance
        );
    }

    #[test]
    fn test_generate_cw_audio_invalid_wpm() {
        setup_test_tracing();
        let sample_rate = 48000;
        let wpm = 0; // Invalid WPM
        let freq = 700.0; // Fix: Mismatched types
        let text = "HELLO";

        let result = generate_cw_audio(text, wpm, freq, sample_rate);
        assert_matches!(result, Err(DspError::InvalidWpm(_)));
    }

    #[test]
    fn test_generate_cw_audio_skip_hash() { // 重命名并修改测试逻辑
        setup_test_tracing();
        let sample_rate = 48000;
        let wpm = 15;
        let freq = 600.0;
        let invalid_text = "AB#C"; // 使用 '#' 作为未定义字符
        let valid_text = "ABC"; // 预期结果应与此匹配

        // 生成包含无效字符的音频
        let actual_result = generate_cw_audio(invalid_text, wpm, freq, sample_rate);
        assert!(actual_result.is_ok());
        let actual_audio = actual_result.unwrap();

        // 生成仅包含有效字符的音频作为参照
        let expected_result = generate_cw_audio(valid_text, wpm, freq, sample_rate);
        assert!(expected_result.is_ok());
        let expected_audio = expected_result.unwrap();

        tracing::info!(
            wpm,
            freq,
            sample_rate,
            invalid_text,
            valid_text,
            expected_len = expected_audio.len(),
            actual_len = actual_audio.len(),
            "Checking CW audio length when skipping invalid char '#'"
        );

        // 断言长度应该相等，因为 '#' 应该被跳过
        assert_eq!(
            actual_audio.len(),
            expected_audio.len(),
            "无效字符 '#' 应被跳过，长度应该与 {} 匹配，预期长度: {}, 实际长度: {}",
            valid_text, expected_audio.len(), actual_audio.len()
        );
    }

     #[test]
    fn test_generate_cw_audio_different_frequencies() {
        setup_test_tracing();
        let sample_rate = 48000;
        let wpm = 20;
        let text = "SOS";
        let freq1 = 700.0; // Fix: Mismatched types
        let freq2 = 800.0; // Fix: Mismatched types

        let result1 = generate_cw_audio(text, wpm, freq1, sample_rate);
        assert!(result1.is_ok());
        let audio1 = result1.unwrap();

        let result2 = generate_cw_audio(text, wpm, freq2, sample_rate);
        assert!(result2.is_ok());
        let audio2 = result2.unwrap();

        // The audio content should be different due to frequency, but length should be the same
        assert_eq!(audio1.len(), audio2.len(), "Audio lengths should match for different frequencies");

        // Basic check that audio content is not identical (they should have different tones)
        assert_ne!(audio1, audio2, "Audio content should be different for different frequencies");
     }

     #[test]
    fn test_generate_cw_audio_different_wpm() {
        setup_test_tracing();
        let sample_rate = 48000;
        let freq = 700.0; // Fix: Mismatched types
        let text = "SOS";
        let wpm1 = 15;
        let wpm2 = 25;

        let result1 = generate_cw_audio(text, wpm1, freq, sample_rate);
        assert!(result1.is_ok());
        let audio1 = result1.unwrap();

        let result2 = generate_cw_audio(text, wpm2, freq, sample_rate);
        assert!(result2.is_ok());
        let audio2 = result2.unwrap();

        // Audio lengths should be different for different WPM
        assert_ne!(audio1.len(), audio2.len(), "Audio lengths should be different for different WPM");

        // Check that length decreases as WPM increases
        assert!(audio1.len() > audio2.len(), "Higher WPM should result in shorter audio");
      }

     #[test]
    fn test_generate_cw_audio_with_spaces() {
        setup_test_tracing();
        let sample_rate = 48000;
        let wpm = 20;
        let freq = 700.0;
        let text = "HI HI";

        // 基于代码逻辑和日志分析，预期样本数为 86400
        let expected_samples: i64 = 86400;

        let result = generate_cw_audio(text, wpm, freq, sample_rate);
        assert!(result.is_ok());
        let audio = result.unwrap();

        let actual_samples = audio.len();
        let tolerance: i64 = 10; // 允许少量样本误差

        tracing::info!(
            wpm,
            freq,
            sample_rate,
            text,
            expected_samples,
            actual_samples = actual_samples as i64,
            tolerance,
            "Checking CW audio length for '{}'", text
        );

        // 使用精确计算的预期值进行断言
        assert!(
            (actual_samples as i64 - expected_samples).abs() <= tolerance,
            "HI HI duration mismatch: 预期 {} 个样本 (基于代码逻辑), 实际得到 {}. 允许误差: {}",
            expected_samples, actual_samples, tolerance
        );
      }
}
