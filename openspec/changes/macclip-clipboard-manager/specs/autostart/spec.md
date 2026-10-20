# Spec Delta

## Purpose

Keep MacClip always available by launching it at login and running exactly one background instance.

## ADDED Requirements

### Requirement: Launch at login
The system SHALL register itself as a login item on first run so it starts automatically at login, and SHALL let the user toggle this from the context menu.

#### Scenario: Reboot
- **WHEN** the user logs in after a restart
- **THEN** MacClip is running with its menu-bar icon visible

### Requirement: Self-healing login item
On every launch the system SHALL re-register itself as a login item if it is not registered, unless the user turned launch-at-login off on purpose.

#### Scenario: Registration lost
- **WHEN** MacClip is launched and is no longer a registered login item and the user never turned it off
- **THEN** it registers itself again

#### Scenario: Turned off by the user
- **WHEN** the user unchecks "Launch at login" in the menu
- **THEN** MacClip does not re-register itself until the user turns it back on

### Requirement: Login item approval
If macOS requires user approval for the login item, the system SHALL tell the user and offer to open Login Items settings.

#### Scenario: Requires approval
- **WHEN** registration reports that approval is required
- **THEN** the user is guided to System Settings → General → Login Items

### Requirement: Single instance
The system SHALL ensure only one instance runs; launching a second instance SHALL exit without creating another icon.

#### Scenario: Double launch
- **WHEN** the user opens MacClip while it is already running
- **THEN** no second icon or process remains
