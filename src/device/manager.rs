use anyhow::{anyhow, Result};
use image::{DynamicImage, RgbImage};
use mirajazz::{
    device::{list_devices, Device},
    types::{DeviceInput, ImageFormat, ImageMirroring, ImageMode, ImageRotation},
};
use std::time::Duration;
use tracing::{debug, info, warn};

use super::protocol::*;

/// Input events from the device
#[derive(Debug, Clone)]
pub enum InputEvent {
    ButtonDown(u8),
    ButtonUp(u8),
    EncoderRotate { encoder: u8, direction: i8 },
    EncoderPress(u8),
    EncoderRelease(u8),
}

/// Device information
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub firmware_version: String,
    pub serial_number: String,
}

/// Previous button/encoder states for edge detection
struct InputState {
    buttons: Vec<bool>,
    encoders: Vec<bool>,
}

impl InputState {
    fn new(button_count: usize, encoder_count: usize) -> Self {
        Self {
            buttons: vec![false; button_count],
            encoders: vec![false; encoder_count],
        }
    }
}

/// Manages connection to the AJAZZ AKP05E / Mirabox N4
pub struct DeviceManager {
    device: Device,
    input_state: InputState,
}

impl DeviceManager {
    /// Find and return device info without connecting
    pub async fn find_device() -> Result<DeviceInfo> {
        let devices = list_devices(&[VENDOR_ID])
            .await
            .map_err(|e| anyhow!("Failed to enumerate devices: {}", e))?;

        for (vid, pid, serial) in devices {
            if vid == VENDOR_ID && pid == PRODUCT_ID {
                return Ok(DeviceInfo {
                    name: "AJAZZ AKP05E".to_string(),
                    firmware_version: "Unknown".to_string(),
                    serial_number: serial,
                });
            }
        }

        Err(anyhow!("No compatible device found"))
    }

    /// Connect to the device
    pub async fn connect() -> Result<Self> {
        info!("Connecting to device...");

        // First, find the device serial
        let devices = list_devices(&[VENDOR_ID])
            .await
            .map_err(|e| anyhow!("Failed to enumerate devices: {}", e))?;

        let serial = devices
            .iter()
            .find(|(vid, pid, _)| *vid == VENDOR_ID && *pid == PRODUCT_ID)
            .map(|(_, _, s)| s.clone())
            .ok_or_else(|| anyhow!("No compatible device found"))?;

        info!("Found device with serial: {}", serial);

        // Connect to the device
        // N4/AKP05E: v2 protocol, supports both states, 10 keys, 4 encoders
        let device = Device::connect(
            VENDOR_ID,
            PRODUCT_ID,
            serial,
            true, // is_v2 (1024-byte packets)
            true, // supports_both_states
            BUTTON_COUNT as usize,
            ENCODER_COUNT as usize,
        )
        .await
        .map_err(|e| anyhow!("Failed to connect to device: {}", e))?;

        info!("Connected to device");

        let input_state = InputState::new(BUTTON_COUNT as usize, ENCODER_COUNT as usize);

        Ok(Self {
            device,
            input_state,
        })
    }

    /// Get image format for square buttons (112x112 JPEG)
    fn button_image_format() -> ImageFormat {
        ImageFormat {
            mode: ImageMode::JPEG,
            size: (BUTTON_WIDTH as usize, BUTTON_HEIGHT as usize),
            rotation: ImageRotation::Rot180,
            mirror: ImageMirroring::None,
        }
    }

    /// Get image format for LCD strip soft buttons (112x112 JPEG)
    fn strip_button_image_format() -> ImageFormat {
        ImageFormat {
            mode: ImageMode::JPEG,
            size: (STRIP_BUTTON_WIDTH as usize, STRIP_BUTTON_HEIGHT as usize),
            rotation: ImageRotation::Rot180,
            mirror: ImageMirroring::None,
        }
    }

    /// Set button image (112x112 RGB) - takes ownership to avoid clone
    pub async fn set_button_image(&self, button: u8, image: RgbImage) -> Result<()> {
        if button >= BUTTON_COUNT {
            return Err(anyhow!("Invalid button index: {}", button));
        }

        // Convert RgbImage to DynamicImage (no clone needed since we own the image)
        let dynamic_image = DynamicImage::ImageRgb8(image);

        self.device
            .set_button_image(button, Self::button_image_format(), dynamic_image)
            .await
            .map_err(|e| anyhow!("Failed to set button image: {}", e))?;

        Ok(())
    }

    /// Set LCD strip soft button image (112x112 RGB) - legacy individual button mode
    /// Strip buttons use display indices 0-3
    pub async fn set_strip_button_image(&self, button: u8, image: &RgbImage) -> Result<()> {
        if button >= STRIP_BUTTON_COUNT {
            return Err(anyhow!("Invalid strip button index: {}", button));
        }

        // Display indices for strip are 0-3
        let display_key = button;
        debug!(
            "Setting image for strip button {} (display key {})",
            button, display_key
        );

        // Convert RgbImage to DynamicImage
        let dynamic_image = DynamicImage::ImageRgb8(image.clone());

        self.device
            .set_button_image(
                display_key,
                Self::strip_button_image_format(),
                dynamic_image,
            )
            .await
            .map_err(|e| anyhow!("Failed to set strip button image: {}", e))?;

        Ok(())
    }

    /// Get image format for full LCD strip (800x128 JPEG)
    fn full_strip_image_format() -> ImageFormat {
        ImageFormat {
            mode: ImageMode::JPEG,
            size: (STRIP_WIDTH as usize, STRIP_HEIGHT as usize),
            rotation: ImageRotation::Rot180,
            mirror: ImageMirroring::None,
        }
    }

    /// Set full LCD strip image (800x128 RGB) - continuous display mode
    /// Sends a single wide image that fills the entire strip without gaps
    pub async fn set_strip_image(&self, image: RgbImage) -> Result<()> {
        debug!("Setting full strip image ({}x{})", image.width(), image.height());

        let dynamic_image = DynamicImage::ImageRgb8(image);

        self.device
            .set_button_image(0, Self::full_strip_image_format(), dynamic_image)
            .await
            .map_err(|e| anyhow!("Failed to set strip image: {}", e))?;

        Ok(())
    }

    /// Flush pending image updates to the device
    pub async fn flush(&self) -> Result<()> {
        self.device
            .flush()
            .await
            .map_err(|e| anyhow!("Failed to flush images: {}", e))
    }

    /// Reset the device (clear display and set brightness)
    pub async fn reset(&self) -> Result<()> {
        debug!("Resetting device");
        self.device
            .reset()
            .await
            .map_err(|e| anyhow!("Failed to reset device: {}", e))
    }

    /// Send keep-alive to prevent device timeout
    pub async fn keep_alive(&self) -> Result<()> {
        self.device
            .keep_alive()
            .await
            .map_err(|e| anyhow!("Failed to send keep-alive: {}", e))
    }

    /// Set device brightness (0-100)
    pub async fn set_brightness(&self, percent: u8) -> Result<()> {
        let percent = percent.min(100);
        debug!("Setting brightness to {}%", percent);
        self.device
            .set_brightness(percent)
            .await
            .map_err(|e| anyhow!("Failed to set brightness: {}", e))
    }

    /// Input processing function for mirajazz
    ///
    /// For N4/AKP05E:
    /// - event_type (data[9]): Action identifier
    ///   - 0x01-0x05: Top row buttons (logical 0-4)
    ///   - 0x06-0x0a: Bottom row buttons (logical 5-9)
    ///   - 0x33, 0x35, 0x36, 0x37: Encoder presses (encoders 0-3)
    ///   - 0x40-0x43: LCD strip soft buttons (0-3)
    ///   - 0x50, 0x51: LCD strip swipe left/right
    ///   - 0x70-0x73: Encoder rotate counter-clockwise
    ///   - 0xa0-0xa3: Encoder rotate clockwise
    /// - state (data[10]): 0x00 = release, non-zero = press (for buttons)
    fn process_input(
        event_type: u8,
        state: u8,
    ) -> Result<DeviceInput, mirajazz::error::MirajazzError> {
        debug!("HID: type=0x{:02x}, state=0x{:02x}", event_type, state);

        match event_type {
            // Main buttons (IDs 1-10 → logical 0-9)
            0x01..=0x0a => {
                let mut buttons = vec![false; BUTTON_COUNT as usize];
                let button_idx = (event_type - 1) as usize;
                if button_idx < buttons.len() {
                    buttons[button_idx] = state != 0;
                }
                debug!(
                    "Button {} {}",
                    button_idx,
                    if state != 0 { "pressed" } else { "released" }
                );
                Ok(DeviceInput::ButtonStateChange(buttons))
            }

            // Encoder presses (actual IDs: 0x33, 0x35, 0x36, 0x37)
            // Mapping based on physical wheel position (left to right: 0, 1, 2, 3)
            0x33 | 0x35 | 0x36 | 0x37 => {
                let mut encoders = vec![false; ENCODER_COUNT as usize];
                let encoder_idx = match event_type {
                    0x37 => 0, // Wheel 1 (leftmost)
                    0x35 => 1, // Wheel 2 (model)
                    0x33 => 2, // Wheel 3
                    0x36 => 3, // Wheel 4 (rightmost)
                    _ => 0,
                };
                if encoder_idx < encoders.len() {
                    encoders[encoder_idx] = state != 0; // Use state param for press/release
                }
                let action = if state != 0 { "pressed" } else { "released" };
                debug!(
                    "Encoder press raw: idx={}, action={}, value={}",
                    encoder_idx, action, encoders[encoder_idx]
                );
                Ok(DeviceInput::EncoderStateChange(encoders))
            }

            // Encoder 3 rotation (rightmost knob)
            // Pattern: 0x70 = CCW, 0x71 = CW
            0x70 | 0x71 => {
                let mut directions = vec![0i8; ENCODER_COUNT as usize];
                let dir = if event_type & 1 == 1 { 1 } else { -1 };
                directions[3] = dir;
                Ok(DeviceInput::EncoderTwist(directions))
            }

            // Encoder 0 rotation (leftmost knob)
            // Pattern: 0xa0 = CCW, 0xa1 = CW
            0xa0 | 0xa1 => {
                let mut directions = vec![0i8; ENCODER_COUNT as usize];
                let dir = if event_type & 1 == 1 { 1 } else { -1 };
                directions[0] = dir;
                Ok(DeviceInput::EncoderTwist(directions))
            }

            // Knob 3 rotation (0x90 CCW, 0x91 CW)
            0x90 | 0x91 => {
                let mut directions = vec![0i8; ENCODER_COUNT as usize];
                directions[2] = if event_type == 0x91 { 1 } else { -1 };
                Ok(DeviceInput::EncoderTwist(directions))
            }

            // LCD strip soft buttons (IDs 0x40-0x43)
            0x40..=0x43 => {
                let mut buttons = vec![false; BUTTON_COUNT as usize];
                let button_idx = (event_type - 0x40 + 10) as usize;
                if button_idx < buttons.len() {
                    buttons[button_idx] = true;
                }
                debug!("LCD strip button {} pressed", event_type - 0x40);
                Ok(DeviceInput::ButtonStateChange(buttons))
            }

            // Knob 2 rotation (0x50 CCW, 0x51 CW)
            0x50 => {
                let mut directions = vec![0i8; ENCODER_COUNT as usize];
                directions[1] = -1; // Encoder 1
                Ok(DeviceInput::EncoderTwist(directions))
            }
            0x51 => {
                let mut directions = vec![0i8; ENCODER_COUNT as usize];
                directions[1] = 1; // Encoder 1
                Ok(DeviceInput::EncoderTwist(directions))
            }

            // Null/empty events (noise or padding)
            0x00 => Ok(DeviceInput::NoData),

            // Unknown event - log it for discovery
            _ => {
                info!(
                    "Unknown HID event: type=0x{:02x}, state=0x{:02x}",
                    event_type, state
                );
                Ok(DeviceInput::NoData)
            }
        }
    }

    /// Poll for input events (non-blocking, 1ms timeout for responsive animations)
    pub async fn poll_event(&mut self) -> Result<Option<InputEvent>> {
        let timeout = Duration::from_millis(1);

        match self
            .device
            .read_input(Some(timeout), Self::process_input)
            .await
        {
            Ok(input) => {
                match input {
                    DeviceInput::NoData => Ok(None),

                    DeviceInput::ButtonStateChange(states) => {
                        // Detect button press/release edges
                        for (i, &pressed) in states.iter().enumerate() {
                            if i < self.input_state.buttons.len() {
                                let was_pressed = self.input_state.buttons[i];
                                self.input_state.buttons[i] = pressed;

                                if pressed && !was_pressed {
                                    return Ok(Some(InputEvent::ButtonDown(i as u8)));
                                } else if !pressed && was_pressed {
                                    return Ok(Some(InputEvent::ButtonUp(i as u8)));
                                }
                            }
                        }
                        Ok(None)
                    }

                    DeviceInput::EncoderStateChange(states) => {
                        // Detect encoder press/release edges
                        for (i, &pressed) in states.iter().enumerate() {
                            if i < self.input_state.encoders.len() {
                                let was_pressed = self.input_state.encoders[i];
                                debug!(
                                    "Encoder state change: idx={}, was={}, now={}",
                                    i, was_pressed, pressed
                                );

                                if pressed && !was_pressed {
                                    // Press detected - immediately reset to allow next press
                                    // (device doesn't send release events)
                                    self.input_state.encoders[i] = false;
                                    debug!("Encoder {} detected press edge", i);
                                    return Ok(Some(InputEvent::EncoderPress(i as u8)));
                                } else if !pressed && was_pressed {
                                    self.input_state.encoders[i] = pressed;
                                    debug!("Encoder {} detected release edge", i);
                                    return Ok(Some(InputEvent::EncoderRelease(i as u8)));
                                } else {
                                    self.input_state.encoders[i] = pressed;
                                }
                            }
                        }
                        Ok(None)
                    }

                    DeviceInput::EncoderTwist(directions) => {
                        // Find first non-zero encoder rotation
                        for (i, &dir) in directions.iter().enumerate() {
                            if dir != 0 {
                                return Ok(Some(InputEvent::EncoderRotate {
                                    encoder: i as u8,
                                    direction: dir,
                                }));
                            }
                        }
                        Ok(None)
                    }
                }
            }
            Err(e) => {
                // Check if this is a disconnect error
                let error_str = format!("{}", e);
                if error_str.contains("Disconnected") {
                    warn!("Device disconnected");
                    return Err(anyhow!("Device disconnected"));
                }
                warn!("Error reading device input: {}", e);
                Ok(None)
            }
        }
    }

    /// Disconnect from device gracefully
    pub async fn disconnect(self) {
        info!("Disconnecting from device...");
        // Just drop the device to release HID connection
        info!("Device disconnected");
    }
}
