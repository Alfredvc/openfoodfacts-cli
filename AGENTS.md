# openfoodfacts CLI — Agent Reference

All output is JSON. Errors go to stderr as `{"error": "..."}` with exit code 1.

## Commands

### products get <barcode>
```
openfoodfacts products get <barcode-string>
openfoodfacts --fields product_name,brands,nutriscore_grade products get <barcode>
```
Returns the product object. Exit 1 with `{"error": "product not found: <barcode>"}` if not found.

### products search
```
openfoodfacts products search [flags...]
openfoodfacts --fields code,product_name products search --category en:chocolates --nutrition-grade a
```
Returns `{"count":N,"page":N,"page_count":N,"page_size":N,"skip":N,"products":[...]}`.
`--all` returns flat array `[...]` of all pages.

Flags: `--query`, `--category`, `--nutrition-grade`, `--ecoscore-grade`, `--label`, `--ingredient`, `--allergen`, `--sort-by`, `--page`, `--page-size`, `--all`

### facets list <type>
```
openfoodfacts facets list <type>
```
Type: `categories` `labels` `ingredients` `brands` `countries` `additives` `allergens` `packaging`
Returns array of `{"id":"en:...","name":"...","products":N}`.

## Global Flags
- `--fields f1,f2` — filter output fields (envelope preserved on paginated results)
- `--json` — compact output

## Rate Limits
100/min product · 10/min search · 2/min facets
