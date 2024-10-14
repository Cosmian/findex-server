- Ignoring lint directives
In some particular cases, linters might prompt warnings that are not suitable for the codebase. Those can be ignored for a particular line by using the #[allow(clippy::lint_name)] attribute directly above the line you want to ignore. However, each ignore should be accompanied with a comment explaining the motive of ignoring it.
