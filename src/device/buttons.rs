/// Button labels for the 10 LCD buttons
pub const BUTTON_LABELS: [&str; 10] = [
    "ACCEPT", // 0 - Top row
    "REJECT", // 1
    "STOP",   // 2
    "RETRY",  // 3
    "REWIND", // 4 - moved from bottom row
    "TRUST",  // 5 - Bottom row - trust/allow tool
    "TAB",    // 6
    "MIC",    // 7
    "ENTER",  // 8 - moved next to MIC
    "CLEAR",  // 9
];

/// Button icons (file names in assets/icons/)
pub const BUTTON_ICONS: [&str; 10] = [
    "accept.png",
    "reject.png",
    "stop.png",
    "retry.png",
    "rewind.png",
    "trust.png",
    "tab.png",
    "mic.png",
    "enter.png",
    "clear.png",
];

/// Encoder labels
pub const ENCODER_LABELS: [&str; 4] = [
    "History", // 0
    "Scroll",  // 1
    "Zoom",    // 2
    "Model",   // 3
];

/// Button descriptions for tooltips/help
pub const BUTTON_DESCRIPTIONS: [&str; 10] = [
    "Send 'y' + Enter to accept",
    "Send 'n' + Enter to reject",
    "Send Escape to stop/interrupt",
    "Send Up + Enter to retry last",
    "Double Escape to browse history",
    "Shift+Tab for allow all this session",
    "Tab for autocomplete (long: new session)",
    "Trigger OS voice input (long: clear line)",
    "Send Enter",
    "Send /clear to reset conversation",
];
