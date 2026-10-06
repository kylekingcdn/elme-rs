# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/kylekingcdn/elme-rs/compare/elme-shutdown-v0.1.0...elme-shutdown-v0.1.1) - 2026-10-06

### Build

- *(feature-flags)* fix additive deps break

### Chore

- version bumps
- *(refactor)* decrease teardown HookDeps params

### Doc

- *(examples)* fix doctests, update deps
- formatting
- *(roadmap)* add v0.2.0 to roadmap, clear v0.1.0

### Feat

- *(config)* impl full progress disable support
- *(progress)* add progress toggle to config
- *(deps)* serde-optional support for `shutdown`

### Fix

- *(reload)* rename `reload_enabled`, fix usage
- *(example)* adjust log message formatting
