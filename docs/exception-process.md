# Exception Process

This document defines how Cogolo handles exceptions to its normal engineering and governance rules.

## Purpose

Exceptions exist to make unusual cases explicit, reviewable, and temporary where possible.

An exception is not silent permission to ignore a rule.

## Exceptions Requiring Review

Review is required for exceptions involving:

- portability or host coupling
- unsafe or privileged code paths
- spec-alignment or merge-gating deviations
- reduced coverage for core logic
- compatibility rule exceptions
- bypass of normal contract, policy, constraint, or trace handling

## Required Exception Content

Every exception should document:

- title
- affected rule
- reason
- scope
- owner
- risk
- mitigation
- review date or expiry expectation

## Review Expectations

- Exceptions should be approved before merge.
- Exceptions should be narrow in scope.
- Exceptions should be revisited when their review date arrives.
- Expired or obsolete exceptions should be removed.

## Default Rule

If a change requires an exception and no approved exception exists, the change should not merge.
