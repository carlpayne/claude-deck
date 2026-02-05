// claude-deck Configuration UI

const API_BASE = '/api';

// State
let profiles = [];
let currentProfile = null;
let currentButton = null;
let colorPresets = [];
let availableKeys = [];
let builtinActions = [];  // Available built-in actions for Claude Code
let currentCustomImage = null;  // Base64 data URL for custom image
let installedApps = [];  // List of installed macOS apps
let draggedButton = null;  // Currently dragged button position
let selectedGifUrl = null;  // Currently selected GIF URL

// DOM Elements
const elements = {
    connectionStatus: document.getElementById('connection-status'),
    profileTabs: document.getElementById('profile-tabs'),
    buttonGrid: document.getElementById('button-grid'),
    lcdTask: document.getElementById('lcd-task'),
    lcdDetail: document.getElementById('lcd-detail'),
    lcdModel: document.getElementById('lcd-model'),
    lcdStatus: document.getElementById('lcd-status'),
    editorForm: document.getElementById('editor-form'),
    editorHint: document.querySelector('.editor-hint'),
    colorPresets: document.getElementById('color-presets'),
    editPosition: document.getElementById('edit-position'),
    editLabel: document.getElementById('edit-label'),
    editColor: document.getElementById('edit-color'),
    editBrightColor: document.getElementById('edit-bright-color'),
    editActionType: document.getElementById('edit-action-type'),
    editActionValue: document.getElementById('edit-action-value'),
    editActionKey: document.getElementById('edit-action-key'),
    labelGroup: document.getElementById('label-group'),
    editDisplayType: document.getElementById('edit-display-type'),
    emojiDisplayGroup: document.getElementById('emoji-display-group'),
    imageDisplayGroup: document.getElementById('image-display-group'),
    editEmojiImage: document.getElementById('edit-emoji-image'),
    imageDropZone: document.getElementById('image-drop-zone'),
    imageFileInput: document.getElementById('image-file-input'),
    imagePreview: document.getElementById('image-preview'),
    clearImageBtn: document.getElementById('clear-image'),
    actionValueGroup: document.getElementById('action-value-group'),
    editActionBuiltin: document.getElementById('edit-action-builtin'),
    modifierGroup: document.getElementById('modifier-group'),
    modCmd: document.getElementById('mod-cmd'),
    modCtrl: document.getElementById('mod-ctrl'),
    modAlt: document.getElementById('mod-alt'),
    modShift: document.getElementById('mod-shift'),
    autoSubmitGroup: document.getElementById('auto-submit-group'),
    editAutoSubmit: document.getElementById('edit-auto-submit'),
    micIconHint: document.getElementById('mic-icon-hint'),
    btnCancel: document.getElementById('btn-cancel'),
    btnCopy: document.getElementById('btn-copy'),
    btnReload: document.getElementById('btn-reload'),
    // Profile management elements
    btnReset: document.getElementById('btn-reset'),
    btnDelete: document.getElementById('btn-delete'),
    btnNewProfile: document.getElementById('btn-new-profile'),
    createProfileModal: document.getElementById('create-profile-modal'),
    createProfileForm: document.getElementById('create-profile-form'),
    newProfileApp: document.getElementById('new-profile-app'),
    newProfileName: document.getElementById('new-profile-name'),
    newProfileCopy: document.getElementById('new-profile-copy'),
    btnCancelCreate: document.getElementById('btn-cancel-create'),
    // GIF elements
    gifDisplayGroup: document.getElementById('gif-display-group'),
    gifUrlInput: document.getElementById('gif-url-input'),
    gifSearchInput: document.getElementById('gif-search-input'),
    gifSearchBtn: document.getElementById('gif-search-btn'),
    gifResults: document.getElementById('gif-results'),
    gifPreviewContainer: document.getElementById('gif-preview-container'),
    gifPreview: document.getElementById('gif-preview'),
    clearGifBtn: document.getElementById('clear-gif'),
};

// Initialize
async function init() {
    try {
        await Promise.all([
            loadColors(),
            loadActions(),
            loadProfiles(),
            loadApps(),
        ]);
        setConnected(true);
        setupEventListeners();
        startStatusPolling();
    } catch (error) {
        console.error('Failed to initialize:', error);
        setConnected(false, error.message);
    }
}

// LCD Status Polling
let statusPollInterval = null;

function startStatusPolling() {
    // Poll immediately, then every 500ms
    pollStatus();
    statusPollInterval = setInterval(pollStatus, 500);
}

async function pollStatus() {
    try {
        const status = await api('/status');
        updateLcdDisplay(status);
    } catch (error) {
        // Silently fail - status endpoint might not be available
    }
}

function updateLcdDisplay(status) {
    // Task
    const task = status.task || 'READY';
    elements.lcdTask.textContent = task;
    elements.lcdTask.className = 'lcd-value';
    if (task === 'THINKING') elements.lcdTask.classList.add('thinking');
    else if (task === 'ERROR' || task === 'RATE LIMITED') elements.lcdTask.classList.add('error');
    else if (status.waiting_for_input) elements.lcdTask.classList.add('waiting');

    // Detail
    elements.lcdDetail.textContent = status.tool_detail || '-';

    // Model
    elements.lcdModel.textContent = (status.model || 'unknown').toUpperCase();

    // Status
    let statusText = 'OFFLINE';
    let statusClass = 'offline';
    if (status.waiting_for_input) {
        statusText = 'WAITING FOR INPUT';
        statusClass = 'waiting';
    } else if (status.processing || status.task !== 'READY') {
        statusText = 'CONNECTED';
        statusClass = 'connected';
    } else if (status.timestamp && (Date.now() / 1000 - status.timestamp) < 30) {
        statusText = 'CONNECTED';
        statusClass = 'connected';
    }
    elements.lcdStatus.textContent = statusText;
    elements.lcdStatus.className = 'lcd-value ' + statusClass;
}

// API Functions
async function api(endpoint, options = {}) {
    const url = `${API_BASE}${endpoint}`;
    const response = await fetch(url, {
        headers: {
            'Content-Type': 'application/json',
        },
        ...options,
    });

    const data = await response.json();

    if (!data.success) {
        throw new Error(data.error || 'API request failed');
    }

    return data.data;
}

async function loadProfiles() {
    profiles = await api('/profiles');
    renderProfileTabs();

    // Select first profile by default
    if (profiles.length > 0) {
        await selectProfile(profiles[0].name);
    }
}

async function loadProfile(name) {
    return await api(`/profiles/${name}`);
}

async function updateButton(profileName, position, data) {
    return await api(`/profiles/${profileName}/buttons/${position}`, {
        method: 'PUT',
        body: JSON.stringify(data),
    });
}

async function loadColors() {
    const data = await api('/colors');
    colorPresets = data.presets;
    renderColorPresets();
}

async function loadActions() {
    const data = await api('/actions');
    availableKeys = data.available_keys;
    builtinActions = data.builtin_actions;
    renderActionKeys();
    renderBuiltinActions();
}

async function reloadConfig() {
    await api('/reload', { method: 'POST' });
    await loadProfiles();
    showToast('Config reloaded', 'success');
}

// Profile management API functions
async function loadApps() {
    const data = await api('/apps');
    installedApps = data.apps;
    renderAppDropdown();
}

async function checkProfileHasDefaults(name) {
    const data = await api(`/profiles/${name}/has-defaults`);
    return data.has_defaults;
}

async function resetProfile(name) {
    return await api(`/profiles/${name}/reset`, { method: 'POST' });
}

async function createProfile(data) {
    return await api('/profiles', {
        method: 'POST',
        body: JSON.stringify(data),
    });
}

async function deleteProfile(name) {
    return await api(`/profiles/${name}`, { method: 'DELETE' });
}

async function resetButton(profileName, position) {
    return await api(`/profiles/${profileName}/buttons/${position}`, { method: 'DELETE' });
}

async function swapButtons(profileName, pos1, pos2) {
    return await api(`/profiles/${profileName}/buttons/swap`, {
        method: 'POST',
        body: JSON.stringify({ position1: pos1, position2: pos2 }),
    });
}

async function searchGiphy(query) {
    return await api(`/giphy/search?q=${encodeURIComponent(query)}&limit=12`);
}

// Render Functions
function renderProfileTabs() {
    elements.profileTabs.innerHTML = profiles.map(profile => {
        // Check if this is the default profile (matches everything with *)
        const isDefault = profile.match_apps.includes('*');
        const displayName = isDefault ? `${profile.name} (default)` : profile.name;
        const tooltip = `Matches: ${profile.match_apps.join(', ')}`;

        return `
            <button class="profile-tab" data-profile="${profile.name}" title="${tooltip}">
                ${displayName}
            </button>
        `;
    }).join('');
}

function renderButtonGrid() {
    if (!currentProfile) return;

    const topRow = [];
    const bottomRow = [];

    // Create button map for quick lookup
    const buttonMap = {};
    currentProfile.buttons.forEach(btn => {
        buttonMap[btn.position] = btn;
    });

    // Generate buttons for positions 0-9
    for (let i = 0; i < 10; i++) {
        const btn = buttonMap[i] || { position: i, label: '?', color: '#505560', bright_color: '#6E737D' };
        const html = createButtonCell(btn);

        if (i < 5) {
            topRow.push(html);
        } else {
            bottomRow.push(html);
        }
    }

    elements.buttonGrid.innerHTML = `
        <div class="button-row">${topRow.join('')}</div>
        <div class="button-row">${bottomRow.join('')}</div>
    `;
}

function createButtonCell(button) {
    const isSelected = currentButton && currentButton.position === button.position;
    const gradient = `linear-gradient(180deg, ${button.bright_color || button.color} 0%, ${button.color} 100%)`;

    // Check if this is a MIC button (action is custom with value "MIC")
    const isMicButton = button.action && button.action.type === 'custom' && button.action.value === 'MIC';

    // Check if button has non-default config (not just "---" placeholder)
    // Include MIC buttons and any button with a meaningful action
    const hasAction = button.action && button.action.type === 'custom' && button.action.value && button.action.value !== '';
    const hasConfig = button.label !== '---' || button.emoji_image || button.custom_image || button.gif_url || isMicButton || hasAction;

    // Build tooltip showing the action
    const tooltip = getButtonTooltip(button);

    // Determine content: mic icon, gif, emoji, custom image, or text label
    let content;
    if (isMicButton) {
        // Display microphone emoji for MIC action
        content = `<span class="button-emoji">🎤</span>`;
    } else if (button.gif_url) {
        // Display GIF
        content = `<img class="button-image" src="${button.gif_url}" alt="${button.label}">`;
    } else if (button.emoji_image && isEmoji(button.emoji_image)) {
        // Display emoji
        content = `<span class="button-emoji">${button.emoji_image}</span>`;
    } else if (button.custom_image) {
        // Display custom uploaded image
        content = `<img class="button-image" src="${button.custom_image}" alt="${button.label}">`;
    } else {
        // Display text label
        content = `<span class="button-label">${button.label}</span>`;
    }

    return `
        <div class="button-cell ${isSelected ? 'selected' : ''}"
             data-position="${button.position}"
             draggable="true"
             title="${tooltip}"
             style="background: ${gradient}; border-color: ${button.color}">
            <span class="button-position">${button.position}</span>
            ${hasConfig ? `<button class="btn-reset-tile" data-position="${button.position}" title="Reset button">✕</button>` : ''}
            ${content}
        </div>
    `;
}

// Generate tooltip text describing what the button does
function getButtonTooltip(button) {
    if (!button.action) return `Button ${button.position}`;

    const action = button.action;
    let actionDesc = '';

    switch (action.type) {
        case 'custom':
            // Find the builtin action name
            const builtin = builtinActions.find(a => a.value === action.value);
            actionDesc = builtin ? builtin.name : action.value;
            break;
        case 'key':
            actionDesc = `Press ${action.value}`;
            break;
        case 'text':
            actionDesc = `Type "${action.value}"`;
            break;
        case 'emoji':
        case 'slack_emoji':
            actionDesc = `Send ${action.value}`;
            break;
        default:
            actionDesc = action.value || 'No action';
    }

    return `${button.label}: ${actionDesc}`;
}

// Check if string is an emoji (non-ASCII character)
function isEmoji(str) {
    if (!str) return false;
    const firstChar = str.codePointAt(0);
    return firstChar > 127;
}

// Common emoji to Slack shortcode mapping
const emojiToShortcode = {
    '👍': ':+1:',
    '👎': ':-1:',
    '✅': ':white_check_mark:',
    '❌': ':x:',
    '❤️': ':heart:',
    '🔥': ':fire:',
    '⭐': ':star:',
    '👀': ':eyes:',
    '🎉': ':tada:',
    '💯': ':100:',
    '🙏': ':pray:',
    '😂': ':joy:',
    '🚀': ':rocket:',
    '💡': ':bulb:',
    '⚠️': ':warning:',
    '🐛': ':bug:',
};

// Get shortcode for an emoji, or return the emoji itself
function getEmojiShortcode(emoji) {
    return emojiToShortcode[emoji] || emoji;
}

// Parse a shortcut string like "Cmd+Shift+C" into modifiers and key
function parseShortcut(shortcut) {
    const parts = shortcut.split('+');
    const result = {
        cmd: false,
        ctrl: false,
        alt: false,  // "alt" internally, displayed as "Option" on Mac
        shift: false,
        key: ''
    };

    for (let i = 0; i < parts.length; i++) {
        const part = parts[i].toLowerCase();
        const isLast = i === parts.length - 1;

        if (!isLast) {
            // Modifier (accept both Mac and cross-platform names)
            if (part === 'cmd' || part === 'command' || part === 'meta' || part === '⌘') result.cmd = true;
            else if (part === 'ctrl' || part === 'control' || part === '⌃') result.ctrl = true;
            else if (part === 'alt' || part === 'option' || part === 'opt' || part === '⌥') result.alt = true;
            else if (part === 'shift' || part === '⇧') result.shift = true;
        } else {
            // Main key - preserve original case for the value
            result.key = parts[i];
        }
    }

    return result;
}

// Build a shortcut string from modifiers and key (Mac style)
function buildShortcut(cmd, ctrl, alt, shift, key) {
    const parts = [];
    if (ctrl) parts.push('Ctrl');
    if (alt) parts.push('Option');
    if (shift) parts.push('Shift');
    if (cmd) parts.push('Cmd');
    parts.push(key);
    return parts.join('+');
}

// Clear modifier checkboxes
function clearModifiers() {
    elements.modCmd.checked = false;
    elements.modCtrl.checked = false;
    elements.modAlt.checked = false;
    elements.modShift.checked = false;
}

function renderColorPresets() {
    elements.colorPresets.innerHTML = colorPresets.map(preset => `
        <div class="color-preset"
             data-color="${preset.color}"
             style="background: ${preset.color}"
             title="${preset.name}">
        </div>
    `).join('');
}

function renderActionKeys() {
    elements.editActionKey.innerHTML = availableKeys.map(key => `
        <option value="${key.value}">${key.name}</option>
    `).join('');
}

function renderBuiltinActions() {
    elements.editActionBuiltin.innerHTML = builtinActions.map(action => `
        <option value="${action.value}" title="${action.description}">${action.name}</option>
    `).join('');
}

function renderAppDropdown() {
    elements.newProfileApp.innerHTML = `
        <option value="">Select an app...</option>
        ${installedApps.map(app => `
            <option value="${app.name}">${app.name}</option>
        `).join('')}
    `;
}

function renderCopyFromDropdown() {
    elements.newProfileCopy.innerHTML = `
        <option value="">Empty (default buttons)</option>
        ${profiles.map(p => `
            <option value="${p.name}">${p.name}</option>
        `).join('')}
    `;
}

async function updateProfileActions() {
    if (!currentProfile) {
        elements.btnReset.classList.add('hidden');
        elements.btnDelete.classList.add('hidden');
        return;
    }

    // Check if profile has defaults (is a built-in profile)
    const hasDefaults = await checkProfileHasDefaults(currentProfile.name);

    if (hasDefaults) {
        elements.btnReset.classList.remove('hidden');
        elements.btnDelete.classList.add('hidden');
    } else {
        elements.btnReset.classList.add('hidden');
        elements.btnDelete.classList.remove('hidden');
    }
}

function renderEditor() {
    if (!currentButton) {
        elements.editorForm.classList.add('hidden');
        elements.editorHint.classList.remove('hidden');
        return;
    }

    elements.editorHint.classList.add('hidden');
    elements.editorForm.classList.remove('hidden');

    // Populate form
    elements.editPosition.value = currentButton.position;
    elements.editLabel.value = currentButton.label;
    elements.editColor.value = currentButton.color;
    // Auto-calculate bright color from base
    elements.editBrightColor.value = brightenColor(currentButton.color);

    // Get action for checks below
    const action = currentButton.action;

    // --- DISPLAY SECTION ---
    // Determine display type from button config
    // Check if this is a MIC button (action is Custom with value "MIC")
    const isMicAction = action.type === 'custom' && action.value === 'MIC';

    let displayType = 'text';
    if (isMicAction) {
        displayType = 'mic-icon';
        elements.micIconHint.classList.remove('hidden');
        clearCustomImage();
        clearGif();
        elements.editEmojiImage.value = '';
    } else if (currentButton.gif_url) {
        displayType = 'gif';
        selectedGifUrl = currentButton.gif_url;
        elements.gifUrlInput.value = currentButton.gif_url;
        elements.gifPreview.src = currentButton.gif_url;
        elements.gifPreviewContainer.classList.remove('hidden');
        clearCustomImage();
        elements.editEmojiImage.value = '';
        elements.micIconHint.classList.add('hidden');
    } else if (currentButton.custom_image) {
        displayType = 'image';
        currentCustomImage = currentButton.custom_image;
        elements.imagePreview.src = currentCustomImage;
        elements.imagePreview.classList.remove('hidden');
        elements.clearImageBtn.classList.remove('hidden');
        const dropContent = elements.imageDropZone.querySelector('.drop-zone-content');
        if (dropContent) dropContent.classList.add('hidden');
        clearGif();
        elements.micIconHint.classList.add('hidden');
    } else if (currentButton.emoji_image && isEmoji(currentButton.emoji_image)) {
        displayType = 'emoji';
        elements.editEmojiImage.value = currentButton.emoji_image;
        clearCustomImage();
        clearGif();
        elements.micIconHint.classList.add('hidden');
    } else {
        clearCustomImage();
        clearGif();
        elements.editEmojiImage.value = '';
        elements.micIconHint.classList.add('hidden');
    }
    elements.editDisplayType.value = displayType;
    updateDisplayUI(displayType);

    // --- ACTION SECTION ---
    // Parse action type (action was defined above)
    // Map slack_emoji to emoji for backwards compatibility
    const actionType = action.type === 'slack_emoji' ? 'emoji' : action.type;
    elements.editActionType.value = actionType;
    updateActionUI(actionType);

    // Populate action value
    if (actionType === 'key') {
        // Parse shortcut string to extract modifiers and key
        const shortcut = parseShortcut(action.value || '');
        elements.modCmd.checked = shortcut.cmd;
        elements.modCtrl.checked = shortcut.ctrl;
        elements.modAlt.checked = shortcut.alt;
        elements.modShift.checked = shortcut.shift;
        elements.editActionKey.value = shortcut.key || action.value;
        elements.editAutoSubmit.checked = false;
    } else if (actionType === 'custom') {
        clearModifiers();
        elements.editActionBuiltin.value = action.value || '';
        elements.editAutoSubmit.checked = false;
    } else {
        // Text or emoji action
        clearModifiers();
        elements.editActionValue.value = action.value || '';
        elements.editAutoSubmit.checked = action.auto_submit || false;
    }

    // Update color preset selection
    updateColorPresetSelection();
}

// Update display section UI based on display type
function updateDisplayUI(displayType) {
    const isText = displayType === 'text';
    const isEmoji = displayType === 'emoji';
    const isImage = displayType === 'image';
    const isGif = displayType === 'gif';
    const isMicIcon = displayType === 'mic-icon';

    // Show/hide display-specific fields
    // For mic-icon, hide all display fields since it's automatic
    if (isText) {
        elements.labelGroup.classList.remove('hidden');
    } else {
        elements.labelGroup.classList.add('hidden');
    }

    if (isEmoji) {
        elements.emojiDisplayGroup.classList.remove('hidden');
    } else {
        elements.emojiDisplayGroup.classList.add('hidden');
    }

    if (isImage) {
        elements.imageDisplayGroup.classList.remove('hidden');
    } else {
        elements.imageDisplayGroup.classList.add('hidden');
    }

    if (isGif) {
        elements.gifDisplayGroup.classList.remove('hidden');
    } else {
        elements.gifDisplayGroup.classList.add('hidden');
    }

    // Mic icon hint is handled separately
}

// Update action section UI based on action type
function updateActionUI(actionType) {
    const isKey = actionType === 'key';
    const isCustom = actionType === 'custom';
    const isEmoji = actionType === 'emoji';
    const isText = actionType === 'text';

    // Hide all inputs first
    elements.editActionValue.classList.add('hidden');
    elements.editActionKey.classList.add('hidden');
    elements.editActionBuiltin.classList.add('hidden');
    elements.modifierGroup.classList.add('hidden');
    elements.autoSubmitGroup.classList.add('hidden');

    // Show appropriate input
    if (isKey) {
        elements.editActionKey.classList.remove('hidden');
        elements.modifierGroup.classList.remove('hidden');
    } else if (isCustom) {
        elements.editActionBuiltin.classList.remove('hidden');
    } else {
        elements.editActionValue.classList.remove('hidden');
        // Show auto-submit option for text and emoji
        elements.autoSubmitGroup.classList.remove('hidden');
    }

    // Update label and placeholder
    const label = elements.actionValueGroup.querySelector('label');
    if (isKey) {
        label.textContent = 'Key';
    } else if (isText) {
        label.textContent = 'Text to type';
        elements.editActionValue.placeholder = 'Hello world';
    } else if (isEmoji) {
        label.textContent = 'Shortcode';
        elements.editActionValue.placeholder = ':+1:';
    } else if (isCustom) {
        label.textContent = 'Action';
    }

    elements.editActionValue.disabled = false;
}

function updateColorPresetSelection() {
    const color = elements.editColor.value.toUpperCase();

    document.querySelectorAll('.color-preset').forEach(preset => {
        const presetColor = preset.dataset.color.toUpperCase();
        preset.classList.toggle('selected', presetColor === color);
    });
}

// Brighten a hex color by ~30% for gradient effect
function brightenColor(hex) {
    // Parse hex
    const r = parseInt(hex.slice(1, 3), 16);
    const g = parseInt(hex.slice(3, 5), 16);
    const b = parseInt(hex.slice(5, 7), 16);

    // Brighten by 30%, capped at 255
    const br = Math.min(255, Math.round(r * 1.3));
    const bg = Math.min(255, Math.round(g * 1.3));
    const bb = Math.min(255, Math.round(b * 1.3));

    // Return hex
    return `#${br.toString(16).padStart(2, '0')}${bg.toString(16).padStart(2, '0')}${bb.toString(16).padStart(2, '0')}`.toUpperCase();
}

// Event Handlers
function setupEventListeners() {
    // Profile tabs
    elements.profileTabs.addEventListener('click', async (e) => {
        const tab = e.target.closest('.profile-tab');
        if (tab) {
            await selectProfile(tab.dataset.profile);
        }
    });

    // Button grid - click to select
    elements.buttonGrid.addEventListener('click', async (e) => {
        // Check if reset button was clicked
        const resetBtn = e.target.closest('.btn-reset-tile');
        if (resetBtn) {
            e.stopPropagation();
            const position = parseInt(resetBtn.dataset.position);
            if (confirm(`Reset button ${position} to default?`)) {
                try {
                    await resetButton(currentProfile.name, position);
                    await selectProfile(currentProfile.name);
                    showToast('Button reset', 'success');
                } catch (error) {
                    showToast(`Failed to reset: ${error.message}`, 'error');
                }
            }
            return;
        }

        const cell = e.target.closest('.button-cell');
        if (cell) {
            selectButton(parseInt(cell.dataset.position));
        }
    });

    // Button grid - drag and drop to reorder
    elements.buttonGrid.addEventListener('dragstart', (e) => {
        const cell = e.target.closest('.button-cell');
        if (cell) {
            draggedButton = parseInt(cell.dataset.position);
            cell.classList.add('dragging');
            e.dataTransfer.effectAllowed = 'move';
        }
    });

    elements.buttonGrid.addEventListener('dragend', (e) => {
        const cell = e.target.closest('.button-cell');
        if (cell) {
            cell.classList.remove('dragging');
        }
        draggedButton = null;
        // Remove all drag-over states
        document.querySelectorAll('.button-cell').forEach(c => c.classList.remove('drag-over'));
    });

    elements.buttonGrid.addEventListener('dragover', (e) => {
        e.preventDefault();
        const cell = e.target.closest('.button-cell');
        if (cell && draggedButton !== null) {
            const targetPos = parseInt(cell.dataset.position);
            if (targetPos !== draggedButton) {
                // Remove drag-over from all cells, add to current
                document.querySelectorAll('.button-cell').forEach(c => c.classList.remove('drag-over'));
                cell.classList.add('drag-over');
            }
        }
    });

    elements.buttonGrid.addEventListener('dragleave', (e) => {
        const cell = e.target.closest('.button-cell');
        if (cell) {
            cell.classList.remove('drag-over');
        }
    });

    elements.buttonGrid.addEventListener('drop', async (e) => {
        e.preventDefault();
        const cell = e.target.closest('.button-cell');
        if (cell && draggedButton !== null) {
            const targetPos = parseInt(cell.dataset.position);
            if (targetPos !== draggedButton) {
                try {
                    await swapButtons(currentProfile.name, draggedButton, targetPos);
                    await selectProfile(currentProfile.name);
                    showToast('Buttons swapped', 'success');
                } catch (error) {
                    showToast(`Failed to swap: ${error.message}`, 'error');
                }
            }
        }
        draggedButton = null;
        document.querySelectorAll('.button-cell').forEach(c => c.classList.remove('drag-over', 'dragging'));
    });

    // Color presets
    elements.colorPresets.addEventListener('click', (e) => {
        const preset = e.target.closest('.color-preset');
        if (preset) {
            elements.editColor.value = preset.dataset.color;
            elements.editBrightColor.value = brightenColor(preset.dataset.color);
            updateColorPresetSelection();
        }
    });

    // Color input - auto-calculate bright color
    elements.editColor.addEventListener('input', () => {
        elements.editBrightColor.value = brightenColor(elements.editColor.value);
        updateColorPresetSelection();
    });

    // Action type change
    elements.editActionType.addEventListener('change', (e) => {
        updateActionUI(e.target.value);
        // If switching away from custom, hide mic hint
        if (e.target.value !== 'custom') {
            elements.micIconHint.classList.add('hidden');
            if (elements.editDisplayType.value === 'mic-icon') {
                elements.editDisplayType.value = 'text';
                updateDisplayUI('text');
            }
        }
    });

    // Built-in action change - update display if MIC selected
    elements.editActionBuiltin.addEventListener('change', (e) => {
        const isMic = e.target.value === 'MIC';
        if (isMic) {
            elements.editDisplayType.value = 'mic-icon';
            elements.micIconHint.classList.remove('hidden');
            updateDisplayUI('mic-icon');
        } else {
            elements.micIconHint.classList.add('hidden');
            if (elements.editDisplayType.value === 'mic-icon') {
                elements.editDisplayType.value = 'text';
                updateDisplayUI('text');
            }
        }
    });

    // Display type change
    elements.editDisplayType.addEventListener('change', (e) => {
        const displayType = e.target.value;
        updateDisplayUI(displayType);

        // If mic-icon selected, auto-set action to MIC
        if (displayType === 'mic-icon') {
            elements.editActionType.value = 'custom';
            elements.editActionBuiltin.value = 'MIC';
            updateActionUI('custom');
            elements.micIconHint.classList.remove('hidden');
        }
    });

    // Form submit
    elements.editorForm.addEventListener('submit', async (e) => {
        e.preventDefault();
        await saveButton();
    });

    // Cancel button
    elements.btnCancel.addEventListener('click', () => {
        currentButton = null;
        renderEditor();
        renderButtonGrid();
    });

    // Copy button
    elements.btnCopy.addEventListener('click', () => {
        if (!currentButton) return;
        copyButtonToPosition();
    });

    // Reload button
    elements.btnReload.addEventListener('click', async () => {
        try {
            await reloadConfig();
        } catch (error) {
            showToast(error.message, 'error');
        }
    });

    // Emoji picker
    document.getElementById('emoji-picker').addEventListener('click', (e) => {
        const option = e.target.closest('.emoji-option');
        if (option) {
            elements.editEmojiImage.value = option.dataset.emoji;
            // Clear custom image when emoji is selected
            clearCustomImage();
        }
    });

    // Emoji input clears custom image
    elements.editEmojiImage.addEventListener('input', () => {
        if (elements.editEmojiImage.value) {
            clearCustomImage();
        }
    });

    // Image upload - click to select
    elements.imageDropZone.addEventListener('click', () => {
        elements.imageFileInput.click();
    });

    // Image upload - file selected
    elements.imageFileInput.addEventListener('change', (e) => {
        const file = e.target.files[0];
        if (file) {
            processImageFile(file);
        }
    });

    // Image upload - drag over
    elements.imageDropZone.addEventListener('dragover', (e) => {
        e.preventDefault();
        elements.imageDropZone.classList.add('drag-over');
    });

    elements.imageDropZone.addEventListener('dragleave', () => {
        elements.imageDropZone.classList.remove('drag-over');
    });

    // Image upload - drop
    elements.imageDropZone.addEventListener('drop', (e) => {
        e.preventDefault();
        elements.imageDropZone.classList.remove('drag-over');
        const file = e.dataTransfer.files[0];
        if (file && file.type.startsWith('image/')) {
            processImageFile(file);
        }
    });

    // Clear image button
    elements.clearImageBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        clearCustomImage();
    });

    // GIF URL input - show preview when URL is pasted/entered
    elements.gifUrlInput.addEventListener('input', () => {
        const url = elements.gifUrlInput.value.trim();
        if (url) {
            selectedGifUrl = url;
            elements.gifPreview.src = url;
            elements.gifPreviewContainer.classList.remove('hidden');
        } else {
            selectedGifUrl = null;
            elements.gifPreviewContainer.classList.add('hidden');
        }
    });

    // GIF search
    elements.gifSearchBtn.addEventListener('click', performGifSearch);
    elements.gifSearchInput.addEventListener('keypress', (e) => {
        if (e.key === 'Enter') {
            e.preventDefault();
            performGifSearch();
        }
    });

    // GIF selection from results
    elements.gifResults.addEventListener('click', (e) => {
        const gifItem = e.target.closest('.gif-item');
        if (gifItem) {
            selectGif(gifItem.dataset.url, gifItem.dataset.preview);
        }
    });

    // Clear GIF button
    elements.clearGifBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        clearGif();
    });

    // Profile management buttons
    elements.btnReset.addEventListener('click', async () => {
        if (!currentProfile) return;
        if (!confirm(`Reset "${currentProfile.name}" profile to default button configuration?`)) return;

        try {
            await resetProfile(currentProfile.name);
            await selectProfile(currentProfile.name);
            showToast('Profile reset to defaults', 'success');
        } catch (error) {
            showToast(`Failed to reset: ${error.message}`, 'error');
        }
    });

    elements.btnDelete.addEventListener('click', async () => {
        if (!currentProfile) return;
        if (!confirm(`Delete "${currentProfile.name}" profile? This cannot be undone.`)) return;

        try {
            await deleteProfile(currentProfile.name);
            await loadProfiles();
            showToast('Profile deleted', 'success');
        } catch (error) {
            showToast(`Failed to delete: ${error.message}`, 'error');
        }
    });

    elements.btnNewProfile.addEventListener('click', () => {
        renderCopyFromDropdown();
        elements.newProfileName.value = '';
        elements.newProfileApp.value = '';
        elements.createProfileModal.classList.remove('hidden');
    });

    elements.btnCancelCreate.addEventListener('click', () => {
        elements.createProfileModal.classList.add('hidden');
    });

    // Close modal on overlay click
    elements.createProfileModal.addEventListener('click', (e) => {
        if (e.target === elements.createProfileModal) {
            elements.createProfileModal.classList.add('hidden');
        }
    });

    // App dropdown auto-fills profile name
    elements.newProfileApp.addEventListener('change', () => {
        const appName = elements.newProfileApp.value;
        if (appName) {
            elements.newProfileName.value = appName.toLowerCase().replace(/\s+/g, '-');
        }
    });

    // Create profile form submit
    elements.createProfileForm.addEventListener('submit', async (e) => {
        e.preventDefault();

        const name = elements.newProfileName.value.trim();
        const appName = elements.newProfileApp.value;
        const copyFrom = elements.newProfileCopy.value || null;

        if (!name) {
            showToast('Please enter a profile name', 'error');
            return;
        }

        if (!appName) {
            showToast('Please select an application', 'error');
            return;
        }

        const matchApps = [appName];

        try {
            await createProfile({ name, match_apps: matchApps, copy_from: copyFrom });
            elements.createProfileModal.classList.add('hidden');
            await loadProfiles();
            await selectProfile(name.toLowerCase().replace(/\s+/g, '-'));
            showToast('Profile created', 'success');
        } catch (error) {
            showToast(`Failed to create profile: ${error.message}`, 'error');
        }
    });
}

// Process uploaded image - resize to 90x90 (buttons are 112x112) and convert to base64
function processImageFile(file) {
    const reader = new FileReader();
    reader.onload = (e) => {
        const img = new Image();
        img.onload = () => {
            // Create canvas for resizing
            const canvas = document.createElement('canvas');
            const ctx = canvas.getContext('2d');
            const size = 90;  // Match device render size
            canvas.width = size;
            canvas.height = size;

            // Calculate crop to center
            const srcSize = Math.min(img.width, img.height);
            const srcX = (img.width - srcSize) / 2;
            const srcY = (img.height - srcSize) / 2;

            // Draw resized/cropped image
            ctx.drawImage(img, srcX, srcY, srcSize, srcSize, 0, 0, size, size);

            // Convert to base64
            currentCustomImage = canvas.toDataURL('image/png');

            // Show preview
            elements.imagePreview.src = currentCustomImage;
            elements.imagePreview.classList.remove('hidden');
            elements.clearImageBtn.classList.remove('hidden');
            const dropContent = elements.imageDropZone.querySelector('.drop-zone-content');
            if (dropContent) dropContent.classList.add('hidden');

            // Clear emoji when custom image is set
            elements.editEmojiImage.value = '';
        };
        img.src = e.target.result;
    };
    reader.readAsDataURL(file);
}

// Clear custom image
function clearCustomImage() {
    currentCustomImage = null;
    elements.imagePreview.src = '';
    elements.imagePreview.classList.add('hidden');
    elements.clearImageBtn.classList.add('hidden');
    elements.imageDropZone.querySelector('.drop-zone-content').classList.remove('hidden');
    elements.imageFileInput.value = '';
}

// Clear selected GIF
function clearGif() {
    selectedGifUrl = null;
    elements.gifUrlInput.value = '';
    elements.gifPreview.src = '';
    elements.gifPreviewContainer.classList.add('hidden');
    elements.gifResults.innerHTML = '<p class="gif-hint">Enter a search term to find GIFs</p>';
    elements.gifSearchInput.value = '';
}

// Search for GIFs
async function performGifSearch() {
    const query = elements.gifSearchInput.value.trim();
    if (!query) {
        showToast('Enter a search term', 'error');
        return;
    }

    elements.gifResults.innerHTML = '<div class="gif-loading">Searching...</div>';

    try {
        const data = await searchGiphy(query);
        renderGifResults(data.gifs);
    } catch (error) {
        elements.gifResults.innerHTML = `<div class="gif-error">${error.message}</div>`;
    }
}

// Render GIF search results
function renderGifResults(gifs) {
    if (!gifs || gifs.length === 0) {
        elements.gifResults.innerHTML = '<p class="gif-hint">No GIFs found. Try another search.</p>';
        return;
    }

    elements.gifResults.innerHTML = `
        <div class="gif-grid">
            ${gifs.map(gif => `
                <div class="gif-item ${selectedGifUrl === gif.url ? 'selected' : ''}"
                     data-url="${gif.url}"
                     data-preview="${gif.preview_url}"
                     title="${gif.title}">
                    <img src="${gif.preview_url}" alt="${gif.title}" loading="lazy">
                </div>
            `).join('')}
        </div>
    `;
}

// Select a GIF from search results
function selectGif(url, previewUrl) {
    selectedGifUrl = url;
    elements.gifPreview.src = previewUrl || url;
    elements.gifPreviewContainer.classList.remove('hidden');

    // Update selection state in grid
    document.querySelectorAll('.gif-item').forEach(item => {
        item.classList.toggle('selected', item.dataset.url === url);
    });

    // Clear other display types
    clearCustomImage();
    elements.editEmojiImage.value = '';
}

async function selectProfile(name) {
    try {
        currentProfile = await loadProfile(name);
        currentButton = null;

        // Update tab selection
        document.querySelectorAll('.profile-tab').forEach(tab => {
            tab.classList.toggle('active', tab.dataset.profile === name);
        });

        renderButtonGrid();
        renderEditor();
        await updateProfileActions();
    } catch (error) {
        showToast(`Failed to load profile: ${error.message}`, 'error');
    }
}

function selectButton(position) {
    const button = currentProfile.buttons.find(b => b.position === position);
    if (button) {
        currentButton = { ...button };
        renderEditor();
        renderButtonGrid();
    }
}

async function saveButton() {
    if (!currentProfile || !currentButton) return;

    // --- DISPLAY ---
    const displayType = elements.editDisplayType.value;
    const displayEmoji = elements.editEmojiImage.value;

    // Determine what's displayed on the button
    let label = elements.editLabel.value;
    let emoji_image = '';
    let custom_image = '';
    let gif_url = '';

    if (displayType === 'emoji') {
        emoji_image = displayEmoji;
        // For emoji display, use the emoji as the label fallback
        label = displayEmoji || label;
    } else if (displayType === 'image') {
        custom_image = currentCustomImage || '';
    } else if (displayType === 'gif') {
        gif_url = selectedGifUrl || '';
    }
    // For 'text' display type, just use the label as-is

    // --- ACTION ---
    const actionType = elements.editActionType.value;

    // Get action value based on action type
    let actionValue;
    let autoSubmit = false;
    if (actionType === 'key') {
        // Build shortcut string from modifiers and key
        const key = elements.editActionKey.value;
        const cmd = elements.modCmd.checked;
        const ctrl = elements.modCtrl.checked;
        const alt = elements.modAlt.checked;
        const shift = elements.modShift.checked;
        actionValue = buildShortcut(cmd, ctrl, alt, shift, key);
    } else if (actionType === 'custom') {
        actionValue = elements.editActionBuiltin.value;
    } else {
        // For emoji and text actions
        actionValue = elements.editActionValue.value;
        autoSubmit = elements.editAutoSubmit.checked;
    }

    // Build action object
    const action = { type: actionType, value: actionValue };
    if (actionType === 'text' || actionType === 'emoji') {
        action.auto_submit = autoSubmit;
    }

    const data = {
        label: label,
        color: elements.editColor.value,
        bright_color: elements.editBrightColor.value,
        action: action,
        emoji_image: emoji_image,
        custom_image: custom_image,
        gif_url: gif_url,
    };

    try {
        await updateButton(currentProfile.name, currentButton.position, data);

        // Update local state
        const idx = currentProfile.buttons.findIndex(b => b.position === currentButton.position);
        if (idx !== -1) {
            currentProfile.buttons[idx] = { ...currentProfile.buttons[idx], ...data };
        }
        currentButton = { ...currentButton, ...data };

        renderButtonGrid();
        showToast('Button saved', 'success');
    } catch (error) {
        showToast(`Failed to save: ${error.message}`, 'error');
    }
}

// UI Helpers
function setConnected(connected, errorMessage = null) {
    elements.connectionStatus.classList.toggle('connected', connected);
    elements.connectionStatus.classList.toggle('error', !connected);
    elements.connectionStatus.querySelector('.status-text').textContent =
        connected ? 'Connected' : (errorMessage || 'Disconnected');
}

// Copy current button config to another position
async function copyButtonToPosition() {
    if (!currentProfile || !currentButton) return;

    // Get available positions (0-9 except current)
    const positions = [];
    for (let i = 0; i < 10; i++) {
        if (i !== currentButton.position) {
            const btn = currentProfile.buttons.find(b => b.position === i);
            const label = btn ? btn.label : '?';
            positions.push({ pos: i, label });
        }
    }

    // Create a simple prompt with position options
    const posStr = positions.map(p => `${p.pos}`).join(', ');
    const targetStr = prompt(`Copy "${currentButton.label}" to which position?\nAvailable: ${posStr}`);

    if (targetStr === null) return; // Cancelled

    const targetPos = parseInt(targetStr.trim());
    if (isNaN(targetPos) || targetPos < 0 || targetPos > 9 || targetPos === currentButton.position) {
        showToast('Invalid position', 'error');
        return;
    }

    // Copy the button data (excluding position)
    const data = {
        label: currentButton.label,
        color: currentButton.color,
        bright_color: currentButton.bright_color,
        action: currentButton.action,
        emoji_image: currentButton.emoji_image || '',
        custom_image: currentButton.custom_image || '',
        gif_url: currentButton.gif_url || '',
    };

    try {
        await updateButton(currentProfile.name, targetPos, data);
        await selectProfile(currentProfile.name);
        showToast(`Copied to position ${targetPos}`, 'success');
    } catch (error) {
        showToast(`Failed to copy: ${error.message}`, 'error');
    }
}

function showToast(message, type = 'info') {
    const toast = document.createElement('div');
    toast.className = `toast ${type}`;
    toast.textContent = message;
    document.body.appendChild(toast);

    // Trigger animation
    requestAnimationFrame(() => {
        toast.classList.add('show');
    });

    // Remove after delay
    setTimeout(() => {
        toast.classList.remove('show');
        setTimeout(() => toast.remove(), 300);
    }, 3000);
}

// Start
init();
