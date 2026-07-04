# hayahoo

A native Yahoo Finance plugin for Hayashi.

It talks directly to the public Yahoo Finance REST endpoints used by
`waybar-tickers` and `yahooquery`, so no API key is required.

## Install

```hay
import("sheep-farm/hayahoo", as=yahoo)
```

## Functions

### Quotes

```hay
let q = yahoo::quote("AAPL")
let p = yahoo::price("AAPL")
let c = yahoo::currency("PETR4.SA")
let n = yahoo::name("MSFT")

let table = yahoo::quotes(["AAPL", "MSFT", "PETR4.SA", "BTC-USD"])
```

`quote()` returns a dictionary with the raw Yahoo quote response.
`quotes()` returns a DataFrame with common fields for every ticker.

### Historical data

```hay
let hist = yahoo::history("AAPL", {"interval": "1d", "range": "1y"})
let window = yahoo::history("AAPL", {"start": "2023-01-01", "end": "2023-12-31", "interval": "1wk"})
```

Returns a DataFrame with `date`, `open`, `high`, `low`, `close`, `adj_close`, `volume`.

Options:

- `interval`: `1d`, `1wk`, `1mo` (default `1d`)
- `range`: `1mo`, `3mo`, `6mo`, `1y`, `2y`, `5y`, `10y`, `ytd`, `max` (default `1y`)
- `start` / `end`: `YYYY-MM-DD` window; when provided, overrides `range`

### Search

```hay
let results = yahoo::search("apple", {"limit": 10})
let sym = yahoo::search_symbol("apple")
```

`search()` returns a DataFrame of matching tickers, companies, ETFs and indices.
`search_symbol()` returns the best single match symbol.

### Company info

```hay
let info = yahoo::info("AAPL", {})
let stats = yahoo::key_statistics("AAPL")
let fin = yahoo::financial_data("AAPL")
let prof = yahoo::profile("AAPL")
```

`info()` fetches Yahoo `quoteSummary` modules. The default modules are
`summaryProfile,financialData,defaultKeyStatistics,price`.
Use the `modules` option to request others, e.g.:

```hay
let info = yahoo::info("AAPL", {"modules": "incomeStatementHistory,balanceSheetHistory"})
```

## Notes

- Yahoo Finance endpoints are unofficial and may change or rate-limit clients.
- A browser-like `User-Agent` is sent automatically.
- `set_apikey()` exists only for symmetry with `hayfred`; it is a no-op.

## License

MIT
