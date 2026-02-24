# Instruction precedence and structure (research summary)

This reference captures practical structure guidance based on GitHub Copilot custom-instruction docs and agent-instruction patterns.

## Recommended layering
1. `.github/copilot-instructions.md`
   - Repository-wide Copilot guidance.
2. `.github/instructions/*.instructions.md`
   - Path-scoped, `applyTo`-based instructions for language or area-specific rules.
3. `AGENTS.md` files
   - Agent-specific operational policy; nearest file in tree takes precedence.
4. `skills/<name>/SKILL.md`
   - Repeatable, domain-specific workflows for autonomous agents.

## Practical implications
- Keep repository-wide files short and durable.
- Put fast-changing, path-specific rules in `.github/instructions` files.
- Avoid duplicating long policy text across all instruction surfaces.
- Keep explicit validation commands in all layers where code generation occurs.

## External references used
- GitHub Docs: *Adding repository custom instructions for GitHub Copilot* (describes repository-wide, path-specific, and agent instruction types).
- GitHub Docs: *Support for different types of custom instructions* (feature support matrix).
- GitHub Docs reusable guidance: *Writing effective custom instructions* (keep instructions concise and broadly applicable).
- OpenAI `agents.md` convention linked from GitHub docs.
