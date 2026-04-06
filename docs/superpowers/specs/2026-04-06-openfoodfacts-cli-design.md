# openfoodfacts CLI — Design Spec

**Date:** 2026-04-06  
**Binary:** `openfoodfacts`  
**Scope:** Read-only agent-friendly CLI wrapping the Open Food Facts API

---

## 1. Purpose

A single-binary Rust CLI for AI agents to look up food product data and explore the Open Food Facts database. Outputs structured JSON to stdout on every command. No authentication required — the API is fully public for read operations.

---

## 2. API Facts (verified against live API)

| Property | Value |
|----------|-------|
| Base URL | `https://world.openfoodfacts.net` |
| Product lookup | `GET /api/v2/product/{barcode}` |
| Filter search | `GET /api/v2/search` |
| Full-text search | `GET /cgi/search.pl?json=1&search_terms=…` (v2 does not support full-text) |
| Facet listing | `GET /{facet_type}.json` (e.g. `/categories.json`) |
| Auth | None required for read operations |
| Product IDs | Barcode strings (e.g. `"3017624010701"`) — never integers |
| Rate limits | 100/min product lookups · 10/min search · 2/min facets |
| Required header | `User-Agent: openfoodfacts-cli/VERSION (https://github.com/alfredvc/openfoodfacts-cli)` |

**Search parameter names (verified):**
- `categories_tags` — e.g. `en:chocolates`
- `nutrition_grades_tags` — `a`–`e`
- `labels_tags` — e.g. `en:organic`
- `ecoscore_tags` — `a`–`e`
- `allergens_tags` — e.g. `en:gluten`
- `ingredients_tags` — e.g. `en:salt`
- `sort_by` — e.g. `last_modified_t`, `unique_scans_n`
- `fields` — comma-separated field names
- `page`, `page_size`

**Search response envelope (verified):**
```json
{"count": 27369, "page": 1, "page_count": 1369, "page_size": 20, "skip": 0, "products": [...]}
```

**Facet response shape (verified):**
```json
{"count": 72718, "tags": [{"id": "en:chocolates", "name": "Chocolates", "products": 4821, "url": "...", "known": 1}]}
```

---

## 3. Command Structure

```
openfoodfacts
├── products
│   ├── get <barcode>     # Look up one product by barcode string
│   └── search            # Filter/search the product database
└── facets
    └── list <type>       # Browse a facet dimension
```

**Global flags (all commands):**
- `--fields f1,f2,f3` — return only the specified fields (token efficiency for agents)
- `--json` — force compact JSON (default when stdout is piped; pretty-printed in TTY)

### 3.1 `products get <barcode>`

Fetches a single product. Returns the `product` object directly. The API wrapper fields (`status`, `status_verbose`) are stripped.

```
openfoodfacts products get 3017624010701
openfoodfacts products get 3017624010701 --fields product_name,brands,nutriscore_grade,ecoscore_grade
```

If the API returns `status: 0` (product not found), the CLI exits 1 with:
```json
{"error": "product not found: 0000000000000"}
```

### 3.2 `products search`

**Endpoint routing:**
- Filter-only (no `--query`): `GET /api/v2/search` with `categories_tags`, `labels_tags`, etc.
- Full-text only (`--query` alone): `GET /cgi/search.pl?json=1&search_terms=<q>`
- Combined (`--query` + filters): `GET /cgi/search.pl?json=1&search_terms=<q>` with tag filters expressed in v1 syntax. Each CLI filter flag maps to one `tagtype_N/tag_contains_N/tag_N` triplet (N starting at 0, incrementing per flag, `tag_contains_N` always `contains`).

**Verified v1 tagtype values (confirmed against live API):**

| CLI flag | v1 tagtype | v2 param |
|----------|-----------|----------|
| `--category` | `categories` | `categories_tags` |
| `--nutrition-grade` | `nutrition_grades` | `nutrition_grades_tags` |
| `--ecoscore-grade` | `ecoscore_grade` | `ecoscore_tags` |
| `--label` | `labels` | `labels_tags` |
| `--allergen` | `allergens` | `allergens_tags` |
| `--ingredient` | `ingredients` | `ingredients_tags` |

Note: the v2 search endpoint does not support full-text search.

**Flags:**
```
--query <text>              Full-text search (routes to v1 /cgi/search.pl)
--category <tag>            categories_tags filter (e.g. en:chocolates)
--nutrition-grade <a-e>     nutrition_grades_tags filter
--ecoscore-grade <a-e>      ecoscore_tags filter
--label <tag>               labels_tags filter (e.g. en:organic)
--ingredient <tag>          ingredients_tags filter (e.g. en:salt)
--allergen <tag>            allergens_tags filter (e.g. en:gluten)
--sort-by <field>           sort_by parameter (default: none)
--page <n>                  page number (default: 1)
--page-size <n>             items per page (default: 20, max: 100)
--all                       fetch all pages, return flat array
```

**Default output** (pagination envelope preserved):
```json
{"count": 1423, "page": 1, "page_count": 72, "page_size": 20, "products": [...]}
```

**`--all` output** (flat array, all pages merged):
```json
[{...}, {...}, ...]
```

**`--fields` with search:** filters fields inside each item in `products[]`. The pagination envelope fields (`count`, `page`, `page_count`, `page_size`, `skip`) are always preserved regardless of `--fields`.

### 3.3 `facets list <type>`

Valid types: `categories`, `labels`, `ingredients`, `brands`, `countries`, `additives`, `allergens`, `packaging`

URL pattern: `GET /{type}.json`

Output: the `tags` array from the API response (count is dropped; agents can derive it from array length).

```json
[
  {"id": "en:chocolates", "name": "Chocolates", "products": 4821},
  ...
]
```

Unknown type produces:
```json
{"error": "unknown facet type: \"foo\" — valid: categories, labels, ingredients, brands, countries, additives, allergens, packaging"}
```

---

## 4. Output Contract

| Condition | Destination | Exit code | Format |
|-----------|------------|-----------|--------|
| Success | stdout | 0 | JSON (pretty in TTY, compact when piped) |
| Error | stderr | 1 | `{"error": "..."}` |

No spinners, progress bars, colors, or human-formatted tables. Ever.

---

## 5. Architecture

```
src/
├── main.rs           # Entry point: parse CLI args, dispatch, handle top-level errors
├── cli.rs            # All clap derive structs (Commands, ProductsCommand, etc.)
├── client.rs         # HTTP client: GET helper, User-Agent injection, error mapping, rate limit detection
├── output.rs         # JSON formatting (TTY detection), field filtering
└── commands/
    ├── mod.rs
    ├── products.rs   # products get + search
    └── facets.rs     # facets list
```

**Dependencies:**

| Crate | Purpose |
|-------|---------|
| `clap 4` (derive) | CLI argument parsing |
| `reqwest` (async, rustls-tls) | HTTP — no OpenSSL dependency |
| `tokio` (rt-multi-thread, macros) | Async runtime |
| `serde` + `serde_json` | JSON deserialization + formatting |
| `anyhow` | Error context |

No config file, no auth module — the API needs neither.

---

## 6. HTTP Client Design

All requests go through a single `Client::get(url, params)` method that:
1. Injects the `User-Agent` header
2. Executes the request
3. Maps non-2xx responses to typed errors (404 → not-found, 429/403 → rate-limit, 5xx → server error)
4. Returns `serde_json::Value`

Rate limit errors surface as: `{"error": "rate limit exceeded — max 100 req/min for product lookups"}`

---

## 7. Testing Strategy

**Layer 1 — Unit tests:**
- `output.rs`: field filtering on objects, arrays, and paginated envelopes
- TTY detection logic

**Layer 2 — Integration tests (wiremock):**
- `products get` success, not-found, malformed barcode
- `products search` with filters, with `--query` (routes to v1 endpoint), with `--all` (multi-page)
- `products search --fields` preserves pagination envelope, filters items
- `facets list` success, unknown type error
- Mock data uses shapes taken from real API responses (verified in Phase 0)

**Layer 3 — Smoke tests (manual / CI with real API):**
- Run each command once against `world.openfoodfacts.net` before release

---

## 8. Distribution

- GitHub Releases with 4 pre-built targets: `x86_64-linux`, `aarch64-linux`, `x86_64-macos`, `aarch64-macos`
- Install script defaulting to `~/.local/bin`
- Release workflow patches `Cargo.toml` version from git tag before building
- `curl -f` in install script to fail fast on HTTP errors

No self-update command (no auth, no agent workflow requires staying current on a schedule — users can re-run the install script).

---

## 9. Documentation

- `README.md` — human-facing: installation, quick start, full command reference
- `AGENTS.md` (symlinked as `CLAUDE.md`) — same content, optimized for agent parsing
