# Spec Delta

## Purpose

Keep a bounded, de-duplicated, ordered, locally and privately persisted history of the most recent 20 clipboard entries.

## ADDED Requirements

### Requirement: Bounded history
The system SHALL keep at most 20 entries, ordered newest first, discarding the oldest when a 21st is added.

#### Scenario: Overflow
- **WHEN** 25 distinct items are copied in sequence
- **THEN** exactly the 20 most recent are retained, newest first

### Requirement: De-duplicate by content
The system SHALL treat identical content as one entry; re-copying existing content moves it to the top instead of adding a duplicate.

#### Scenario: Re-copy old item
- **WHEN** the user copies content identical to an existing older entry
- **THEN** that entry moves to the top and the total count does not increase

### Requirement: Persist across restarts
The system SHALL persist history on disk so entries survive app restarts and reboots.

#### Scenario: Relaunch
- **WHEN** the app is quit and launched again
- **THEN** the same entries appear in the same order

### Requirement: Private local storage
The system SHALL store all data only on the local machine under the user's Application Support directory, with directory permissions 0700 and file permissions 0600, and SHALL NOT make network connections.

#### Scenario: Permissions
- **WHEN** the data directory is inspected
- **THEN** it is owner-only accessible and contains no world- or group-readable files

### Requirement: Clean up removed images
The system SHALL delete image and thumbnail files of entries that are evicted or cleared.

#### Scenario: Eviction of image entry
- **WHEN** an image entry is evicted from the history
- **THEN** its image and thumbnail files no longer exist on disk

### Requirement: Clear history
The system SHALL let the user clear all history from the menu-bar menu.

#### Scenario: Clear
- **WHEN** the user chooses Clear history
- **THEN** the history is empty and its stored files are removed
