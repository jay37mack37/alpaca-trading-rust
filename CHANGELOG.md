# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-04-27
### Added
- Re-introduced the "Create Agent" workstation panel.
- Added strategy templates: `gamma_flip`, `jarrod_vwap`, and `distribution_sniper`.
- Added `CHANGELOG.md` and `FEATURES.md` for project organization.
- Added version display to the Diagnostic Header.

### Fixed
- Stabilized `start.bat` with proper `.env` parsing and cargo binary selection.
- Fixed API token synchronization between frontend and backend.
- Added "Reset Session" feature to clear stale localStorage tokens.
- Addressed backend startup crashes by implementing more robust credential fallback.

## [0.1.0] - 2026-04-20
### Added
- Initial release of the Alpaca Trading Platform.
- Real-time P&L tracking and workstation dashboard.
