# openfoodfacts

Single-binary CLI for the [Open Food Facts](https://world.openfoodfacts.net) API. Designed for AI agent consumption — all output is JSON.

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

# Search by category
openfoodfacts products search --category en:chocolates --nutrition-grade a

# Full-text search
openfoodfacts products search --query "organic olive oil" --label en:organic

# List all categories
openfoodfacts facets list categories

# Return only specific fields (saves tokens for AI agents)
openfoodfacts --fields product_name,brands,nutriscore_grade products get 3017624010701
```

## Commands

### Global Flags

| Flag | Description |
|------|-------------|
| `--fields f1,f2` | Return only specified fields |
| `--json` | Force compact JSON (default when piped) |

### `products get <barcode>`

Look up a single product by barcode string.

```bash
openfoodfacts products get 3017624010701
openfoodfacts products get 3017624010701 --fields product_name,brands,nutriscore_grade,ecoscore_grade
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

### `facets list <type>`

Browse a facet dimension. Valid types: `categories`, `labels`, `ingredients`, `brands`, `countries`, `additives`, `allergens`, `packaging`.

```bash
openfoodfacts facets list categories
openfoodfacts --fields id,products facets list labels
```

## Output

- **Success:** JSON to stdout, exit 0
- **Error:** `{"error": "..."}` to stderr, exit 1
- TTY: pretty-printed JSON; piped: compact JSON

## Rate Limits

Open Food Facts enforces: 100 req/min (product lookups), 10 req/min (search), 2 req/min (facets).
