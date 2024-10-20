# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

### Changed
- use "return" command for conditionals instead of data storage when using supported pack format

### Removed


## [0.1.0] - 2024-10-01

### Added

- Datapack struct with compile function
- Namespace struct with compile function
- Function struct with compile function
- Tag struct with compile function
- Command struct with compile function
    - Raw command
    - Comment
    - Execute
    - Debug
    - Group
- Validate function for checking pack format compatibility
- Virtual file system


[unreleased]: https://github.com/moritz-hoelting/shulkerbox/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/moritz-hoelting/shulkerbox/releases/tag/v0.1.0