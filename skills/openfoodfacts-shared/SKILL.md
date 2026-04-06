---
name: openfoodfacts-shared
description: Runtime contract for the openfoodfacts CLI. Covers output format, error handling, field filtering, and invocation patterns. No authentication required. Use as foundation before any other openfoodfacts skill.
compatibility: Requires openfoodfacts binary installed and network access.
---

# openfoodfacts-shared

Foundation skill for the openfoodfacts CLI. Read this before using any other openfoodfacts skill.

## No Authentication Required

The Open Food Facts API is fully public. No API key, no login, no environment variables needed.

## Invocation Pattern

Always use `--json` to ensure compact, parseable output:

```bash
openfoodfacts --json <command>
```

Use `--fields` to reduce output to only the fields you need (saves tokens):

```bash
openfoodfacts --json --fields product_name,brands,nutriscore_grade products get 3017624010701
```

## Output Format

- **Success**: JSON to stdout, exit code 0
- **Error**: JSON to stderr (`{"error": "message"}`), exit code 1

Search commands without `--all` return a paginated envelope:

```json
{"count": 4821, "page": 1, "page_count": 242, "page_size": 20, "skip": 0, "products": [...]}
```

With `--all`, they return a flat array of all products across all pages:

```json
[{"code": "...", "product_name": "..."}, ...]
```

Single product lookups return the product object directly (no envelope).

## Error Handling

Always check exit code. On failure, parse stderr for the error message:

```bash
result=$(openfoodfacts --json products get 0000000000000 2>err.tmp) || {
  error=$(cat err.tmp)
  # handle error
}
```

## Rate Limits

| Command | Limit |
|---------|-------|
| `products get` | 100 req/min |
| `products search` | 10 req/min |
| `facets list` | 2 req/min |

If you exceed limits, the CLI exits 1 with `{"error": "rate limit exceeded"}`.

## Available Commands

- Products: `openfoodfacts products get <barcode>` · `openfoodfacts products search [flags...]`
- Facets: `openfoodfacts facets list <type>`
