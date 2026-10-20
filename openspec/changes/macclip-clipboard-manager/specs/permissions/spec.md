# Spec Delta

## Purpose

Request and handle the macOS permissions MacClip depends on transparently, degrading gracefully when they are missing.

## ADDED Requirements

### Requirement: Accessibility onboarding
On first launch without Accessibility trust, the system SHALL explain why the permission is needed and then trigger the system prompt, offering a shortcut to the Accessibility pane of System Settings.

#### Scenario: First run
- **WHEN** MacClip launches for the first time
- **THEN** an explanation is shown once, followed by the macOS Accessibility prompt

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
