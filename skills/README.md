# Local Skills (Repository Scaffold)

This folder is reserved for project-specific skills used by autonomous agents.

## Why this exists
- Keep repeatable, domain-specific workflows close to the codebase.
- Standardize how agents perform diagnostics, docs updates, and release chores.

## Suggested structure
- `skills/<skill-name>/SKILL.md` — required entrypoint.
- `skills/<skill-name>/references/` — concise reference docs.
- `skills/<skill-name>/scripts/` — helper scripts invoked by skill steps.
- `skills/<skill-name>/assets/` — reusable templates/checklists.

## Minimal `SKILL.md` template
```md
# <skill-name>

## Use when
Describe the trigger conditions.

## Inputs
List required context/parameters.

## Steps
1. ...
2. ...

## Validation
Commands/checks to run.

## Output
Expected artifacts and summary format.
```

## Notes
- Keep skills narrowly scoped.
- Avoid embedding large, duplicated docs; link to canonical files.
- Prefer scripts for repeatable procedures.
