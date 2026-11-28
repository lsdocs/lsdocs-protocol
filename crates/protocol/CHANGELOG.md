# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/lsdocs/lsdocs-protocol/compare/lsdocs-protocol/v0.1.0...lsdocs-protocol/v0.1.1) - 2026-11-28

### Other

- *(spec)* serve restarts plugins when their config_files change ([#21](https://github.com/lsdocs/lsdocs-protocol/pull/21))

## [0.1.0](https://github.com/lsdocs/lsdocs-protocol/releases/tag/lsdocs-protocol/v0.1.0) - 2026-11-22

### Added

- release automation ([#3](https://github.com/lsdocs/lsdocs-protocol/pull/3))
- *(render-html)* search
- *(host)* plugin settings in .config/ (Phase 13)
- *(host)* result dependencies and site/parse (Phase 12a)
- *(sphinx-addons)* index page, toctree navigation and numbering, rubric (Phase 11)
- *(sphinx-addons)* syntax extensions and Sphinx's directives and roles (Phase 11)
- protocol 1.0-rc.0, finalized spec and docs site (Phase 9)
- *(parser-rst)* reStructuredText parser plugin (Phase 8e)
- content-hash build cache (Phase 7b)
- add site navigation and a sitemap (Phase 6b)
- check links, fill cross-references and add backlinks (Phase 6a)
- render a real site from CommonMark with templates (Phase 4)
- implement core types, msgpack codec and golden fixtures (Phase 1)

### Other

- *(protocol)* rename pipeline methods (Phase 13)
- deny unwrap, expect and panic outside tests
- parallel pipeline, faster node decoding, benchmarks (Phase 7a)
- use ASCII in comments and docs; plan parser-neutral cross-references
- License lsdocs under Apache-2.0 OR MIT
- Scaffold the lsdocs workspace (roadmap Phase 0)
