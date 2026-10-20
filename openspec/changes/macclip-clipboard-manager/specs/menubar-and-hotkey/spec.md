# Spec Delta

## Purpose

Give MacClip a menu-bar presence in the top-right status area and a global shortcut so the history is reachable from anywhere in one action.

## ADDED Requirements

### Requirement: Menu-bar icon without Dock presence
The system SHALL show a template-style status icon in the right side of the menu bar while running, and SHALL NOT show a Dock icon or app-switcher entry.

#### Scenario: Launch
- **WHEN** MacClip starts
- **THEN** its icon appears in the menu bar and no Dock icon appears

### Requirement: Icon click opens history
Left-clicking the status icon SHALL toggle the history panel anchored beneath the icon.

#### Scenario: Click icon
- **WHEN** the user left-clicks the icon
- **THEN** the history panel appears below the icon, and a second click closes it

### Requirement: Context menu
Right-clicking (or Control-clicking) the icon SHALL show a menu with Pause/Resume capture, Clear history, Launch at login (checkmark), Accessibility status and Quit.

#### Scenario: Quit
- **WHEN** the user chooses Quit
- **THEN** the app terminates and the icon disappears

### Requirement: Global Option+V shortcut
The system SHALL open the history panel on the screen containing the mouse pointer when the user presses Option+V in any application, and SHALL close it if already open.

#### Scenario: Shortcut in another app
- **WHEN** the user presses Option+V while another app is focused
- **THEN** the history panel appears and the other app stays frontmost

### Requirement: Hotkey registration failure is visible
If the shortcut cannot be registered, the system SHALL indicate this in the context menu and log it.

#### Scenario: Conflict
- **WHEN** another process already owns Option+V
- **THEN** the menu shows that the shortcut is unavailable while the icon still works
