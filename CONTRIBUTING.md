# Contributing

Below are the conventions that are used in the repository and that should be respected when working on the repository. Please respect them if you are are submitting code, and try to make sure of their enforcement if you a reviewer.

## Code Style and Linting

### Clippy Directives

- All `#[allow(clippy::...)]` directives must be accompanied by an explanatory comment justifying the bypass. Example:

  ```rust
  #[allow(clippy::cognitive_complexity)] // Allowing this because the cognitive complexity cannot be reduced further
  fn complex_domain_logic() {
      // ...
  }
  ```

## Pull Request Process

This projects follow the [git-flow branching model](https://git-flow.readthedocs.io/fr/latest/presentation.html).
To contribute a new feature, pull the `develop` branch locally and submit a PR to be merged into `develop` then request review from at least one team member.

## License

By contributing to this project, you agree that your contributions will be licensed under the project's license.
