# CI Integration

## GitHub Actions

```yaml
- uses: sbryngelson/ffmt@latest
  with:
    args: "--check src/"
```

## pre-commit

```yaml
repos:
  - repo: https://github.com/sbryngelson/ffmt
    rev: v0.2.8
    hooks:
      - id: ffmt
```

To auto-update to the latest version:

```bash
pre-commit autoupdate
```

Or use [pre-commit.ci](https://pre-commit.ci/) to update hooks automatically via pull requests.

## GitLab CI

```yaml
format:
  image: python:3.12
  script:
    - pip install ffmt
    - ffmt --check src/
```

## Generic CI

```bash
pip install ffmt
ffmt --check src/
```

The `--check` flag exits with code 1 if any files would be changed, making it suitable for CI gates. Use `--diff` to see what would change:

```bash
ffmt --diff src/
```
