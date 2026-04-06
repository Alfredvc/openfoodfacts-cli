---
name: openfoodfacts-product-lookup
description: Look up food products by barcode or search with filters. Find products by category, nutrition grade, eco-score, label, ingredient, or allergen. Use when you have a barcode or need to find products matching specific criteria.
compatibility: Requires openfoodfacts binary installed (see openfoodfacts-shared skill).
---

# openfoodfacts-product-lookup

Workflow for finding food product data from the Open Food Facts database.

## Look Up a Product by Barcode

```bash
openfoodfacts --json products get <barcode>
```

Example (Nutella):

```bash
openfoodfacts --json products get 3017624010701
```

Use `--fields` to return only what you need:

```bash
openfoodfacts --json --fields product_name,brands,nutriscore_grade,ecoscore_grade,ingredients_text_en products get 3017624010701
```

If the product is not found, the CLI exits 1 with `{"error": "product not found: <barcode>"}`.

## Search by Filters

Filter-only search (no full-text query) uses the v2 API:

```bash
openfoodfacts --json products search --category en:chocolates --nutrition-grade a
```

Available filter flags:

| Flag | Description | Example |
|------|-------------|---------|
| `--category <tag>` | Category tag | `en:chocolates` |
| `--nutrition-grade <a-e>` | Nutri-Score | `a` |
| `--ecoscore-grade <a-e>` | Eco-Score | `b` |
| `--label <tag>` | Label tag | `en:organic` |
| `--ingredient <tag>` | Ingredient tag | `en:salt` |
| `--allergen <tag>` | Allergen tag | `en:gluten` |

Combine filters freely — all are ANDed:

```bash
openfoodfacts --json products search --category en:biscuits-and-cakes --nutrition-grade a --label en:organic
```

## Full-Text Search

Use `--query` for keyword search (routes to the v1 search endpoint):

```bash
openfoodfacts --json products search --query "organic olive oil"
```

Combine with filters:

```bash
openfoodfacts --json products search --query "dark chocolate" --nutrition-grade b --label en:fair-trade
```

## Pagination

Default: 20 results per page. The response envelope tells you how many pages exist:

```json
{"count": 4821, "page": 1, "page_count": 242, "page_size": 20, "products": [...]}
```

Fetch a specific page:

```bash
openfoodfacts --json products search --category en:chocolates --page 3 --page-size 50
```

Fetch all pages at once (returns flat array — use with care on large result sets):

```bash
openfoodfacts --json products search --category en:chocolates --nutrition-grade a --all
```

## Tips

- Tag values are always prefixed with a language code: `en:chocolates`, not `chocolates`. Use `facets list` to discover valid tags.
- Use `--fields code,product_name,nutriscore_grade` on search results to keep response sizes small.
- `--all` can trigger many API calls. Check `page_count` in a regular search first.
- Rate limit for search is 10 req/min — avoid tight loops.
