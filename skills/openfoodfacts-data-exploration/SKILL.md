---
name: openfoodfacts-data-exploration
description: Explore the Open Food Facts database structure. Discover valid category, label, ingredient, brand, and allergen tags using facets, then use them as filters in product search. Use when you don't know the exact tag names to filter by.
compatibility: Requires openfoodfacts binary installed (see openfoodfacts-shared skill).
---

# openfoodfacts-data-exploration

Workflow for exploring the Open Food Facts database when you don't know which tags to filter by.

## Step 1: List a Facet Dimension

Facets show you the valid tag values for each filter dimension, along with how many products use each tag.

Valid facet types: `categories`, `labels`, `ingredients`, `brands`, `countries`, `additives`, `allergens`, `packaging`

```bash
openfoodfacts --json facets list categories
```

Response shape:

```json
[
  {"id": "en:beverages", "name": "Beverages", "products": 182043},
  {"id": "en:dairies", "name": "Dairies", "products": 97612},
  {"id": "en:cereals-and-their-products", "name": "Cereals and their products", "products": 74801}
]
```

The `id` field is the tag value to use in `products search`.

## Step 2: Filter to Relevant Tags

Use `--fields` to reduce output, then filter with `jq`:

```bash
openfoodfacts --json --fields id,name,products facets list categories | jq '[.[] | select(.name | test("chocolate"; "i"))]'
```

Or sort by product count to find the most-used tags:

```bash
openfoodfacts --json facets list categories | jq 'sort_by(-.products) | .[:20]'
```

## Step 3: Search Products Using Discovered Tags

Once you have the tag ID:

```bash
openfoodfacts --json products search --category en:chocolates
```

Combine multiple dimensions:

```bash
openfoodfacts --json products search --category en:chocolates --label en:organic --nutrition-grade a
```

## Explore Other Dimensions

### Find allergen-free products

```bash
# List all allergen tags
openfoodfacts --json facets list allergens

# Then find products without a specific allergen (note: allergen filter matches products CONTAINING the allergen)
openfoodfacts --json products search --category en:biscuits-and-cakes --allergen en:gluten
```

### Discover labels (certifications)

```bash
openfoodfacts --json facets list labels | jq '[.[] | select(.products > 1000)] | sort_by(-.products)'
```

### Find products by brand

```bash
# Check how many products a brand has
openfoodfacts --json --fields id,name,products facets list brands | jq '[.[] | select(.name | test("ferrero"; "i"))]'
```

## Rate Limit Note

Facet endpoints are limited to 2 req/min. Cache results if you need to query multiple dimensions — the facet data changes slowly.
