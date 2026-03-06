# Migration Guide: 1.0-beta Public Surface

This guide explains the 1.0-beta cleanup of OMX's public review/analysis surface.

## What changed

The public review/analysis surface is now centered on:
- `analyze`
- `code-review`
- `critic`

### Public entry points
- **`analyze`** — the main public entry point for investigation, diagnosis, and system understanding
- **`code-review`** — the main public entry point for reviewing code changes
- **`critic`** — a **first-class public critique agent** for plans, designs, and approaches

## What stayed public

`critic` remains public and first-class. It is not deprecated, hidden, or reduced to a compatibility wrapper.

## What moved internal

These prompts still exist, but they are no longer the primary public menu for the review/analysis lane:
- `architect`
- `debugger`
- `code-reviewer`
- `security-reviewer`

They remain available for advanced/direct invocation and internal routing.

## Compatibility / deprecated terms

### Public compatibility lanes during beta
- `security-review` — specialist compatibility lane; prefer `code-review` as the primary public review entry
- `review` — deprecated public term; prefer `critic` or `plan --review`

### Hidden compatibility aliases
These remain functional but should no longer be treated as primary discovery terms:
- `style-reviewer`
- `quality-reviewer`
- `api-reviewer`
- `performance-reviewer`

## Old-to-new examples

### Analysis
- old: `/prompts:architect "analyze current auth boundaries"`
- new: `$analyze "analyze current auth boundaries"`

### Code review
- old: `$security-review "review this branch"`
- new: `$code-review "review this branch"`

### Critique
- old: `$review "review this plan"`
- new: `/prompts:critic "challenge this plan"`

## What did NOT change

- `prompts/executor.md` remains untouched in this phase
- direct expert prompt invocation can still work for advanced/internal use

## Why this changed

The goal of 1.0-beta is to reduce public overlap and stop leaking internal expert ontology into the user-facing product.

Users should think in:
- analyze this
- review this code
- critique this plan

rather than having to choose among multiple overlapping expert labels before work even begins.
