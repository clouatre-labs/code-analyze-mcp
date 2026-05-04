# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- GitHub Actions workflow for automated issue triage using [aptu](https://github.com/clouatre-labs/aptu) (pinned to `9cdcf9ac` = v0.5.1)
- GitHub Actions workflow for automated PR review using aptu (pinned to `9cdcf9ac` = v0.5.1) with Gemini primary and OpenRouter Mercury-2 fallback

### Changed

- Replaced GitHub Copilot code review with aptu-powered PR review; the `copilot_code_review` rule has been removed from the `Protect main branch` ruleset
