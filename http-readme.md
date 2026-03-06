# HTTP REST API Documentation

## Base URL

```
http://localhost:12000/api/v1
```

All endpoints return JSON responses.

---

## Market Data Endpoints

### 1. Get Exchange Info

Get information about all available markets.

**Endpoint:** `GET /exchangeInfo`

**Request:**
```bash
curl http://localhost:12000/api/v1/exchangeInfo
```

**Response:**
```json
[
  {
    "symbol": "BTC-USD",
    "baseAsset": "BTC",
    "quoteAsset": "USDC",
    "status": "TRADING",
    "pricePrecision": 1,
    "sizePrecision": 8,
    "tickSize": 0.5,
    "lotSize": 0.001,
    "minNotional": 10.0,
    "maxLeverage": 10,
    "orderTypes": ["LIMIT", "MARKET", "STOP", "STOP_LIMIT", "TAKE_PROFIT"],
    "timeInForces": ["GTC", "IOC"]
  }
]
```

---

### 2. Get Ticker / Market Statistics

Get 24-hour statistics for a specific symbol.

**Endpoint:** `GET /ticker/{symbol}`

**Request:**
```bash
curl http://localhost:12000/api/v1/ticker/BTC-USD
```

**Response:**
```json
{
  "symbol": "BTC-USD",
  "priceChange": 2777.5,
  "priceChangePercent": 2.77,
  "lastPrice": 102777.5,
  "highPrice": 103500.0,
  "lowPrice": 101000.0,
  "volume": 1234.56,
  "quoteVolume": 126543210.0,
  "markPrice": 102780.0,
  "oraclePrice": 102775.0,
  "openInterest": 5432.1,
  "fundingRate": 0.0001,
  "regime": 1,
  "regimeDt": 93,
  "regimeVol": 0.35,
  "regimeMv": 0.02,
  "fairBookPx": 102779.0,
  "fairVol": 0.28,
  "fairBias": 0.001,
  "timestamp": 1704067200000000000
}
```

**Ticker Fields:**
- `priceChange`: Absolute price change over 24h
- `priceChangePercent`: Percentage price change over 24h
- `lastPrice`: Last traded price
- `highPrice`: 24h high
- `lowPrice`: 24h low
- `volume`: 24h base asset volume
- `quoteVolume`: 24h quote asset volume
- `markPrice`: Current fair/mark price
- `oraclePrice`: Oracle-reported price
- `openInterest`: Total open interest
- `fundingRate`: Current funding rate
- `regime`: Market regime indicator (-1, 0, 1)
- `regimeDt`: Regime duration in 10s intervals
- `regimeVol`: Regime-adjusted volatility
- `regimeMv`: Regime mean value
- `fairBookPx`: Fair price derived from order book
- `fairVol`: Fair volatility estimate
- `fairBias`: Fair price bias
- `timestamp`: Timestamp (nanoseconds)

---

### 3. Get Candle History / Klines

Get OHLCV candlestick data for charting.

**Endpoint:** `GET /klines`

**Query Parameters:**
- `symbol` (String, required): Market symbol
- `interval` (String, required): Timeframe (10s, 1m, 3m, 5m, 15m, 30m, 1h, 2h, 4h, 6h, 8h, 12h, 1d, 3d, 1w, 1M)
- `startTime` (Number, optional): Start timestamp in milliseconds
- `endTime` (Number, optional): End timestamp in milliseconds

**Request:**
```bash
curl "http://localhost:12000/api/v1/klines?symbol=BTC-USD&interval=1m"
```

**Response:**
```json
[
  {
    "t": 1699564800000,
    "T": 1699564860000,
    "o": 102000.0,
    "h": 102100.0,
    "l": 101900.0,
    "c": 101950.0,
    "v": 1.6,
    "n": 3
  }
]
```

**Candle Fields:**
- `t`: Open timestamp (milliseconds)
- `T`: Close timestamp (milliseconds)
- `o`: Open price
- `h`: High price
- `l`: Low price
- `c`: Close price
- `v`: Volume
- `n`: Number of trades

---

### 4. Get Exchange Stats

Get aggregate exchange statistics across all markets.

**Endpoint:** `GET /stats`

**Query Parameters:**
- `period` (String, optional): Time period - "1d", "7d", "30d", "90d", "1y", "all" (default: "1d")
- `period` aliases accepted: `24h`, `1w`, `1m`, `3m`, `365d` (echoed back as canonical value)
- `symbol` (String, optional): Filter to single market (e.g., "BTC-USD")

**Request:**
```bash
curl http://localhost:12000/api/v1/stats
```

**Request (filtered):**
```bash
curl "http://localhost:12000/api/v1/stats?symbol=BTC-USD&period=7d"
```

**Request (alias period):**
```bash
curl "http://localhost:12000/api/v1/stats?period=24h"
```

**Response:**
```json
{
  "timestamp": 1704067200000,
  "period": "1d",
  "volume": {
    "totalUsd": 126543210.0
  },
  "openInterest": {
    "totalUsd": 54321000.0
  },
  "funding": {
    "rates": {
      "BTC-USD": {
        "current": 0.0001,
        "annualized": 0.1095
      },
      "ETH-USD": {
        "current": 0.00008,
        "annualized": 0.0876
      }
    }
  },
  "markets": [
    {
      "symbol": "BTC-USD",
      "volume": 1234.56,
      "quoteVolume": 126543210.0,
      "openInterest": 50000000.0,
      "fundingRate": 0.0001,
      "fundingRateAnnualized": 0.1095,
      "lastPrice": 102500.0,
      "markPrice": 102480.0
    }
  ]
}
```

**Behavior Notes:**
- Aggregate queries (`symbol` omitted) are cached with a 600s TTL.
- Symbol-specific queries bypass cache.
- `period` is echoed in the response (`1d`, `7d`, `30d`, `90d`, `1y`, `all`).
- Unknown `symbol` returns `200` with empty `markets` and zero totals (not `404`).

---

### 5. Runtime Metrics

Get the runtime metrics snapshot JSON.

**Endpoints:** `GET /metrics` and `GET /api/v1/metrics`

**Request:**
```bash
curl http://localhost:12000/metrics
```

**Response:**
```json
{
  "node_id": 0,
  "timestamp_unix_ms": 1704067200000,
  "latency_stats": { "count": 100, "p95_ms": 3.2 }
}
```

If callback wiring is disabled:
```json
{"error":"metrics not configured"}
```

---

### 6. Verification

Compare ledger verification values with committed metrics values.

**Endpoints:** `GET /verify` and `GET /api/v1/verify`

**Request:**
```bash
curl http://localhost:12000/verify
```

**Response:**
```json
{
  "ledger": {
    "tx_count": 1200,
    "tx_xor": "0f3a9b10e3a319ab",
    "latest_round": 245
  },
  "metrics": {
    "committed_count": 1200,
    "committed_xor": "0f3a9b10e3a319ab"
  },
  "match": true
}
```

---

### 7. Get L2 Order Book

Get current order book snapshot with optional filtering.

**Endpoint:** `GET /l2book`

**Query Parameters:**
- `type` (String, required): Must be "l2Book"
- `coin` (String, required): Market symbol
- `nlevels` (Number, optional): Number of price levels to return
- `aggregation` (Number, optional): Price increment for aggregation

**Request:**
```bash
curl "http://localhost:12000/api/v1/l2book?type=l2Book&coin=BTC-USD&nlevels=10&aggregation=0.5"
```

**Response:**
```json
{
  "updateType": "snapshot",
  "symbol": "BTC-USD",
  "levels": [
    [
      {"px": 102777.0, "sz": 1.5, "n": 3},
      {"px": 102776.5, "sz": 2.3, "n": 5}
    ],
    [
      {"px": 102780.0, "sz": 2.0, "n": 4},
      {"px": 102780.5, "sz": 1.2, "n": 3}
    ]
  ],
  "timestamp": 1704067200000000000
}
```

**Level Fields:**
- `px`: Price
- `sz`: Total size at this price
- `n`: Number of orders at this price

**Levels Array:**
- Index 0: Bids (highest to lowest)
- Index 1: Asks (lowest to highest)

---

## Transaction Endpoint

All state-mutating operations use a **unified transaction model** submitted to a single endpoint.

**Endpoint:** `POST /order`

### Transaction Structure

```json
{
  "actions": [Action, ...],
  "nonce": 1704067200000,
  "account": "base58_pubkey",
  "signer": "base58_pubkey",
  "signature": "base58_signature"
}
```

**Fields:**
- `actions`: Array of actions to execute atomically (see Action Types below)
- `nonce`: Unique integer for replay protection (use timestamp in nanoseconds or incrementing counter)
- `account`: Account public key (base58) - the account performing the action
- `signer`: Signer public key (base58) - who is signing (usually same as account, or authorized agent)
- `signature`: Ed25519 signature (base58)

### Action Types

Each action in the `actions` array is a tagged object. The key is the compact action tag, and the value contains the action fields.

#### `l` (LimitOrder)

Place a resting limit order.

```json
{"l": {"c": "BTC-USD", "b": true, "px": 100000.0, "sz": 0.1, "tif": "GTC", "r": false}}
```

| Field | Type | Description |
|-------|------|-------------|
| `c` | String | Symbol (e.g., "BTC-USD") |
| `b` | bool | true = buy, false = sell |
| `px` | f64 | Limit price |
| `sz` | f64 | Size / quantity |
| `tif` | String | Time in force: "GTC", "IOC", or "ALO" |
| `r` | bool | Reduce only |

**Time In Force:**
- `GTC` - Good Till Cancel (rests on book)
- `IOC` - Immediate or Cancel (fill or kill, no resting)
- `ALO` - Add Liquidity Only (post-only, maker; rejects if would cross)

#### `m` (MarketOrder)

Execute at market price immediately.

```json
{"m": {"c": "BTC-USD", "b": true, "sz": 0.1, "r": false}}
```

| Field | Type | Description |
|-------|------|-------------|
| `c` | String | Symbol |
| `b` | bool | true = buy, false = sell |
| `sz` | f64 | Size / quantity |
| `r` | bool | Reduce only |

#### `mod` (ModifyOrder)

Modify the size of an existing resting order.

```json
{"mod": {"oid": "base58_hash", "symbol": "BTC-USD", "amount": 0.05}}
```

| Field | Type | Description |
|-------|------|-------------|
| `oid` | String | Order ID (base58 hash) |
| `symbol` | String | Symbol |
| `amount` | f64 | New order size |

#### `cx` (Cancel)

Cancel a specific order by ID.

```json
{"cx": {"c": "BTC-USD", "oid": "base58_hash"}}
```

| Field | Type | Description |
|-------|------|-------------|
| `c` | String | Symbol |
| `oid` | String | Order ID (base58 hash) |

#### `cxa` (CancelAll)

Cancel all orders, optionally filtered by symbol.

```json
{"cxa": {"c": ["BTC-USD"]}}
```

| Field | Type | Description |
|-------|------|-------------|
| `c` | String[] | Symbols to cancel. Empty array `[]` cancels all symbols |

#### Faucet

Request testnet funds.

```json
{"faucet": {"u": "base58_pubkey", "amount": 10000.0}}
```

| Field | Type | Description |
|-------|------|-------------|
| `u` | String | Recipient public key (base58) |
| `amount` | f64? | Amount to deposit (optional) |

#### AgentWalletCreation

Register or remove an agent wallet for automated trading.

```json
{"agentWalletCreation": {"a": "base58_pubkey", "d": false}}
```

| Field | Type | Description |
|-------|------|-------------|
| `a` | String | Agent public key (base58) |
| `d` | bool | true = remove agent, false = add agent |

#### UpdateUserSettings

Update per-symbol leverage settings.

```json
{"updateUserSettings": {"m": [["BTC-USD", 5.0], ["ETH-USD", 3.0]]}}
```

| Field | Type | Description |
|-------|------|-------------|
| `m` | [String, f64][] | Array of `[symbol, max_leverage]` tuples |

---

### Admin Action Types

These actions require admin privileges and are used for exchange operations.

#### WhitelistFaucet

Whitelist or un-whitelist an account for faucet access.

```json
{"whitelistFaucet": {"target": "base58_pubkey", "whitelist": true}}
```

| Field | Type | Description |
|-------|------|-------------|
| `target` | String | Target account public key (base58) |
| `whitelist` | bool | true = whitelist, false = un-whitelist |

#### `px` (Price)

Submit a price update (admin/oracle feeder only).

```json
{"px": {"t": 1704067200000000000, "c": "BTC-USD", "px": 102500.0}}
```

| Field | Type | Description |
|-------|------|-------------|
| `t` | u64 | Timestamp (nanoseconds) |
| `c` | String | Asset symbol |
| `px` | f64 | Oracle price |

#### `o` (PythOracle)

Submit a batch of Pyth oracle price updates (admin/oracle feeder only).

```json
{"o": {"oracles": [{"t": 1704067200000000000, "fi": 1, "px": 10250000000000, "e": -8}]}}
```

**PythOracle Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `oracles` | PythPrice[] | Array of Pyth price updates |

**PythPrice Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `t` | u64 | Timestamp (nanoseconds) |
| `fi` | u64 | Pyth feed ID |
| `px` | u64 | Price (raw integer, apply exponent) |
| `e` | i16 | Price exponent (e.g., -8 means price * 10^-8) |

---

### Response Format

All transaction submissions return an `OrderResponse`:

```json
{
  "status": "ok",
  "response": {
    "type": "order",
    "data": {
      "statuses": [Status, ...]
    }
  }
}
```

One status is returned per execution event. Batch actions may produce multiple statuses.

**Status Types:**

| Status | Terminal | Description | Fields |
|--------|----------|-------------|--------|
| `resting` | No | Order placed and resting on book | `{oid}` |
| `working` | No | Order has partial fills, still resting | `{oid, filledSz, remainingSz, vwap}` |
| `filled` | Yes | Order fully filled | `{oid, totalSz, avgPx}` |
| `partiallyFilled` | Yes | Partially filled then terminal | `{oid, totalSz, avgPx}` |
| `cancelled` | Yes | Cancelled by user | `{oid}` |
| `cancelledRiskLimit` | Yes | Cancelled due to risk limit | `{oid, reason?}` |
| `cancelledSelfCrossing` | Yes | Cancelled due to self-crossing | `{oid}` |
| `cancelledReduceOnly` | Yes | Would not reduce position | `{oid}` |
| `cancelledIOC` | Yes | IOC expired without full fill | `{oid, filledSz}` |
| `rejectedCrossing` | Yes | Post-only rejected for crossing | `{oid}` |
| `rejectedDuplicate` | Yes | Duplicate order ID | `{oid}` |
| `rejectedRiskLimit` | Yes | Rejected due to risk limit | `{oid, reason?}` |
| `rejectedInvalid` | Yes | Invalid order parameters | `{oid, reason?}` |
| `deposit` | Yes | Faucet deposit succeeded | `{amount}` |
| `depositFailed` | Yes | Faucet deposit failed | `{message}` |
| `agentWallet` | Yes | Agent wallet registered | `{agentWallet}` |
| `agentWalletFailed` | Yes | Agent wallet failed | `{message}` |
| `cancelOneRejected` | Yes | Cancel rejected | `{oid, reason}` |
| `cancelAllRejected` | Yes | Cancel all rejected | `{reason}` |
| `error` | Yes | Generic error | `{message}` |

**Example - Order Resting:**
```json
{
  "status": "ok",
  "response": {
    "type": "order",
    "data": {
      "statuses": [
        {"resting": {"oid": "Fpa3oVuL3UzjNANAMZZdmrn6D1Zhk83GmBuJpuAWG51F"}}
      ]
    }
  }
}
```

**Example - Order Filled:**
```json
{
  "status": "ok",
  "response": {
    "type": "order",
    "data": {
      "statuses": [
        {"filled": {"oid": "Fpa3oVuL3UzjNANAMZZdmrn6D1Zhk83GmBuJpuAWG51F", "totalSz": 0.1, "avgPx": 102500.0}}
      ]
    }
  }
}
```

**Example - Faucet Deposit:**
```json
{
  "status": "ok",
  "response": {
    "type": "order",
    "data": {
      "statuses": [
        {"deposit": {"amount": 10000.0}}
      ]
    }
  }
}
```

---

## Account Endpoint

Query account information including positions, orders, and history.

**Endpoint:** `POST /account`

**Request Body (JSON):**
```json
{
  "type": "fullAccount",
  "user": "base58_pubkey"
}
```

**Account Query Types:**

| Type | Description |
|------|-------------|
| `fullAccount` | Complete account state (margin + positions + orders + leverage) |
| `openOrders` | Only resting orders |
| `fills` | Trade history (last 5000) |
| `positions` | Closed position history (last 5000) |
| `fundingHistory` | Funding payment history (last 5000) |
| `orderHistory` | Terminal order history (last 5000) |

### Full Account

**Request:**
```bash
curl -X POST http://localhost:12000/api/v1/account \
  -H "Content-Type: application/json" \
  -d '{"type": "fullAccount", "user": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt"}'
```

**Response:**
```json
[
  {
    "fullAccount": {
      "margin": {
        "totalBalance": 100000.0,
        "availableBalance": 95000.0,
        "marginUsed": 5000.0,
        "notional": 50000.0,
        "realizedPnl": 1234.0,
        "unrealizedPnl": 500.0,
        "fees": 12.5,
        "funding": -5.0
      },
      "positions": [
        {
          "symbol": "BTC-USD",
          "size": 0.5,
          "price": 100000.0,
          "fairPrice": 100050.0,
          "notional": 50000.0,
          "realizedPnl": 1234.0,
          "unrealizedPnl": 500.0,
          "leverage": 5.0,
          "liquidationPrice": 80000.0,
          "fees": 12.5,
          "funding": -5.0,
          "maintenanceMargin": 2500.0,
          "lambda": 0.05,
          "riskAllocation": 0.8,
          "allocMargin": 4000.0
        }
      ],
      "openOrders": [
        {
          "symbol": "BTC-USD",
          "orderId": "Fpa3oVuL3UzjNANAMZZdmrn6D1Zhk83GmBuJpuAWG51F",
          "price": 100000.0,
          "originalSize": 0.1,
          "size": 0.1,
          "filledSize": 0.0,
          "vwap": 0.0,
          "isBuy": false,
          "maker": true,
          "reduceOnly": false,
          "tif": "gtc",
          "status": "resting",
          "timestamp": 1704067200000000000
        }
      ],
      "leverageSettings": [
        {"symbol": "BTC-USD", "leverage": 5.0},
        {"symbol": "ETH-USD", "leverage": 3.0}
      ]
    }
  }
]
```

**Margin Fields:**
- `totalBalance`: Total account equity
- `availableBalance`: Balance available for new orders (totalBalance - marginUsed)
- `marginUsed`: Total maintenance margin requirement
- `notional`: Total position notional value
- `realizedPnl`: Total realized profit/loss
- `unrealizedPnl`: Total unrealized profit/loss
- `fees`: Total fees paid
- `funding`: Total funding payments

**Position Fields:**
- `symbol`: Market symbol
- `size`: Position size (positive = long, negative = short)
- `price`: Volume-weighted average entry price
- `fairPrice`: Current fair/mark price
- `notional`: Position notional value
- `realizedPnl`: Realized PnL for this position
- `unrealizedPnl`: Unrealized PnL at current fair price
- `leverage`: Effective leverage
- `liquidationPrice`: Estimated liquidation price
- `fees`: Fees paid on this position
- `funding`: Funding payments for this position
- `maintenanceMargin`: Maintenance margin requirement
- `lambda`: Risk lambda parameter
- `riskAllocation`: Fraction of portfolio risk allocated
- `allocMargin`: Allocated margin

### Open Orders

**Request:**
```bash
curl -X POST http://localhost:12000/api/v1/account \
  -H "Content-Type: application/json" \
  -d '{"type": "openOrders", "user": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt"}'
```

**Response:**
```json
[
  {
    "openOrder": {
      "symbol": "BTC-USD",
      "orderId": "base58_hash",
      "price": 99000.0,
      "originalSize": 0.1,
      "size": 0.1,
      "filledSize": 0.0,
      "vwap": 0.0,
      "isBuy": true,
      "maker": true,
      "reduceOnly": false,
      "tif": "gtc",
      "status": "resting",
      "timestamp": 1699564800000000000
    }
  }
]
```

### Fill History

Returns up to 5000 recent fills.

**Request:**
```bash
curl -X POST http://localhost:12000/api/v1/account \
  -H "Content-Type: application/json" \
  -d '{"type": "fills", "user": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt"}'
```

**Response:**
```json
[
  {
    "fills": {
      "maker": "maker_pubkey_base58",
      "taker": "taker_pubkey_base58",
      "orderIdMaker": "maker_order_hash",
      "orderIdTaker": "taker_order_hash",
      "isBuy": true,
      "symbol": "BTC-USD",
      "amount": 0.1,
      "price": 100000.0,
      "reason": "normal",
      "slot": 12345,
      "timestamp": 1699564800000
    }
  }
]
```

### Closed Position History

Returns up to 5000 closed positions.

**Request:**
```bash
curl -X POST http://localhost:12000/api/v1/account \
  -H "Content-Type: application/json" \
  -d '{"type": "positions", "user": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt"}'
```

**Response:**
```json
[
  {
    "positions": {
      "owner": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt",
      "symbol": "BTC-USD",
      "maxQuantity": 0.5,
      "totalVolume": 1.2,
      "avgOpenPrice": 100000.0,
      "avgClosePrice": 102500.0,
      "realizedPnl": 1250.0,
      "fees": 12.5,
      "funding": -5.0,
      "openTime": 1699564800000000000,
      "closeTime": 1699651200000000000,
      "closeReason": "normal"
    }
  }
]
```

**Close Reasons:** `normal`, `liquidation`, `adl`

### Funding History

Returns up to 5000 funding payments.

**Request:**
```bash
curl -X POST http://localhost:12000/api/v1/account \
  -H "Content-Type: application/json" \
  -d '{"type": "fundingHistory", "user": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt"}'
```

**Response:**
```json
[
  {
    "fundingPayment": {
      "owner": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt",
      "symbol": "BTC-USD",
      "size": 0.5,
      "payment": -12.5,
      "fundingRate": 0.0001,
      "markPrice": 102500.0,
      "slot": 12345,
      "timestamp": 1699564800000000000
    }
  }
]
```

### Order History

Returns up to 5000 terminal orders.

**Request:**
```bash
curl -X POST http://localhost:12000/api/v1/account \
  -H "Content-Type: application/json" \
  -d '{"type": "orderHistory", "user": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt"}'
```

**Response:**
```json
[
  {
    "orderHistory": {
      "orderId": "Fpa3oVuL3UzjNANAMZZdmrn6D1Zhk83GmBuJpuAWG51F",
      "symbol": "BTC-USD",
      "side": "buy",
      "orderType": "limit",
      "tif": "gtc",
      "price": 100000.0,
      "vwap": 100025.5,
      "originalSize": 0.1,
      "executedSize": 0.1,
      "reduceOnly": false,
      "status": "filled",
      "slot": 12345,
      "timestamp": 1699564800000000000
    }
  }
]
```

---

## Complete Examples

### Example 1: Get Market Data
```bash
# Get all markets
curl http://localhost:12000/api/v1/exchangeInfo

# Get BTC ticker
curl http://localhost:12000/api/v1/ticker/BTC-USD

# Get 1-minute candles
curl "http://localhost:12000/api/v1/klines?symbol=BTC-USD&interval=1m"

# Get weekly candles
curl "http://localhost:12000/api/v1/klines?symbol=BTC-USD&interval=1w"

# Get order book (top 10, $0.5 aggregation)
curl "http://localhost:12000/api/v1/l2book?type=l2Book&coin=BTC-USD&nlevels=10&aggregation=0.5"

# Get exchange stats for 7 days
curl "http://localhost:12000/api/v1/stats?period=7d"
```

### Example 2: Place a Limit Buy Order
```bash
curl -X POST http://localhost:12000/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "actions": [
      {"l": {"c": "BTC-USD", "b": true, "px": 100000.0, "sz": 0.1, "tif": "GTC", "r": false}}
    ],
    "nonce": 1704067200000,
    "account": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt",
    "signer": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt",
    "signature": "5j7sVt3k2YxPqH4w..."
  }'
```

### Example 3: Place a Market Order
```bash
curl -X POST http://localhost:12000/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "actions": [
      {"m": {"c": "BTC-USD", "b": true, "sz": 0.1, "r": false}}
    ],
    "nonce": 1704067200000,
    "account": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt",
    "signer": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt",
    "signature": "5j7sVt3k2YxPqH4w..."
  }'
```

### Example 4: Batch Orders (Multiple Actions)
```bash
curl -X POST http://localhost:12000/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "actions": [
      {"cx": {"c": "BTC-USD", "oid": "old_order_hash_base58"}},
      {"l": {"c": "BTC-USD", "b": true, "px": 100000.0, "sz": 0.05, "tif": "GTC", "r": false}},
      {"l": {"c": "BTC-USD", "b": false, "px": 105000.0, "sz": 0.05, "tif": "GTC", "r": false}}
    ],
    "nonce": 1704067200000,
    "account": "...",
    "signer": "...",
    "signature": "..."
  }'
```

### Example 5: Cancel All Orders in a Symbol
```bash
curl -X POST http://localhost:12000/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "actions": [
      {"cxa": {"c": ["BTC-USD"]}}
    ],
    "nonce": 1704067200000,
    "account": "...",
    "signer": "...",
    "signature": "..."
  }'
```

### Example 6: Cancel All Orders Across All Symbols
```bash
curl -X POST http://localhost:12000/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "actions": [
      {"cxa": {"c": []}}
    ],
    "nonce": 1704067200001,
    "account": "...",
    "signer": "...",
    "signature": "..."
  }'
```

### Example 7: Request Faucet Funds
```bash
curl -X POST http://localhost:12000/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "actions": [
      {"faucet": {"u": "8DmyR3yJhpQHBqgSGua4c69PZ9ZMeaJddTumUdmTx7a", "amount": 10000.0}}
    ],
    "nonce": 1704067200000,
    "account": "8DmyR3yJhpQHBqgSGua4c69PZ9ZMeaJddTumUdmTx7a",
    "signer": "8DmyR3yJhpQHBqgSGua4c69PZ9ZMeaJddTumUdmTx7a",
    "signature": "..."
  }'
```

### Example 8: Register Agent Wallet
```bash
curl -X POST http://localhost:12000/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "actions": [
      {"agentWalletCreation": {"a": "5Am6JkEHAjYG1itNWRMGpQrxvY8AaqkXCo1TZvenqVux", "d": false}}
    ],
    "nonce": 1704067200000,
    "account": "8DmyR3yJhpQHBqgSGua4c69PZ9ZMeaJddTumUdmTx7a",
    "signer": "8DmyR3yJhpQHBqgSGua4c69PZ9ZMeaJddTumUdmTx7a",
    "signature": "..."
  }'
```

### Example 9: Update Leverage Settings
```bash
curl -X POST http://localhost:12000/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "actions": [
      {"updateUserSettings": {"m": [["BTC-USD", 5.0], ["ETH-USD", 3.0]]}}
    ],
    "nonce": 1704067200000,
    "account": "5Am6JkEHAjYG1itNWRMGpQrxvY8AaqkXCo1TZvenqVux",
    "signer": "5Am6JkEHAjYG1itNWRMGpQrxvY8AaqkXCo1TZvenqVux",
    "signature": "..."
  }'
```

### Example 10: Whitelist Faucet (Admin)
```bash
curl -X POST http://localhost:12000/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "actions": [
      {"whitelistFaucet": {"target": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt", "whitelist": true}}
    ],
    "nonce": 1704067200000,
    "account": "ADMIN_PUBKEY",
    "signer": "ADMIN_PUBKEY",
    "signature": "..."
  }'
```

### Example 11: Oracle Price Update (Admin)
```bash
curl -X POST http://localhost:12000/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "actions": [
      {"px": {"t": 1704067200000000000, "c": "BTC-USD", "px": 102500.0}}
    ],
    "nonce": 1704067200000,
    "account": "ORACLE_PUBKEY",
    "signer": "ORACLE_PUBKEY",
    "signature": "..."
  }'
```

### Example 12: Pyth Oracle Batch Update (Admin)
```bash
curl -X POST http://localhost:12000/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "actions": [
      {"o": {"oracles": [
        {"t": 1704067200000000000, "fi": 0, "px": 10250000000000, "e": -8},
        {"t": 1704067200000000000, "fi": 1, "px": 325000000000, "e": -8},
        {"t": 1704067200000000000, "fi": 2, "px": 18500000000, "e": -8}
      ]}}
    ],
    "nonce": 1704067200000,
    "account": "ORACLE_PUBKEY",
    "signer": "ORACLE_PUBKEY",
    "signature": "..."
  }'
```

### Example 13: Batch Multi-Action Transaction

Multiple actions of different types can be combined into a single atomic transaction. All actions share the same `nonce`, `account`, `signer`, and `signature`. The signature covers all actions together.

**Cancel + Replace (cancel old order, place two new ones):**
```bash
curl -X POST http://localhost:12000/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "actions": [
      {"cx": {"c": "BTC-USD", "oid": "old_order_hash_base58"}},
      {"l": {"c": "BTC-USD", "b": true, "px": 99500.0, "sz": 0.05, "tif": "GTC", "r": false}},
      {"l": {"c": "BTC-USD", "b": false, "px": 105000.0, "sz": 0.05, "tif": "GTC", "r": false}}
    ],
    "nonce": 1704067200000,
    "account": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt",
    "signer": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt",
    "signature": "..."
  }'
```

**Faucet + Set Leverage + Place Order (full onboarding in one txn):**
```bash
curl -X POST http://localhost:12000/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "actions": [
      {"faucet": {"u": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt", "amount": 100000.0}},
      {"updateUserSettings": {"m": [["BTC-USD", 10.0], ["ETH-USD", 5.0]]}},
      {"l": {"c": "BTC-USD", "b": true, "px": 100000.0, "sz": 0.1, "tif": "GTC", "r": false}}
    ],
    "nonce": 1704067200000,
    "account": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt",
    "signer": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt",
    "signature": "..."
  }'
```

**CancelAll + Multi-Market Orders:**
```bash
curl -X POST http://localhost:12000/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "actions": [
      {"cxa": {"c": []}},
      {"l": {"c": "BTC-USD", "b": true, "px": 100000.0, "sz": 0.1, "tif": "GTC", "r": false}},
      {"l": {"c": "ETH-USD", "b": true, "px": 3200.0, "sz": 1.0, "tif": "GTC", "r": false}},
      {"m": {"c": "SOL-USD", "b": true, "sz": 10.0, "r": false}}
    ],
    "nonce": 1704067200000,
    "account": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt",
    "signer": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt",
    "signature": "..."
  }'
```

One status per execution event is returned in the response `statuses` array.

### Example 14: Trading Flow
```bash
# Step 1: Request faucet funds
curl -X POST http://localhost:12000/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "actions": [{"faucet": {"u": "YOUR_PUBKEY"}}],
    "nonce": 1704067200000,
    "account": "YOUR_PUBKEY",
    "signer": "YOUR_PUBKEY",
    "signature": "YOUR_SIGNATURE"
  }'

# Step 2: Place a limit buy order
curl -X POST http://localhost:12000/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "actions": [{"l": {"c": "BTC-USD", "b": true, "px": 100000.0, "sz": 0.1, "tif": "GTC", "r": false}}],
    "nonce": 1704067200001,
    "account": "YOUR_PUBKEY",
    "signer": "YOUR_PUBKEY",
    "signature": "YOUR_SIGNATURE"
  }'

# Step 3: Cancel the order
curl -X POST http://localhost:12000/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "actions": [{"cx": {"c": "BTC-USD", "oid": "ORDER_ID_BASE58"}}],
    "nonce": 1704067200002,
    "account": "YOUR_PUBKEY",
    "signer": "YOUR_PUBKEY",
    "signature": "YOUR_SIGNATURE"
  }'
```

---

## Response Status Codes

| Code | Description | Used By |
|------|-------------|---------|
| 200 | Success | All endpoints |
| 400 | Bad Request - Invalid parameters | Market data |
| 404 | Not Found - Symbol or account doesn't exist | Ticker, L2Book |
| 408 | Request Timeout - Executor didn't respond within 2s | /order, /account |
| 500 | Internal Server Error | All endpoints |

---

## API Endpoints Summary

| Method | Endpoint | Description | Parameters |
|--------|----------|-------------|------------|
| GET | `/exchangeInfo` | List all markets | None |
| GET | `/ticker/{symbol}` | Market statistics | symbol (path) |
| GET | `/klines` | Candle history | symbol, interval, startTime, endTime (query) |
| GET | `/l2book` | Order book snapshot | type, coin, nlevels, aggregation (query) |
| GET | `/stats` | Exchange statistics | period, symbol (query) |
| GET | `/metrics` | Runtime metrics snapshot | None |
| GET | `/verify` | Ledger/metrics consistency check | None |
| POST | `/order` | Submit transaction (orders, cancels, faucet, settings, agent wallet, admin) | Transaction (body) |
| POST | `/account` | Query account | type, user (body) |

---

## Transaction Signing Guide

### Overview

All transactions submitted to `POST /order` require **Ed25519 signatures** for authentication.

### JSON vs Binary Formats

The API accepts transactions as **JSON** where all cryptographic fields are **base58-encoded strings**:

| Field | JSON Format | Example |
|-------|-------------|---------|
| `account` | base58 string | `"9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt"` |
| `signer` | base58 string | `"9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt"` |
| `signature` | base58 string | `"5j7sVt3k2YxPqH4w..."` |
| Order IDs (`oid`) | base58 string | `"Fpa3oVuL3UzjNANAMZZdmrn6D1Zhk83GmBuJpuAWG51F"` |
| Pubkeys (`u`, `a`, `target`) | base58 string | `"8DmyR3yJhpQHBqgSGua4c69PZ9ZMeaJddTumUdmTx7a"` |

For **signing**, you must construct the **binary (wincode) representation** of the transaction (without the signature), sign those bytes with Ed25519, then encode the resulting 64-byte signature as a base58 string for the JSON payload.

### What Gets Signed

The signature is computed over the **wincode binary serialization** of the transaction, excluding the signature field:

```
binary_message = wincode_serialize(actions + nonce + account + signer)
signature = ed25519_sign(binary_message, secret_key)
json_signature = bs58_encode(signature)   // 64 raw bytes → base58 string
```

### Binary Serialization Format (wincode)

This describes how each type is encoded in the **binary message** used for signing. This is NOT the JSON format.

| Type | Binary Encoding |
|------|-----------------|
| Enum variant | u32 discriminant (0, 1, 2...), little-endian |
| Pubkey | 32 bytes (bs58.decode the base58 string) |
| Hash (order IDs) | 32 bytes (bs58.decode the base58 string) |
| String | u64 length prefix (LE) + UTF-8 bytes |
| `Option<T>` | 1 byte (0=None, 1=Some) + T if Some |
| `Vec<T>` | u64 count (LE) + elements |
| bool | 1 byte (0 or 1) |
| u64/f64 | 8 bytes little-endian |
| u32 | 4 bytes little-endian |
| i16 | 2 bytes little-endian |

### Action Discriminants

```typescript
const ACTION_CODES = {
  m: 0,              // market order
  l: 1,              // limit order
  mod: 2,            // modify order
  cx: 3,             // cancel
  cxa: 4,            // cancel all
  px: 5,             // price
  o: 6,              // pyth oracle
  faucet: 7,
  agentWalletCreation: 8,
  updateUserSettings: 9,
  whitelistFaucet: 10,
};

const TIME_IN_FORCE_CODES = {
  GTC: 0,
  IOC: 1,
  ALO: 2,
};
```

### Binary Layout

All Pubkey and Hash fields are 32 bytes obtained by `bs58.decode()` on the base58 string.

```
Transaction:
  [8 bytes]  Actions count (u64, LE)
  For each action:
    [4 bytes]  Action discriminant (u32, LE)
    [...]      Action-specific fields (see below)
  [8 bytes]  Nonce (u64, LE)
  [32 bytes] Account pubkey (bs58.decode)
  [32 bytes] Signer pubkey (bs58.decode)
  -- signature is NOT included in the signed message --

MarketOrder (discriminant 0):
  [8 bytes + N]  Symbol string (u64 length + UTF-8)
  [1 byte]       is_buy (bool)
  [8 bytes]      size (f64, LE)
  [1 byte]       reduce_only (bool)

LimitOrder (discriminant 1):
  [8 bytes + N]  Symbol string (u64 length + UTF-8)
  [1 byte]       is_buy (bool)
  [8 bytes]      price (f64, LE)
  [8 bytes]      size (f64, LE)
  [4 bytes]      tif (u32, LE: 0=GTC, 1=IOC, 2=ALO)
  [1 byte]       reduce_only (bool)

ModifyOrder (discriminant 2):
  [32 bytes]     Order ID (bs58.decode)
  [8 bytes + N]  Symbol string (u64 length + UTF-8)
  [8 bytes]      Amount (f64, LE)

Cancel (discriminant 3):
  [8 bytes + N]  Symbol string (u64 length + UTF-8)
  [32 bytes]     Order ID (bs58.decode)

CancelAll (discriminant 4):
  [8 bytes]      Symbols count (u64, LE)
  For each symbol:
    [8 bytes + N]  Symbol string

Price (discriminant 5):
  [8 bytes]      Timestamp (u64, LE)
  [8 bytes + N]  Asset string (u64 length + UTF-8)
  [8 bytes]      Price (f64, LE)

PythOracle (discriminant 6):
  [8 bytes]      Oracles count (u64, LE)
  For each oracle:
    [8 bytes]    Timestamp (u64, LE)
    [8 bytes]    Feed ID (u64, LE)
    [8 bytes]    Price (u64, LE)
    [2 bytes]    Exponent (i16, LE)

Faucet (discriminant 7):
  [32 bytes] User pubkey (bs58.decode)
  [1 byte]   Amount Option (0=None, 1=Some)
  If Some:
    [8 bytes] Amount (f64, LE)

AgentWalletCreation (discriminant 8):
  [32 bytes] Agent pubkey (bs58.decode)
  [1 byte]   Delete flag (bool)

UpdateUserSettings (discriminant 9):
  [8 bytes]      Entry count (u64, LE)
  For each entry:
    [8 bytes + N]  Symbol string
    [8 bytes]      Max leverage (f64, LE)

WhitelistFaucet (discriminant 10):
  [32 bytes] Target pubkey (bs58.decode)
  [1 byte]   Whitelist flag (bool)
```

### Signing Example (JavaScript/TypeScript)

```typescript
import * as nacl from 'tweetnacl';
import bs58 from 'bs58';

function writeU32(value: number): Uint8Array {
  const buf = new Uint8Array(4);
  new DataView(buf.buffer).setUint32(0, value, true);
  return buf;
}

function writeU64(value: number): Uint8Array {
  const buf = new Uint8Array(8);
  new DataView(buf.buffer).setBigUint64(0, BigInt(value), true);
  return buf;
}

function writeBool(value: boolean): Uint8Array {
  return new Uint8Array([value ? 1 : 0]);
}

function writeF64(value: number): Uint8Array {
  const buf = new Uint8Array(8);
  new DataView(buf.buffer).setFloat64(0, value, true);
  return buf;
}

function writeString(str: string): Uint8Array {
  const bytes = new TextEncoder().encode(str);
  return concatBytes(writeU64(bytes.length), bytes);
}

function concatBytes(...arrays: Uint8Array[]): Uint8Array {
  const totalLength = arrays.reduce((sum, arr) => sum + arr.length, 0);
  const result = new Uint8Array(totalLength);
  let offset = 0;
  for (const arr of arrays) { result.set(arr, offset); offset += arr.length; }
  return result;
}

function serializeLimitOrder(order: any): Uint8Array {
  return concatBytes(
    writeU32(1),                    // discriminant
    writeString(order.c),           // symbol
    writeBool(order.b),             // is_buy
    writeF64(order.px),             // price
    writeF64(order.sz),             // size
    writeU32({GTC:0,IOC:1,ALO:2}[order.tif] || 0),
    writeBool(order.r),             // reduce_only
  );
}

function serializeMarketOrder(order: any): Uint8Array {
  return concatBytes(
    writeU32(0),                    // discriminant
    writeString(order.c),           // symbol
    writeBool(order.b),             // is_buy
    writeF64(order.sz),             // size
    writeBool(order.r),             // reduce_only
  );
}

function serializeCancel(cancel: any): Uint8Array {
  return concatBytes(
    writeU32(3),                    // discriminant
    writeString(cancel.c),          // symbol
    bs58.decode(cancel.oid),        // order ID (32 bytes from base58)
  );
}

function serializeCancelAll(cancelAll: any): Uint8Array {
  const parts = [writeU32(4), writeU64(cancelAll.c.length)];
  for (const sym of cancelAll.c) parts.push(writeString(sym));
  return concatBytes(...parts);
}

function serializeFaucet(faucet: any): Uint8Array {
  const parts = [writeU32(7), bs58.decode(faucet.u)];
  if (faucet.amount != null) {
    parts.push(writeBool(true), writeF64(faucet.amount));
  } else {
    parts.push(writeBool(false));
  }
  return concatBytes(...parts);
}

function serializeModifyOrder(data: any): Uint8Array {
  return concatBytes(
    writeU32(2),                    // discriminant
    bs58.decode(data.oid),          // order ID (32 bytes from base58)
    writeString(data.symbol),       // symbol
    writeF64(data.amount),          // new size
  );
}

function serializeAction(action: any): Uint8Array {
  const [type, data] = Object.entries(action)[0] as [string, any];
  switch (type) {
    case 'l':                     return serializeLimitOrder(data);
    case 'm':                     return serializeMarketOrder(data);
    case 'mod':                   return serializeModifyOrder(data);
    case 'cx':                    return serializeCancel(data);
    case 'cxa':                   return serializeCancelAll(data);
    case 'px':                    return concatBytes(writeU32(5), writeU64(data.t), writeString(data.c), writeF64(data.px));
    case 'o': {
      const oracles = data.oracles || [];
      const parts: Uint8Array[] = [writeU32(6), writeU64(oracles.length)];
      for (const o of oracles) {
        parts.push(writeU64(o.t), writeU64(o.fi), writeU64(o.px));
        const ebuf = new Uint8Array(2);
        new DataView(ebuf.buffer).setInt16(0, o.e, true);
        parts.push(ebuf);
      }
      return concatBytes(...parts);
    }
    case 'faucet':                return serializeFaucet(data);
    case 'agentWalletCreation':   return concatBytes(writeU32(8), bs58.decode(data.a), writeBool(data.d));
    case 'updateUserSettings': {
      const entries = data.m || [];
      const parts: Uint8Array[] = [writeU32(9), writeU64(entries.length)];
      for (const [sym, lev] of entries) { parts.push(writeString(sym), writeF64(lev)); }
      return concatBytes(...parts);
    }
    case 'whitelistFaucet':       return concatBytes(writeU32(10), bs58.decode(data.target), writeBool(data.whitelist));
    default: throw new Error(`Unknown action: ${type}`);
  }
}

function serializeTransaction(actions: any[], nonce: number, account: string, signer: string): Uint8Array {
  const parts: Uint8Array[] = [writeU64(actions.length)];
  for (const action of actions) parts.push(serializeAction(action));
  parts.push(writeU64(nonce));
  parts.push(bs58.decode(account));   // 32 bytes
  parts.push(bs58.decode(signer));    // 32 bytes
  return concatBytes(...parts);
}

function signTransaction(secretKey: Uint8Array, actions: any[], nonce: number, account: string, signer: string): string {
  const message = serializeTransaction(actions, nonce, account, signer);
  const signature = nacl.sign.detached(message, secretKey);
  return bs58.encode(signature);
}

// --- Usage ---

const account = "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt";
const signer = account;
const secretKey = bs58.decode("YOUR_SECRET_KEY_BASE58"); // 64 bytes
const nonce = Date.now();

const actions = [
  {l: {c: "BTC-USD", b: true, px: 100000.0, sz: 0.1, tif: "GTC", r: false}}
];

const signature = signTransaction(secretKey, actions, nonce, account, signer);

const tx = { actions, nonce, account, signer, signature };

await fetch('http://localhost:12000/api/v1/order', {
  method: 'POST',
  headers: {'Content-Type': 'application/json'},
  body: JSON.stringify(tx)
});
```

### Agent Wallet Signing

When an agent wallet places orders on behalf of a user:

```json
{
  "actions": [{"l": {"c": "BTC-USD", "b": true, "px": 100000.0, "sz": 0.1, "tif": "GTC", "r": false}}],
  "nonce": 1704067200000,
  "account": "UserPubkey123...",
  "signer": "AgentPubkey456...",
  "signature": "AgentSignature..."
}
```

Requirements:
1. Agent must be pre-authorized via an `agentWalletCreation` action
2. Agent signs with their own private key
3. Order executes against the user's account

### Common Issues

**"Invalid signature"**: The binary message must match the server's wincode serialization exactly. Use `bs58.decode()` to convert base58 Pubkey/Hash strings into raw 32-byte arrays for the binary message. Enum discriminants are u32 (LE). Vec lengths are u64 (LE). The signature itself is NOT part of the signed message - sign the binary, then `bs58.encode()` the 64-byte Ed25519 signature for the JSON payload.

**"Unauthorized signer"**: If `signer != account`, the agent must be pre-authorized first.

**"Account not found"**: Account must be funded first via a faucet action.

---
