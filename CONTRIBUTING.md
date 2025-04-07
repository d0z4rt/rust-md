# Contribute

### ⚠️ You MUST use [Conventional Commits](https://d0z.eu/brain/notes/Conventional_Commits)

> We use an automated versioning process that rely on commit messages for the version bump and changelog.

## Commit Message Format

```js
<type>(<scope>): <message>
```

```js
<type>(<scope>): <subject>
<BLANK_LINE>
<body>
<BLANK_LINE>
<footer>
```

## Types

- `feat`: _A new feature_
- `fix`: _A bug fix or code update_
- `docs`: _Documentation only changes_
- `build`: _Changes that affect the build system or external dependencies (example scopes: gulp, broccoli, npm)_
- `style`: _Changes that do not affect the meaning of the code (white-space, formatting, missing semi-colons, etc)_
- `refactor`: _A code change that neither fixes a bug nor adds a feature_
- `perf`: _A code change that improves performance_
- `test`: _Adding missing or correcting existing tests_
- `chore`: _Editing comments or README_

### Scopes

- `brain`
- `d0z`
- `...`

## BREAKING CHANGES

```js
<type>(<scope>)!: <message>
```

```js
<type>(<scope>)!: <subject>
<BLANK_LINE>
<body>
<BLANK_LINE>
BREAKING CHANGE: <explanation>
```

## Examples

```js
feat(layer-ms): add method to update layer
```

```js
fix(layer-ms)!: fix layers imports

add a new thing to that special file
remove that from this

BREAKING CHANGE: an id need to be specified when you call a layer
```
