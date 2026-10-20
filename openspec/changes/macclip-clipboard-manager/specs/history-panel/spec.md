# Spec Delta

## Purpose

Present the clipboard history in a fast, keyboard-friendly floating panel that does not steal focus from the app the user is working in.

## ADDED Requirements

### Requirement: Show history list
The panel SHALL list entries newest first in a compact, low-contrast layout: a single-line truncated text preview, a small thumbnail for image entries, and a faint relative copy time. Selection SHALL be shown with a subtle tint, not a high-contrast fill.

#### Scenario: Mixed history
- **WHEN** the panel opens with text and image entries
- **THEN** text entries show single-line previews, image entries show small thumbnails, and the selected row uses a faint highlight

### Requirement: Non-activating panel
The panel SHALL NOT make MacClip the active application, so the previously focused app and text field keep focus.

#### Scenario: Open over a text editor
- **WHEN** the panel opens while a text field is focused in another app
- **THEN** that app remains the frontmost application

### Requirement: Keyboard navigation
The panel SHALL support Up/Down to move selection, Return to paste the selected entry, Escape to close, and keys 1–9 to paste the corresponding entry.

#### Scenario: Quick pick
- **WHEN** the panel is open and the user presses 3
- **THEN** the third entry is pasted and the panel closes

### Requirement: Search
The panel SHALL filter entries by typed text as the user types.

#### Scenario: Filter
- **WHEN** the user types "invoice"
- **THEN** only entries whose text contains "invoice" (case-insensitive) are listed

### Requirement: Dismissal
The panel SHALL close when the user presses Escape, clicks outside it, switches app or Space, or selects an entry.

#### Scenario: Click outside
- **WHEN** the user clicks outside the panel
- **THEN** the panel closes without pasting

### Requirement: Empty state
The panel SHALL show a message when the history is empty.

#### Scenario: Fresh install
- **WHEN** the panel opens with no entries
- **THEN** it displays an empty-state message instead of a blank list
