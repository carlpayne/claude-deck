//! N4/AKP05E device constants
//!
//! Display index mapping (for set_button_image):
//!   - Top row (5 buttons):    display keys 10-14
//!   - Bottom row (5 buttons): display keys 5-9
//!   - LCD strip (4 softkeys): display keys 0-3
//!
//! Input mapping (button presses):
//!   - Top row:    IDs 1-5  (0x01-0x05) → logical buttons 0-4
//!   - Bottom row: IDs 6-10 (0x06-0x0a) → logical buttons 5-9
//!   - LCD strip:  IDs 0x40-0x43        → logical softkeys 0-3

// Button image dimensions (N4 uses 112x112 for square LCD buttons)
pub const BUTTON_WIDTH: u32 = 112;
pub const BUTTON_HEIGHT: u32 = 112;

/// LCD strip soft button dimensions (legacy - for individual button mode)
pub const STRIP_BUTTON_WIDTH: u32 = 112;
pub const STRIP_BUTTON_HEIGHT: u32 = 112;

/// Full LCD strip dimensions (for continuous display mode)
/// 800x128 fills the entire strip width without gaps
pub const STRIP_WIDTH: u32 = 800;
pub const STRIP_HEIGHT: u32 = 128;

/// Number of LCD buttons (N4 has 10 square + 4 strip = 14 addressable displays)
pub const BUTTON_COUNT: u8 = 15;

/// Number of LCD strip soft buttons
pub const STRIP_BUTTON_COUNT: u8 = 4;

/// Number of rotary encoders
pub const ENCODER_COUNT: u8 = 4;

/// Protocol version
pub const PROTOCOL_VERSION: u8 = 3;

/// Packet size for v3 protocol
pub const PACKET_SIZE: usize = 1024;

/// USB Vendor ID for AJAZZ/Mirabox (HOTSPOTEKUSB)
pub const VENDOR_ID: u16 = 0x0300;

/// USB Product ID for AKP05E/N4
pub const PRODUCT_ID: u16 = 0x3004;

/// Long press threshold in milliseconds
pub const LONG_PRESS_MS: u64 = 2000;

/// Convert logical button ID (0-9) to device display key
///
/// The N4 display mapping is:
/// - Top row (buttons 0-4) → display keys 10-14
/// - Bottom row (buttons 5-9) → display keys 5-9
#[inline]
pub fn button_to_display_key(button_id: u8) -> u8 {
    if button_id < 5 {
        button_id + 10 // 0-4 → 10-14 (top row)
    } else {
        button_id // 5-9 → 5-9 (bottom row)
    }
}
