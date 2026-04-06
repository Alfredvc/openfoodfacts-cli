# openfoodfacts

[![CI](https://github.com/alfredvc/openfoodfacts-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/alfredvc/openfoodfacts-cli/actions/workflows/ci.yml)
[![Latest Release](https://img.shields.io/github/v/release/alfredvc/openfoodfacts-cli)](https://github.com/alfredvc/openfoodfacts-cli/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

A single-binary CLI for the [Open Food Facts](https://world.openfoodfacts.net) API.

[Open Food Facts](https://world.openfoodfacts.net) is a free, open database of food products from around the world — over 6 million products, contributed and maintained by a global community. This CLI wraps its REST API and outputs structured JSON, making it easy to use from scripts, pipelines, and AI agents. No API key or account needed.

## Installation

```bash
curl -fsSL https://raw.githubusercontent.com/alfredvc/openfoodfacts-cli/main/scripts/install.sh | bash
```

Or build from source:

```bash
cargo install --git https://github.com/alfredvc/openfoodfacts-cli
```

## Quick Start

```bash
# Look up a product by barcode
openfoodfacts products get 3017624010701

# Search by category and nutrition grade
openfoodfacts products search --category en:chocolates --nutrition-grade a

# Full-text search with label filter
openfoodfacts products search --query "organic olive oil" --label en:organic

# List all categories
openfoodfacts facets list categories

# Return only specific fields — critical for AI agent usage (fewer tokens)
openfoodfacts --fields product_name,brands,nutriscore_grade products get 3017624010701
```

## Commands

### Global Flags

| Flag | Description |
|------|-------------|
| `--fields f1,f2` | Return only specified fields |
| `--json` | Force compact JSON (default when piped) |

### `products get <barcode>`

Look up a single product by barcode.

```bash
openfoodfacts products get 3017624010701
openfoodfacts products get 3017624010701 --fields product_name,brands,nutriscore_grade,ecoscore_grade
```

**Example output** (Nutella, barcode `3017624010701`):

```json
{
  "code": "3017624010701",
  "product_name": "Nutella",
  "brands": "Ferrero",
  "nutriscore_grade": "e",
  "ecoscore_grade": "b",
  "ingredients_text_en": "Sugar, Palm Oil, Hazelnuts 13%, Skimmed Milk Powder 8.7%, Fat-Reduced Cocoa 7.4%, Emulsifier: Lecithins (Soy), Vanillin",
  "image_url": "https://images.openfoodfacts.org/images/products/301/762/401/0701/front_en.jpg",
  "countries_tags": ["en:france", "en:united-kingdom", "en:united-states"]
}
```

### `products search`

Filter and search the product database.

| Flag | Description |
|------|-------------|
| `--query <text>` | Full-text search |
| `--category <tag>` | e.g. `en:chocolates` |
| `--nutrition-grade <a-e>` | Filter by Nutri-Score |
| `--ecoscore-grade <a-e>` | Filter by Eco-Score |
| `--label <tag>` | e.g. `en:organic` |
| `--ingredient <tag>` | e.g. `en:salt` |
| `--allergen <tag>` | e.g. `en:gluten` |
| `--sort-by <field>` | e.g. `last_modified_t`, `unique_scans_n` |
| `--page <n>` | Page number (default: 1) |
| `--page-size <n>` | Items per page (default: 20, max: 100) |
| `--all` | Fetch all pages, return flat array |

**Example output** (envelope shape):

```json
{
  "count": 4821,
  "page": 1,
  "page_size": 20,
  "products": [
    {
      "code": "3017624010701",
      "product_name": "Nutella",
      "brands": "Ferrero",
      "nutriscore_grade": "e"
    }
  ]
}
```

### `facets list <type>`

Browse a facet dimension. Valid types: `categories`, `labels`, `ingredients`, `brands`, `countries`, `additives`, `allergens`, `packaging`.

```bash
openfoodfacts facets list categories
openfoodfacts --fields id,products facets list labels
```

**Example output**:

```json
[
  { "id": "en:beverages", "name": "Beverages", "products": 182043 },
  { "id": "en:dairies", "name": "Dairies", "products": 97612 },
  { "id": "en:cereals-and-their-products", "name": "Cereals and their products", "products": 74801 }
]
```

## Output

- **Success:** JSON to stdout, exit 0
- **Error:** `{"error": "..."}` to stderr, exit 1
- **TTY:** pretty-printed JSON — readable for humans
- **Piped / redirected:** compact single-line JSON — ready for `jq`, `grep`, log ingestion, or passing to an LLM

The format switches automatically. Use `--json` to force compact output even in a terminal.

### Using `--fields` with AI agents

The full product record can be several kilobytes of nested JSON. Use `--fields` to return only what you need — this directly reduces token cost when passing output to a language model:

```bash
# Instead of the full record (~3 KB):
openfoodfacts products get 3017624010701

# Return only what the agent needs (~150 bytes):
openfoodfacts --fields product_name,brands,nutriscore_grade,ecoscore_grade,ingredients_text_en products get 3017624010701
```

## Rate Limits

Open Food Facts is a free public API with no authentication. The limits are generous for scripted use but worth knowing:

| Endpoint | Limit |
|----------|-------|
| Product lookups | 100 req/min |
| Search | 10 req/min |
| Facets | 2 req/min |

If you need to fetch many products, use `--all` with search rather than individual barcode lookups.

## Agent Skills

Install skills for AI coding agents (Claude Code, Cursor, Gemini CLI, etc.):

```bash
npx skills add alfredvc/openfoodfacts-cli
```

This installs workflow-oriented instruction files that teach agents how to use the CLI. Available skills:

- **openfoodfacts-shared** — Output format, error handling, rate limits, and invocation patterns
- **openfoodfacts-product-lookup** — Look up products by barcode, search with filters, full-text search
- **openfoodfacts-data-exploration** — Discover valid tag IDs via facets, then build filtered searches

See [AGENTS.md](AGENTS.md) for the agent-specific command reference.
