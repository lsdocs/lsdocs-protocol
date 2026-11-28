# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/lsdocs/lsdocs-protocol/compare/lsdocs-config/v0.1.0...lsdocs-config/v0.1.1) - 2026-11-28

### Fixed

- name the parser-myst builtin in place of parser-markdown ([#18](https://github.com/lsdocs/lsdocs-protocol/pull/18))

## [0.1.0](https://github.com/lsdocs/lsdocs-protocol/releases/tag/lsdocs-config/v0.1.0) - 2026-11-22

### Added

- release automation ([#3](https://github.com/lsdocs/lsdocs-protocol/pull/3))
- *(render-html)* themes, their settings and assets
- *(host)* plugin settings in .config/ (Phase 13)
- render a real site from CommonMark with templates (Phase 4)
- implement core types, msgpack codec and golden fixtures (Phase 1)

### Fixed

- *(render-html)* theme fixes from review

### Other

- deny unwrap, expect and panic outside tests
- License lsdocs under Apache-2.0 OR MIT
- Scaffold the lsdocs workspace (roadmap Phase 0)
