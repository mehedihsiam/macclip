# Spec Delta

## Purpose

Let the user paste any history entry immediately into the field they were typing in, preserving the original content formats.

## ADDED Requirements

### Requirement: Paste selected entry
When the user selects an entry and Accessibility permission is granted, the system SHALL close the panel and paste the entry into the previously focused field of the previously frontmost app.

#### Scenario: Select in TextEdit
- **WHEN** a TextEdit field is focused, the user opens the panel and selects an entry
- **THEN** the entry's content is inserted at the caret in TextEdit

### Requirement: Preserve formats
The system SHALL restore all stored representations (plain text, RTF, HTML, image) to the clipboard before pasting.

#### Scenario: Rich text re-paste
- **WHEN** a rich-text entry is selected and pasted into a rich-text editor
- **THEN** formatting is preserved

### Requirement: Selected entry becomes current
After selection the entry SHALL be the current clipboard content and SHALL move to the top of the history.

#### Scenario: Manual paste afterwards
- **WHEN** the user presses ⌘V after selecting an entry
- **THEN** the same content is pasted again

### Requirement: Fallback without permission
If Accessibility permission is not granted, the system SHALL still place the entry on the clipboard and SHALL notify the user to press ⌘V manually.

#### Scenario: Not trusted
- **WHEN** an entry is selected without Accessibility permission
- **THEN** the entry is on the clipboard and a notification explains manual paste and how to enable auto-paste
