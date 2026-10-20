# Spec Delta

## Purpose

Request and handle the macOS permissions MacClip depends on transparently, degrading gracefully when they are missing.

## ADDED Requirements

### Requirement: Permission request on every launch
On every launch where a needed permission is missing, the system SHALL show one dialog explaining each missing permission and why it is needed, then trigger the system prompt and open the matching System Settings pane. The dialog SHALL offer a "Don't remind me again" option, which is cleared automatically once all permissions are granted.

#### Scenario: Missing Accessibility
- **WHEN** MacClip launches without Accessibility trust
- **THEN** a dialog explains it and offers "Allow Access…", which shows the macOS prompt and opens Accessibility settings

#### Scenario: Clipboard access denied
- **WHEN** macOS is set to never allow MacClip to read other apps' clipboard content
- **THEN** the same dialog also explains that setting and opens Privacy & Security

#### Scenario: Stale entry after an update
- **WHEN** the dialog is shown
- **THEN** it tells the user to remove and re-add MacClip if it is already listed but not working

### Requirement: Grant is detected automatically
After the user chooses "Allow Access…", the system SHALL watch for the permission and confirm when auto-paste becomes available, without a restart.

#### Scenario: Granted in System Settings
- **WHEN** the user switches MacClip on in Accessibility settings
- **THEN** within a few seconds MacClip reports that it is ready

### Requirement: Live permission status
The system SHALL re-check Accessibility trust whenever the panel opens and while waiting after the prompt, and SHALL show its status in the context menu.

#### Scenario: Granted later
- **WHEN** the user grants permission in System Settings while MacClip runs
- **THEN** auto-paste works without restarting the app

### Requirement: Works without extra permissions
Clipboard monitoring, history and the Option+V shortcut SHALL function without Accessibility, Input Monitoring or Screen Recording permission.

#### Scenario: Permission denied
- **WHEN** Accessibility is denied
- **THEN** history capture and the panel still work, with copy-only selection
