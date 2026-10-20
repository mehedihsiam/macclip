# Spec Delta

## Purpose

Deliver MacClip as a properly identified, signed macOS application bundle built entirely in Rust that runs on current Macs.

## ADDED Requirements

### Requirement: Application identity
The app SHALL be named "MacClip" with the description "Clipboard history for macOS" and bundle identifier com.macclip.app.

#### Scenario: Bundle metadata
- **WHEN** the built app's Info.plist is read
- **THEN** the display name is MacClip and the bundle identifier is com.macclip.app

### Requirement: Rust implementation
All application logic SHALL be implemented in Rust; no Swift or Objective-C source files are part of the product.

#### Scenario: Source audit
- **WHEN** the repository sources are listed
- **THEN** application code consists of Rust files only (plus scripts and resources)

### Requirement: Background-only app bundle
The bundle SHALL be an agent app (no Dock icon) requiring macOS 13.0 or later.

#### Scenario: Plist flags
- **WHEN** the Info.plist is inspected
- **THEN** LSUIElement is true and LSMinimumSystemVersion is 13.0

### Requirement: Universal signed build
The build process SHALL produce a universal (arm64 + x86_64) MacClip.app that is ad-hoc code-signed, and SHALL offer installation to /Applications.

#### Scenario: Build script
- **WHEN** the build script is run
- **THEN** MacClip.app contains a universal binary and passes codesign verification
