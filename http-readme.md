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
curl http://localhost:12001/api/v1/exchangeInfo
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
curl http://localhost:12001/api/v1/ticker/BTC-USD
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
  "fundingRate": 0.0001
}
```

---

### 3. Get Candle History / Klines

Get OHLCV candlestick data for charting.

**Endpoint:** `GET /klines`

**Request Body:**
```json
{
  "symbol": "BTC-USD",
  "interval": "1m",
  "startTime": 1699564800000,
  "endTime": 1699651200000
}
```

**Request:**
```bash
curl -X GET http://localhost:12001/api/v1/klines \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "BTC-USD",
    "interval": "1m"
  }'
```

**Parameters:**
- `symbol` (String, required): Market symbol
- `interval` (String, required): Timeframe (10s, 1m, 5m, 15m, 1h, 4h, 1d)
- `startTime` (Number, optional): Start timestamp in milliseconds
- `endTime` (Number, optional): End timestamp in milliseconds

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
  },
  {
    "t": 1699564860000,
    "T": 1699564920000,
    "o": 101950.0,
    "h": 102050.0,
    "l": 101900.0,
    "c": 102020.0,
    "v": 2.1,
    "n": 5
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

### 4. Get L2 Order Book

Get current order book snapshot with optional filtering.

**Endpoint:** `GET /l2book`

**Query Parameters:**
- `type` (String, required): Must be "l2Book"
- `coin` (String, required): Market symbol
- `nlevels` (Number, optional): Number of price levels to return
- `aggregation` (Number, optional): Price increment for aggregation

**Request (Basic):**
```bash
curl "http://localhost:12001/api/v1/l2book?type=l2Book&coin=BTC-USD"
```

**Request (Top 10 levels):**
```bash
curl "http://localhost:12001/api/v1/l2book?type=l2Book&coin=BTC-USD&nlevels=10"
```

**Request (Aggregated to $0.5):**
```bash
curl "http://localhost:12001/api/v1/l2book?type=l2Book&coin=BTC-USD&aggregation=0.5"
```

**Request (Top 10 levels, $1 aggregation):**
```bash
curl "http://localhost:12001/api/v1/l2book?type=l2Book&coin=BTC-USD&nlevels=10&aggregation=1.0"
```

**Response:**
```json
{
  "updateType": "snapshot",
  "symbol": "BTC-USD",
  "levels": [
    [
      {"px": 102777.0, "sz": 1.5, "n": 3},
      {"px": 102776.5, "sz": 2.3, "n": 5},
      {"px": 102776.0, "sz": 0.8, "n": 2}
    ],
    [
      {"px": 102780.0, "sz": 2.0, "n": 4},
      {"px": 102780.5, "sz": 1.2, "n": 3},
      {"px": 102781.0, "sz": 1.8, "n": 2}
    ]
  ]
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

## Order Endpoints

All order operations (placing orders, canceling single orders, and canceling all orders) use the **same transaction type** (`"type": "order"`) and are submitted to the same endpoint (`POST /order`).

**Transaction Structure:**
```json
{
  "action": {
    "type": "order",
    "orders": [
      {"order": {...}},      // Place a new order
      {"cancel": {...}},     // Cancel a specific order by ID
      {"cancelAll": {...}}   // Cancel all orders (by symbol or all)
    ],
    "nonce": 1704067200000
  },
  "account": "...",
  "signer": "...",
  "signature": "..."
}
```

The `orders` array can contain **any combination** of `order`, `cancel`, and `cancelAll` items, allowing you to batch multiple operations in a single atomic transaction.

**Mixed Batch Example:**
```bash
curl -X POST http://localhost:12001/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "action": {
      "type": "order",
      "orders": [
        {
          "cancel": {
            "c": "BTC-USD",
            "oid": "old_order_hash_base58"
          }
        },
        {
          "order": {
            "c": "BTC-USD",
            "b": true,
            "px": 100000.0,
            "sz": 0.1,
            "r": false,
            "t": {"limit": {"tif": "GTC"}}
          }
        }
      ],
      "nonce": 1704067200000
    },
    "account": "...",
    "signer": "...",
    "signature": "..."
  }'
```

This example cancels an existing order and places a new one in a single transaction.

---

### 5. Place Order

Place a new order (limit, IOC, ALO, or market order).

**Endpoint:** `POST /order`

**Request Body:**
```json
{
  "action": {
    "type": "order",
    "orders": [
      {
        "order": {
          "c": "BTC-USD",
          "b": true,
          "px": 100000.0,
          "sz": 0.1,
          "r": false,
          "t": {
            "limit": {
              "tif": "GTC"
            }
          },
          "cloid": "Fpa3oVuL3UzjNANAMZZdmrn6D1Zhk83GmBuJpuAWG51F"
        }
      }
    ],
    "nonce": 1704067200000
  },
  "account": "9J8T...base58...",
  "signer": "9J8T...base58...",
  "signature": "5j7s...base58..."
}
```

**Request:**
```bash
curl -X POST http://localhost:12001/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "action": {
      "type": "order",
      "orders": [{
        "order": {
          "c": "BTC-USD",
          "b": true,
          "px": 100000.0,
          "sz": 0.1,
          "r": false,
          "t": {"limit": {"tif": "GTC"}}
        }
      }],
      "nonce": 1704067200000
    },
    "account": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt",
    "signer": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt",
    "signature": "5j7sVt3k2YxPqH4w..."
  }'
```

**Transaction Fields:**
- `action`: Order action object
  - `type`: Must be "order"
  - `orders`: Array of orders (supports batch)
  - `nonce`: Unique integer for replay protection (use timestamp in ms or incrementing counter)
- `account`: Account public key (base58) - the account placing the order
- `signer`: Signer public key (base58) - who's signing (usually same as account, or agent)
- `signature`: Ed25519 signature (base58) - signature of `action + account + signer`

**Order Fields:**
- Each item in the `orders` array must be wrapped in an `"order"` object (camelCase) due to the externally tagged enum format
- `order`: Order object containing:
  - `c`: Coin/Symbol (e.g., "BTC-USD")
  - `b`: is_buy (true for buy, false for sell)
  - `px`: Price
  - `sz`: Size/Quantity
  - `r`: reduce_only (true to only reduce position)
  - `t`: Order type object
  - `cloid`: Client order ID (optional, base58 hash) - allows you to track orders with your own identifier. If not provided, this field can be omitted entirely from the request.

**Time In Force (tif):**
- `GTC` - Good Till Cancel (rests on book)
- `IOC` - Immediate or Cancel (fill or kill)
- `ALO` - Add Liquidity Only (post-only, maker)

**Response (Order Placed and Resting):**
```json
{
  "status": "ok",
  "response": {
    "type": "order",
    "data": {
      "statuses": [
        {
          "resting": {
            "oid": "Fpa3oVuL3UzjNANAMZZdmrn6D1Zhk83GmBuJpuAWG51F"
          }
        }
      ]
    }
  }
}
```

**Response (Order Filled Immediately):**
```json
{
  "status": "ok",
  "response": {
    "type": "order",
    "data": {
      "statuses": [
        {
          "filled": {
            "oid": "Fpa3oVuL3UzjNANAMZZdmrn6D1Zhk83GmBuJpuAWG51F",
            "totalSz": 0.1,
            "avgPx": 102500.0
          }
        }
      ]
    }
  }
}
```

**Response (Order Partially Filled):**
```json
{
  "status": "ok",
  "response": {
    "type": "order",
    "data": {
      "statuses": [
        {
          "partiallyfilled": {
            "oid": "Fpa3oVuL3UzjNANAMZZdmrn6D1Zhk83GmBuJpuAWG51F",
            "totalSz": 0.05,
            "avgPx": 102500.0
          }
        }
      ]
    }
  }
}
```

**Response (Order Error):**
```json
{
  "status": "ok",
  "response": {
    "type": "order",
    "data": {
      "statuses": [
        {
          "error": {
            "message": "Insufficient margin"
          }
        }
      ]
    }
  }
}
```

**Response (Order Working - Partial Fill, Still Resting):**
```json
{
  "status": "ok",
  "response": {
    "type": "order",
    "data": {
      "statuses": [
        {
          "working": {
            "oid": "Fpa3oVuL3UzjNANAMZZdmrn6D1Zhk83GmBuJpuAWG51F",
            "filledSz": 0.05,
            "remainingSz": 0.05,
            "vwap": 102500.0
          }
        }
      ]
    }
  }
}
```

**Response (Cancelled Due to Risk Limit):**
```json
{
  "status": "ok",
  "response": {
    "type": "order",
    "data": {
      "statuses": [
        {
          "cancelledRiskLimit": {
            "oid": "Fpa3oVuL3UzjNANAMZZdmrn6D1Zhk83GmBuJpuAWG51F",
            "reason": "Position would exceed max leverage"
          }
        }
      ]
    }
  }
}
```

**Response (IOC Expired Without Full Fill):**
```json
{
  "status": "ok",
  "response": {
    "type": "order",
    "data": {
      "statuses": [
        {
          "cancelledIOC": {
            "oid": "Fpa3oVuL3UzjNANAMZZdmrn6D1Zhk83GmBuJpuAWG51F",
            "filledSz": 0.03
          }
        }
      ]
    }
  }
}
```

**Response (Post-Only Rejected for Crossing):**
```json
{
  "status": "ok",
  "response": {
    "type": "order",
    "data": {
      "statuses": [
        {
          "rejectedCrossing": {
            "oid": "Fpa3oVuL3UzjNANAMZZdmrn6D1Zhk83GmBuJpuAWG51F"
          }
        }
      ]
    }
  }
}
```

**Status Types:**

| Status | Terminal | Description | Fields |
|--------|----------|-------------|--------|
| `resting` | No | Order placed and resting on book | `{oid}` |
| `working` | No | Order has partial fills, still resting | `{oid, filledSz, remainingSz, vwap}` |
| `filled` | Yes | Order fully filled | `{oid, totalSz, avgPx}` |
| `partiallyFilled` | Yes | Order partially filled and terminal | `{oid, totalSz, avgPx}` |
| `cancelled` | Yes | Order cancelled by user | `{oid}` |
| `cancelledRiskLimit` | Yes | Cancelled due to risk limit | `{oid, reason?}` |
| `cancelledSelfCrossing` | Yes | Cancelled due to self-crossing | `{oid}` |
| `cancelledReduceOnly` | Yes | Cancelled - would not reduce position | `{oid}` |
| `cancelledIOC` | Yes | IOC expired without full fill | `{oid, filledSz}` |
| `rejectedCrossing` | Yes | Post-only rejected for crossing | `{oid}` |
| `rejectedDuplicate` | Yes | Duplicate order ID | `{oid}` |
| `rejectedRiskLimit` | Yes | Rejected due to risk limit on submission | `{oid, reason?}` |
| `rejectedInvalid` | Yes | Invalid order parameters | `{oid, reason?}` |
| `error` | Yes | Generic error | `{message}` |

---

### 6. Cancel Order

Cancel an existing order by order ID. Uses the same `POST /order` endpoint with a `cancel` item.

**Endpoint:** `POST /order`

**Request Body:**
```json
{
  "action": {
    "type": "order",
    "orders": [
      {
        "cancel": {
        "c": "BTC-USD",
        "oid": "order_id_hash_base58"
        }
      }
    ],
    "nonce": 1704067200000
  },
  "account": "9J8T...base58...",
  "signer": "9J8T...base58...",
  "signature": "5j7s...base58..."
}
```

**Request:**
```bash
curl -X POST http://localhost:12001/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "action": {
      "type": "order",
      "orders": [{
        "cancel": {
        "c": "BTC-USD",
          "oid": "order_hash_base58"
        }
      }],
      "nonce": 1704067200000
    },
    "account": "...",
    "signer": "...",
    "signature": "..."
  }'
```

**Cancel Fields:**
- `cancel`: Cancel order object (part of orders array)
- `c`: Coin/Symbol
- `oid`: Order ID (Hash in base58 format)

**Response:**
```json
{
  "status": "ok",
  "response": {
    "type": "order",
    "data": {
      "statuses": [
        {
          "cancelled": {
            "oid": "order_hash_base58"
          }
        }
      ]
    }
  }
}
```

---

### 6a. Cancel All Orders

Cancel all orders for a specific symbol or all orders across all symbols.

**Endpoint:** `POST /order`

**Cancel All Orders in a Symbol:**
```bash
curl -X POST http://localhost:12001/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "action": {
      "type": "order",
      "orders": [
        {
          "cancelAll": {
            "c": ["BTC-USD"]
          }
        }
      ],
      "nonce": 1704067200000
    },
    "account": "5Am6JkEHAjYG1itNWRMGpQrxvY8AaqkXCo1TZvenqVux",
    "signer": "5Am6JkEHAjYG1itNWRMGpQrxvY8AaqkXCo1TZvenqVux",
    "signature": "rpkxJezRft2xqfFxaqYRCTRtoobV4Z2Btqj6P52bEcAKczLn5Rgf2Yfm37UN4HGJwywR4QkuDjJUkwZ93DB2Fw9"
  }'
```

**Cancel All Orders Across All Symbols:**
```bash
curl -X POST http://localhost:12001/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "action": {
      "type": "order",
      "orders": [
        {
          "cancelAll": {
            "c": []
          }
        }
      ],
      "nonce": 1704067200001
    },
    "account": "5Am6JkEHAjYG1itNWRMGpQrxvY8AaqkXCo1TZvenqVux",
    "signer": "5Am6JkEHAjYG1itNWRMGpQrxvY8AaqkXCo1TZvenqVux",
    "signature": "rpkxJezRft2xqfFxaqYRCTRtoobV4Z2Btqj6P52bEcAKczLn5Rgf2Yfm37UN4HGJwywR4QkuDjJUkwZ93DB2Fw9"
  }'
```

**Cancel All Fields:**
- `cancelAll`: Cancel all order object (part of orders array, uses camelCase)
  - `c`: Array of symbol strings
  - To cancel all orders in specific symbols: `["BTC-USD", "ETH-USD"]`
  - To cancel all orders across all symbols: `[]` (empty array)

**Response:**
```json
{
  "status": "ok",
  "response": {
    "type": "order",
    "data": {
      "statuses": [
        {
          "cancelled": {
            "oid": "order_hash_base58"
          }
        }
      ]
    }
  }
}
```

**Note:** The response will contain one `cancelled` status for each order that was cancelled.

---

### 6b. Update User Settings (Leverage)

Update user settings, including maximum leverage per symbol.

**Endpoint:** `POST /user-settings`

**Request:**
```bash
curl -X POST http://localhost:12001/api/v1/user-settings \
  -H "Content-Type: application/json" \
  -d '{
    "action": {
      "type": "updateUserSettings",
      "settings": {
        "m": [
          ["BTC-USD", 5.0],
          ["ETH-USD", 3.0]
        ]
      },
      "nonce": 1704067200000
    },
    "account": "5Am6JkEHAjYG1itNWRMGpQrxvY8AaqkXCo1TZvenqVux",
    "signer": "5Am6JkEHAjYG1itNWRMGpQrxvY8AaqkXCo1TZvenqVux",
    "signature": "5JXWgp1fW6px2Gjhw6YHhQ4wEqb6FqMam6m4yg4uRcCksH9WxSv9dVjizGfD4StGtv1z9gR71unZY6tQ6dNDdJ3K"
  }'
```

**Settings Fields:**
- `type`: Must be `"updateUserSettings"`
- `settings`: Settings object
  - `m`: Array of `[symbol, max_leverage]` tuples
    - `symbol`: Market symbol (e.g., "BTC-USD")
    - `max_leverage`: Maximum leverage (1.0 to 50.0)

**Response:**
```json
{
  "status": "ok",
  "response": {
    "type": "order",
    "data": {
      "statuses": []
    }
  }
}
```

**Note:** Leverage settings are clamped between 1.0 and 50.0. The maximum leverage for a symbol cannot exceed the market's configured maximum leverage.

---

## Account Endpoints

### 7. Query Account

Query account information including positions, orders, and fill history.

**Endpoint:** `POST /account`

#### 7a. Get Full Account

**Request:**
```bash
curl -X POST http://localhost:12001/api/v1/account \
  -H "Content-Type: application/json" \
  -d '{
    "type": "fullAccount",
    "user": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt"
  }'
```

**Response:**
```json
[
  {
    "fullAccount": {
      "positions": [
        {
          "coin": "BTC-USD",
          "size": 0.5,
          "price": 100000.0,
          "realizedPnl": 1234,
          "leverage": 5
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
          "status": "placed",
          "timestamp": 1763316177219383423
        }
      ],
      "marginSummary": {
        "positions": [
          {
            "coin": "USDC",
            "size": 100000.0
          }
        ]
      },
      "settings": {
        "maxLeverage": [
          ["BTC-USD", 5.0],
          ["ETH-USD", 3.0]
        ]
      }
    }
  }
]
```

**Account Settings Fields:**
- `settings`: User account settings
  - `maxLeverage`: Array of `[symbol, max_leverage]` tuples
    - `symbol`: Market symbol
    - `max_leverage`: Maximum leverage setting for that symbol (1.0 to 50.0)

#### 7b. Get Open Orders Only

**Request:**
```bash
curl -X POST http://localhost:12001/api/v1/account \
  -H "Content-Type: application/json" \
  -d '{
    "type": "openOrders",
    "user": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt"
  }'
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
      "status": "placed",
      "timestamp": 1699564800000
    }
  }
]
```

#### 7c. Get Fill History

Returns up to 5000 recent fills (trades).

**Request:**
```bash
curl -X POST http://localhost:12001/api/v1/account \
  -H "Content-Type: application/json" \
  -d '{
    "type": "fills",
    "user": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt"
  }'
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
      "liquidation": false,
      "slot": 12345,
      "timestamp": 1699564800000
    }
  }
]
```

#### 7d. Get Closed Position History

Returns up to 5000 closed positions (position history).

**Request:**
```bash
curl -X POST http://localhost:12001/api/v1/account \
  -H "Content-Type: application/json" \
  -d '{
    "type": "positions",
    "user": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt"
  }'
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

**Position Fields:**
- `owner`: Owner public key (base58)
- `symbol`: Market symbol
- `maxQuantity`: Maximum position size reached (positive=long, negative=short)
- `totalVolume`: Total traded volume over position lifetime
- `avgOpenPrice`: Volume-weighted average entry price
- `avgClosePrice`: Volume-weighted average exit price
- `realizedPnl`: Total realized profit/loss
- `fees`: Total fees paid
- `funding`: Total funding payments (positive=received, negative=paid)
- `openTime`: Position open timestamp (nanoseconds)
- `closeTime`: Position close timestamp (nanoseconds)
- `closeReason`: Reason for closure (`normal`, `liquidation`, `adl`)

#### 7e. Get Funding History

Returns up to 5000 funding payments for an account.

**Request:**
```bash
curl -X POST http://localhost:12001/api/v1/account \
  -H "Content-Type: application/json" \
  -d '{
    "type": "fundingHistory",
    "user": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt"
  }'
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

**Funding Payment Fields:**
- `owner`: Owner public key (base58)
- `symbol`: Market symbol
- `size`: Position size at time of funding (positive=long, negative=short)
- `payment`: Funding payment amount in USD (positive=received, negative=paid)
- `fundingRate`: Applied funding rate
- `markPrice`: Fair price at time of funding
- `slot`: Slot number when funding was applied
- `timestamp`: Timestamp (nanoseconds)

#### 7f. Get Order History

Returns up to 5000 terminal orders (filled, cancelled, rejected) for an account.

**Request:**
```bash
curl -X POST http://localhost:12001/api/v1/account \
  -H "Content-Type: application/json" \
  -d '{
    "type": "orderHistory",
    "user": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt"
  }'
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
  },
  {
    "orderHistory": {
      "orderId": "ABC123...",
      "symbol": "BTC-USD",
      "side": "sell",
      "orderType": "limit",
      "tif": "ioc",
      "price": 105000.0,
      "vwap": 0.0,
      "originalSize": 0.5,
      "executedSize": 0.0,
      "reduceOnly": false,
      "status": "cancelledIOC",
      "slot": 12346,
      "timestamp": 1699564900000000000
    }
  }
]
```

**Order History Fields:**
- `orderId`: Order ID (base58)
- `symbol`: Market symbol
- `side`: Order side (`buy` or `sell`)
- `orderType`: Order type (`limit` or `market`)
- `tif`: Time in force (`gtc`, `ioc`, or `postOnly`)
- `price`: Order price
- `vwap`: Volume-weighted average fill price (0 if no fills)
- `originalSize`: Original order size
- `executedSize`: Amount filled
- `reduceOnly`: Whether order was reduce-only
- `status`: Terminal status (see below)
- `reason`: Rejection/cancellation reason (optional)
- `slot`: Slot number when order became terminal
- `timestamp`: Timestamp (nanoseconds)

**Order History Status Values:**
- `filled`: Order fully filled
- `partiallyFilled`: Order partially filled then terminal
- `cancelled`: Order cancelled by user
- `cancelledRiskLimit`: Cancelled due to risk limit
- `cancelledSelfCrossing`: Cancelled due to self-crossing
- `cancelledReduceOnly`: Cancelled - would not reduce position
- `cancelledIOC`: IOC expired without full fill
- `rejectedInvalid`: Invalid order parameters
- `rejectedRiskLimit`: Rejected due to risk limit
- `rejectedCrossing`: Post-only rejected for crossing
- `rejectedDuplicate`: Duplicate order ID

**Account Query Types:**
- `fullAccount`: Complete account state (positions + orders + margin)
- `openOrders`: Only resting orders
- `fills`: Trade history (last 5000 fills)
- `positions`: Closed position history (last 5000 positions)
- `fundingHistory`: Funding payment history (last 5000 payments)
- `orderHistory`: Terminal order history (last 5000 orders)

---

## Private Endpoints

Private endpoints require special authorization.

### 8. Request Faucet

Request testnet funds (10,000 mock USD, once per hour).

**Endpoint:** `POST /api/v1/private/faucet`

**Request Body:**
```json
{
  "action": {
    "type": "faucet",
    "faucet": {
      "u": "8DmyR3yJhpQHBqgSGua4c69PZ9ZMeaJddTumUdmTx7a"
    },
    "nonce": 1704067200000
  },
  "account": "8DmyR3yJhpQHBqgSGua4c69PZ9ZMeaJddTumUdmTx7a",
  "signer": "8DmyR3yJhpQHBqgSGua4c69PZ9ZMeaJddTumUdmTx7a",
  "signature": "rpkxJezRft2xqfFxaqYRCTRtoobV4Z2Btqj6P52bEcAKczLn5Rgf2Yfm37UN4HGJwywR4QkuDjJUkwZ93DB2Fw9"
}
```

**Faucet Fields:**
- `u`: User public key to receive funds (base58)
- `nonce`: Unique integer for replay protection

**Request:**
```bash
curl -X POST http://localhost:12001/api/v1/private/faucet \
  -H "Content-Type: application/json" \
  -d '{
    "action": {"type": "faucet", "faucet": {"u": "8DmyR3yJhpQHBqgSGua4c69PZ9ZMeaJddTumUdmTx7a"}, "nonce": 1704067200000},
    "account": "8DmyR3yJhpQHBqgSGua4c69PZ9ZMeaJddTumUdmTx7a",
    "signer": "8DmyR3yJhpQHBqgSGua4c69PZ9ZMeaJddTumUdmTx7a",
    "signature": "rpkxJezRft2xqfFxaqYRCTRtoobV4Z2Btqj6P52bEcAKczLn5Rgf2Yfm37UN4HGJwywR4QkuDjJUkwZ93DB2Fw9"
  }'
```

**Response:**
```json
{
  "success": true
}
```

Or on error:
```json
{
  "error": "Account already funded"
}
```

---

### 9. Send Oracle Data

Submit oracle price updates (permissioned).

**Endpoint:** `POST /api/v1/private/oracle`

**Request Body:**
```json
{
  "action": {
    "type": "oracle",
    "oracles": [
      {
        "t": 1699564800000,
        "c": "BTC",
        "px": 100000.0
      }
    ],
    "nonce": 1704067200000
  },
  "signature": "5j7s...base58...",
  "account": "oracle_pubkey_base58",
  "signer": "oracle_pubkey_base58"
}
```

**Oracle Fields:**
- `t`: Timestamp (milliseconds)
- `c`: Asset symbol (e.g., "BTC")
- `px`: Price
- `nonce`: Unique integer for replay protection

**Request:**
```bash
curl -X POST http://localhost:12001/api/v1/private/oracle \
  -H "Content-Type: application/json" \
  -d '{
    "action": {
      "type": "oracle",
      "oracles": [{
        "t": 1699564800000,
        "c": "BTC",
        "px": 100000.0
      }],
      "nonce": 1704067200000
    },
    "account": "...",
    "signer": "...",
    "signature": "..." 
  }'
```

**Response:**
```
200 OK
```

---

## Complete Examples

### Example 1: Get Market Data
```bash
# Get all markets
curl http://localhost:12001/api/v1/exchangeInfo

# Get BTC ticker
curl http://localhost:12001/api/v1/ticker/BTC-USD

# Get 1-minute candles
curl "http://localhost:12001/api/v1/klines?symbol=BTC-USD&interval=1m"

# Get order book (top 10, $0.5 aggregation)
curl "http://localhost:12001/api/v1/l2book?type=l2Book&coin=BTC-USD&nlevels=10&aggregation=0.5"
```

### Example 2: Trading Flow
```bash
# Step 1: Request faucet funds
curl -X POST http://localhost:12001/api/v1/private/faucet \
  -H "Content-Type: application/json" \
  -d '{
    "action": {
      "type": "faucet",
      "faucet": {"u": "YOUR_PUBKEY"},
      "nonce": 1704067200000
    },
    "account": "YOUR_PUBKEY",
    "signer": "YOUR_PUBKEY",
    "signature": "YOUR_SIGNATURE" 
  }'

# Step 2: Place a limit buy order
curl -X POST http://localhost:12001/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "action": {
      "type": "order",
      "orders": [{
        "order": {
          "c": "BTC-USD",
          "b": true,
          "px": 100000.0,
          "sz": 0.1,
          "r": false,
          "t": {"limit": {"tif": "GTC"}}
        }
      }],
      "nonce": 1704067200001
    },
    "account": "YOUR_PUBKEY",
    "signer": "YOUR_PUBKEY",
    "signature": "YOUR_SIGNATURE" 
  }'

# Step 3: Cancel the order
curl -X POST http://localhost:12001/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "action": {
      "type": "order",
      "orders": [{
        "cancel": {
        "c": "BTC-USD",
        "oid": "ORDER_ID"
        }
      }],
      "nonce": 1704067200002
    },
    "account": "YOUR_PUBKEY",
    "signer": "YOUR_PUBKEY",
    "signature": "YOUR_SIGNATURE" 
  }'
```

### Example 3: Place Multiple Orders (Batch)
```bash
curl -X POST http://localhost:12001/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "action": {
      "type": "order",
      "orders": [
        {
          "order": {
            "c": "BTC-USD",
            "b": true,
            "px": 100000.0,
            "sz": 0.05,
            "r": false,
            "t": {"limit": {"tif": "GTC"}}
          }
        },
        {
          "order": {
            "c": "BTC-USD",
            "b": false,
            "px": 105000.0,
            "sz": 0.05,
            "r": false,
            "t": {"limit": {"tif": "GTC"}}
          }
        }
      ],
      "nonce": 1704067200000
    },
    "account": "...",
    "signer": "...",
    "signature": "..." 
  }'
```

### Example 4: Place Market Order
```bash
curl -X POST http://localhost:12001/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "action": {
      "type": "order",
      "orders": [{
        "order": {
          "c": "BTC-USD",
          "b": true,
          "px": 0.0,
          "sz": 0.1,
          "r": false,
          "t": {
            "trigger": {
              "is_market": true,
              "triggerPx": 0.0
            }
          }
        }
      }],
      "nonce": 1704067200000
    },
    "account": "...",
    "signer": "...",
    "signature": "..." 
  }'
```

**Market Order Fields:**
- `px`: Price (set to `0.0` for market orders)
- `t.trigger.is_market`: Must be `true` for market execution
- `t.trigger.triggerPx`: Set to `0.0` for immediate market execution

**Note:** Market orders execute immediately at the best available price. Use IOC limit orders with aggressive prices as an alternative.

### Example 5: Place IOC Order (Market Execution)
```bash
curl -X POST http://localhost:12001/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "action": {
      "type": "order",
      "orders": [{
        "order": {
          "c": "BTC-USD",
          "b": true,
          "px": 999999.0,
          "sz": 0.1,
          "r": false,
          "t": {"limit": {"tif": "IOC"}}
        }
      }],
      "nonce": 1704067200000
    },
    "account": "...",
    "signer": "...",
    "signature": "..." 
  }'
```

### Example 6: Cancel All Orders in a Symbol
```bash
curl -X POST http://localhost:12001/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "action": {
      "type": "order",
      "orders": [
        {
          "cancelAll": {
            "c": ["BTC-USD"]
          }
        }
      ],
      "nonce": 1704067200000
    },
    "account": "5Am6JkEHAjYG1itNWRMGpQrxvY8AaqkXCo1TZvenqVux",
    "signer": "5Am6JkEHAjYG1itNWRMGpQrxvY8AaqkXCo1TZvenqVux",
    "signature": "rpkxJezRft2xqfFxaqYRCTRtoobV4Z2Btqj6P52bEcAKczLn5Rgf2Yfm37UN4HGJwywR4QkuDjJUkwZ93DB2Fw9"
  }'
```

### Example 7: Cancel All Orders Across All Symbols
```bash
curl -X POST http://localhost:12001/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "action": {
      "type": "order",
      "orders": [
        {
          "cancelAll": {
            "c": []
          }
        }
      ],
      "nonce": 1704067200001
    },
    "account": "5Am6JkEHAjYG1itNWRMGpQrxvY8AaqkXCo1TZvenqVux",
    "signer": "5Am6JkEHAjYG1itNWRMGpQrxvY8AaqkXCo1TZvenqVux",
    "signature": "rpkxJezRft2xqfFxaqYRCTRtoobV4Z2Btqj6P52bEcAKczLn5Rgf2Yfm37UN4HGJwywR4QkuDjJUkwZ93DB2Fw9"
  }'
```

### Example 8: Place Order with Client Order ID (cloid)
```bash
curl -X POST http://localhost:12001/api/v1/order \
  -H "Content-Type: application/json" \
  -d '{
    "action": {
      "type": "order",
      "orders": [{
        "order": {
          "c": "BTC-USD",
          "b": true,
          "px": 100000.0,
          "sz": 0.1,
          "r": false,
          "t": {"limit": {"tif": "GTC"}},
          "cloid": "Fpa3oVuL3UzjNANAMZZdmrn6D1Zhk83GmBuJpuAWG51F"
        }
      }],
      "nonce": 1704067200000
    },
    "account": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt",
    "signer": "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt",
    "signature": "5j7sVt3k2YxPqH4w..."
  }'
```

**Note:** The `cloid` field is optional. You can:
- Include it with a base58-encoded hash: `"cloid": "Fpa3oVuL3UzjNANAMZZdmrn6D1Zhk83GmBuJpuAWG51F"`
- Omit it entirely from the request (no need to include `"cloid": null`)

### Example 9: Update User Settings (Leverage)
```bash
curl -X POST http://localhost:12001/api/v1/user-settings \
  -H "Content-Type: application/json" \
  -d '{
    "action": {
      "type": "updateUserSettings",
      "settings": {
        "m": [
          ["BTC-USD", 5.0],
          ["ETH-USD", 3.0]
        ]
      },
      "nonce": 1704067200000
    },
    "account": "5Am6JkEHAjYG1itNWRMGpQrxvY8AaqkXCo1TZvenqVux",
    "signer": "5Am6JkEHAjYG1itNWRMGpQrxvY8AaqkXCo1TZvenqVux",
    "signature": "5JXWgp1fW6px2Gjhw6YHhQ4wEqb6FqMam6m4yg4uRcCksH9WxSv9dVjizGfD4StGtv1z9gR71unZY6tQ6dNDdJ3K"
  }'
```

---

## Order Types

### Limit Order
```json
{
  "t": {
    "limit": {
      "tif": "GTC"  // or "IOC", "ALO"
    }
  }
}
```

### Market Order
```json
{
  "t": {
    "trigger": {
      "is_market": true,
      "triggerPx": 0.0
    }
  }
}
```

**Market Order Fields:**
- `px`: Price (set to `0.0` for market orders)
- `t.trigger.is_market`: Must be `true` for market execution
- `t.trigger.triggerPx`: Set to `0.0` for immediate market execution

---

## Response Status Codes

### Current Implementation

| Code | Description | Used By |
|------|-------------|---------|
| 200 | Success | All endpoints |
| 400 | Bad Request - Invalid parameters or transaction type | Market data, Orders |
| 404 | Not Found - Symbol or account doesn't exist | Ticker, L2Book, Account |
| 408 | Request Timeout - Executor didn't respond within 2s | Orders, Faucet, Agent Wallet |
| 500 | Internal Server Error - Database or channel error | All endpoints |

### Status Code Details by Endpoint

**Market Data (Read-only)**:
- `/exchangeInfo`: 200, 500
- `/ticker/:symbol`: 200, 404, 500
- `/klines`: 200 (including empty array), 500
- `/l2book`: 200, 400, 404, 500

**Trading (Write)**:
- `/order` (POST): 200, 408, 500
- `/agent-wallet`: 200, 400, 408, 500
- `/user-settings`: 200, 400, 408, 500

**Account**:
- `/account`: 200, 404, 500

**Private**:
- `/private/faucet`: 200, 400, 408, 500
- `/private/oracle`: 200, 400, 500
---

## Error Responses

**Example Error:**
```json
{
  "status": "error",
  "response": {
    "type": "order",
    "data": {
      "statuses": [
        {
          "error": {
            "message": "Insufficient margin"
          }
        }
      ]
    }
  }
}
```

---

## API Endpoints Summary

### Public Endpoints (No Auth Required)

| Method | Endpoint | Description | Parameters |
|--------|----------|-------------|------------|
| GET | `/exchangeInfo` | List all markets | None |
| GET | `/ticker/{symbol}` | Market statistics | symbol (path) |
| GET | `/klines` | Candle history | symbol, interval (query) |
| GET | `/l2book` | Order book snapshot | type, coin, nlevels, aggregation (query) |
| POST | `/order` | Place order, cancel order, or cancel all | Transaction (body) |
| POST | `/account` | Query account | type, user (body) |
| POST | `/agent-wallet` | Manage agent wallet | Transaction (body) |
| POST | `/user-settings` | Update user settings (leverage) | Transaction (body) |

### Private Endpoints (Auth Required)

| Method | Endpoint | Description | Parameters |
|--------|----------|-------------|------------|
| POST | `/private/faucet` | Request testnet funds | Transaction (body) |
| POST | `/private/oracle` | Submit oracle price | Transaction (body) |

---

## Notes

### Market Orders
There is no explicit "market" order type. To execute at market:
- Use **trigger** type with `is_market: true` and `triggerPx: 0.0` for immediate market execution
- Alternative: Use **IOC limit** with an aggressive price that will cross the spread

### Time In Force Options
- **GTC** (Good Till Cancel): Order rests on book until filled or cancelled
- **IOC** (Immediate or Cancel): Fill immediately or cancel, no resting
- **ALO** (Add Liquidity Only): Post-only, maker order (rejects if crosses)

### Transaction Structure & Signing

All order operations require:
- `action`: The action to perform (order, cancel, etc.)
- `account`: Account public key (base58) - the account performing the action
- `signer`: Signer public key (base58) - who's signing (usually same as account, or authorized agent)
- `signature`: Ed25519 signature (base58)

**What gets signed**:
```
signature = sign(wincode_serialize(action + nonce + account + signer))
```

The signature is computed over the wincode serialization of the action (with nonce), account, and signer fields (signature field itself is excluded). See the "Transaction Signing Guide" section below for full details.

**Agent Wallets**: If an agent wallet is authorized, `signer` can be different from `account`:
- `account`: "UserPubkey123..." (the account being traded)
- `signer`: "AgentPubkey456..." (authorized agent signing)

### Data Sources & Implementation Notes

**Market Data:**
- `/exchangeInfo` - RocksDB blockstore (latest slot)
- `/ticker/:symbol` - RocksDB blockstore (latest slot)
- `/klines` - ClickHouse database
  - 10s: Fast path from blockstore, fallback to ClickHouse
  - Other intervals: ClickHouse materialized views with GROUP BY deduplication
  - All views read directly from base 10s data (not chained)
- `/l2book` - In-memory cache (real-time book state)

**Trading:**
- `/order` - Sends to executor, waits for execution response (2s timeout)
- `/agent-wallet` - Registers agent for automated trading (2s timeout)

**Account:**
- `/account` (fullAccount, openOrders) - Bank in-memory state
- `/account` (fills) - RocksDB fills column (up to 5000 recent fills)
- `/account` (positions) - RocksDB positions column (up to 5000 closed positions)

### Account Endpoint Notes
- **fullAccount**: Reads from bank (current state)
- **openOrders**: Reads from bank (current state)
- **fills**: Reads from RocksDB fills column (historical)
- **positions**: Reads from RocksDB positions column (closed position history)
- **fundingHistory**: Reads from RocksDB funding payments column (historical)
- **orderHistory**: Reads from RocksDB order history column (terminal orders)
- Limit: 5000 most recent fills/positions/funding payments/orders per user

---

### 10. Agent Wallet

Register or manage agent wallet addresses for automated trading.

**Endpoint:** `POST /agent-wallet`

**Purpose:** Add authorized agent wallet addresses to your account. Agent wallets can execute trades on behalf of the account (used for automated trading bots/strategies).

**Request Body:**
```json
{
  "action": {
    "type": "agentWalletCreation",
    "agent": {
      "a": "5Am6JkEHAjYG1itNWRMGpQrxvY8AaqkXCo1TZvenqVux",
      "d": false
    },
    "nonce": 1704067200000
  },
  "account": "8DmyR3yJhpQHBqgSGua4c69PZ9ZMeaJddTumUdmTx7a",
  "signer": "8DmyR3yJhpQHBqgSGua4c69PZ9ZMeaJddTumUdmTx7a",
  "signature": "5JX7N4f2r2qsLheHyvCpwzs4iXQjLPp1UZH1Ec4a7B4wJQi8Np4ncn3tDrEVW9BLjLVWm2nqFaGgk9T3o4WdTgpx"
}
```

**Agent Fields:**
- `a`: Agent public key to authorize (base58)
- `d`: Delete flag (true to remove agent, false to add agent)
- `nonce`: Unique integer for replay protection

**Request:**
```bash
curl -X POST http://localhost:12001/api/v1/agent-wallet \
  -H "Content-Type: application/json" \
  -d '{
    "action": {
      "type": "agentWalletCreation",
      "agent": {
        "a": "5Am6JkEHAjYG1itNWRMGpQrxvY8AaqkXCo1TZvenqVux",
        "d": false
      },
      "nonce": 1704067200000
    },
    "account": "8DmyR3yJhpQHBqgSGua4c69PZ9ZMeaJddTumUdmTx7a",
    "signer": "8DmyR3yJhpQHBqgSGua4c69PZ9ZMeaJddTumUdmTx7a",
    "signature": "5JX7N4f2r2qsLheHyvCpwzs4iXQjLPp1UZH1Ec4a7B4wJQi8Np4ncn3tDrEVW9BLjLVWm2nqFaGgk9T3o4WdTgpx"
  }'
```

**Response:**
```json
{
  "status": "ok",
  "response": {
    "type": "agent_wallet",
    "data": {
      "statuses": [
        {
          "agentWallet": {
            "agent_wallet": "AgentPubkeyHere123..."
          }
        }
      ]
    }
  }
}
```

**Note:** Once an agent wallet is registered (d=false), that address can place orders on behalf of the main account. Set d=true to remove/deauthorize the agent. 

---

## Transaction Signing Guide

### Overview

All state-mutating operations (orders, cancels, agent wallets) require **Ed25519 signatures** for authentication and security.

### Transaction Structure

```json
{
  "action": {...},       // The action to perform
  "account": "...",      // Account public key (base58)
  "signer": "...",       // Signer public key (base58)
  "signature": "..."     // Ed25519 signature (base58)
}
```

**Fields**:
- `action`: The operation (order, cancel, etc.) - includes `nonce` for replay protection
- `account`: The account performing the action
- `signer`: Who's signing (usually same as account, or authorized agent)
- `signature`: Ed25519 signature of `action + account + signer`

**Nonce**: Every action requires a unique `nonce` (u64) for replay protection. Use:
- Timestamp in milliseconds: `Date.now()`
- Or an incrementing counter per account

### What Gets Signed

The signature is computed over the **wincode serialization** of:
```
action + nonce + account + signer
```

The `signature` field itself is NOT included in what gets signed.

### Serialization Format (wincode)

The exchange uses **wincode** (a custom binary format) for signing. Key differences from JSON:

| Type | Encoding |
|------|----------|
| Enum variant | u32 discriminant (0, 1, 2...) |
| Pubkey/Hash | Raw 32 bytes (decoded from base58) |
| Signature | Raw 64 bytes |
| String | u64 length prefix + UTF-8 bytes |
| Option<T> | 1 byte (0=None, 1=Some) + T if Some |
| Vec<T> | u64 count + elements |
| bool | 1 byte (0 or 1) |
| u64/f64 | 8 bytes little-endian |
| u32 | 4 bytes little-endian |

### Enum Discriminant Mappings

**Action Types (OrderTransaction):**
```typescript
const ACTION_CODES = {
  order: 0,
  oracle: 1,
  faucet: 2,
  updateUserSettings: 3,
  agentWalletCreation: 4,
  testnetAdmin: 5,
};
```

**Order Item Types (OrderItem):**
```typescript
const ORDER_ITEM_CODES = {
  order: 0,
  cancel: 1,
  cancelAll: 2,
};
```

**Time In Force:**
```typescript
const TIME_IN_FORCE_CODES = {
  GTC: 0,
  IOC: 1,
  ALO: 2,
};
```

**Order Type:**
```typescript
const ORDER_TYPE_CODES = {
  limit: 0,
  trigger: 1,
};
```

### Step-by-Step Signing (JavaScript/TypeScript)

**Install dependencies:**
```bash
pnpm install tweetnacl bs58
# or
yarn add tweetnacl bs58
```

**Complete signing implementation:**

```typescript
import * as nacl from 'tweetnacl';
import bs58 from 'bs58';

// ============================================================================
// Enum Discriminant Mappings
// ============================================================================

const ACTION_CODES: Record<string, number> = {
  order: 0,
  oracle: 1,
  faucet: 2,
  updateUserSettings: 3,
  agentWalletCreation: 4,
  testnetAdmin: 5,
};

const ORDER_ITEM_CODES: Record<string, number> = {
  order: 0,
  cancel: 1,
  cancelAll: 2,
};

const TIME_IN_FORCE_CODES: Record<string, number> = {
  GTC: 0,
  IOC: 1,
  ALO: 2,
};

const ORDER_TYPE_CODES: Record<string, number> = {
  limit: 0,
  trigger: 1,
};

const ADMIN_ACTION_CODES: Record<string, number> = {
  whitelistFaucet: 0,
};

// ============================================================================
// Primitive Writers
// ============================================================================

function writeU32(value: number): Uint8Array {
  const buf = new Uint8Array(4);
  new DataView(buf.buffer).setUint32(0, value, true); // little-endian
  return buf;
}

function writeU64(value: number): Uint8Array {
  const buf = new Uint8Array(8);
  new DataView(buf.buffer).setBigUint64(0, BigInt(value), true); // little-endian
  return buf;
}

function writeBool(value: boolean): Uint8Array {
  return new Uint8Array([value ? 1 : 0]);
}

function writeString(str: string): Uint8Array {
  const bytes = new TextEncoder().encode(str);
  return concatBytes(writeU64(bytes.length), bytes);
}

function writeF64(value: number): Uint8Array {
  const buf = new Uint8Array(8);
  new DataView(buf.buffer).setFloat64(0, value, true); // little-endian
  return buf;
}

function concatBytes(...arrays: Uint8Array[]): Uint8Array {
  const totalLength = arrays.reduce((sum, arr) => sum + arr.length, 0);
  const result = new Uint8Array(totalLength);
  let offset = 0;
  for (const arr of arrays) {
    result.set(arr, offset);
    offset += arr.length;
  }
  return result;
}

// ============================================================================
// Validation Helpers
// ============================================================================

function decodeAndValidateKey(key: string): Uint8Array {
  const bytes = bs58.decode(key);
  if (bytes.length !== 32) {
    throw new Error(`Key must be 32 bytes, got ${bytes.length}`);
  }
  return bytes;
}

function decodeAndValidateHash(hash: string): Uint8Array {
  const bytes = bs58.decode(hash);
  if (bytes.length !== 32) {
    throw new Error(`Hash must be 32 bytes, got ${bytes.length}`);
  }
  return bytes;
}

// ============================================================================
// Order Serialization
// ============================================================================

function serializeOrderItem(orderWrapper: any): Uint8Array {
  // Get the order item type (order, cancel, or cancelAll)
  const itemType = Object.keys(orderWrapper)[0];
  if (!itemType || !(itemType in ORDER_ITEM_CODES)) {
    throw new Error(`Invalid order item type: ${itemType}`);
  }

  const parts: Uint8Array[] = [];

  // Write order item discriminant (u32)
  parts.push(writeU32(ORDER_ITEM_CODES[itemType]));

  const data = orderWrapper[itemType];

  if (itemType === 'order') {
    // Order: asset, is_buy, price, size, reduce_only, order_type, client_id
    parts.push(writeString(data.c));           // asset
    parts.push(writeBool(data.b));             // is_buy
    parts.push(writeF64(data.px));             // price
    parts.push(writeF64(data.sz));             // size
    parts.push(writeBool(data.r));             // reduce_only

    // Order type (limit or trigger)
    if (data.t.limit) {
      parts.push(writeU32(ORDER_TYPE_CODES.limit));
      parts.push(writeU32(TIME_IN_FORCE_CODES[data.t.limit.tif] || 0));
    } else if (data.t.trigger) {
      parts.push(writeU32(ORDER_TYPE_CODES.trigger));
      parts.push(writeBool(data.t.trigger.is_market));
      parts.push(writeF64(data.t.trigger.triggerPx));
    } else {
      throw new Error('Order must have either limit or trigger type');
    }

    // client_id: Option<Hash> - 1 byte discriminant + 32 bytes if Some
    if (data.cloid) {
      parts.push(writeBool(true));
      parts.push(decodeAndValidateHash(data.cloid)); // Raw 32 bytes, NOT writeString!
    } else {
      parts.push(writeBool(false));
    }

  } else if (itemType === 'cancel') {
    // Cancel: asset, oid (Hash as raw 32 bytes)
    parts.push(writeString(data.c));           // asset
    parts.push(decodeAndValidateHash(data.oid)); // oid - raw 32 bytes

  } else if (itemType === 'cancelAll') {
    // CancelAll: assets (Vec<String>)
    const assets = data.c || [];
    parts.push(writeU64(assets.length));
    for (const asset of assets) {
      parts.push(writeString(asset));
    }
  }

  return concatBytes(...parts);
}

function serializeOrders(orders: any[]): Uint8Array {
  const parts: Uint8Array[] = [];

  // Write order count (u64)
  parts.push(writeU64(orders.length));

  // Serialize each order item
  for (const order of orders) {
    parts.push(serializeOrderItem(order));
  }

  return concatBytes(...parts);
}

// ============================================================================
// Action Serialization
// ============================================================================

function serializeFaucet(faucet: any): Uint8Array {
  const parts: Uint8Array[] = [];

  // user: Pubkey (raw 32 bytes)
  parts.push(decodeAndValidateKey(faucet.u));

  // amount: Option<f64>
  if (faucet.amount !== undefined && faucet.amount !== null) {
    parts.push(writeBool(true));
    parts.push(writeF64(faucet.amount));
  } else {
    parts.push(writeBool(false));
  }

  return concatBytes(...parts);
}

function serializeAgentWalletCreation(agent: any): Uint8Array {
  return concatBytes(
    decodeAndValidateKey(agent.a),  // agent: Pubkey (raw 32 bytes)
    writeBool(agent.d)               // delete: bool
  );
}

function serializeUpdateUserSettings(settings: any): Uint8Array {
  const parts: Uint8Array[] = [];

  const leverageMap = settings.m || [];
  parts.push(writeU64(leverageMap.length));

  for (const [symbol, leverage] of leverageMap) {
    parts.push(writeString(symbol));
    parts.push(writeF64(leverage));
  }

  return concatBytes(...parts);
}

function serializeOracle(oracles: any[]): Uint8Array {
  const parts: Uint8Array[] = [];

  parts.push(writeU64(oracles.length));

  for (const oracle of oracles) {
    parts.push(writeU64(oracle.t));      // timestamp
    parts.push(writeString(oracle.c));   // asset
    parts.push(writeF64(oracle.px));     // price
  }

  return concatBytes(...parts);
}

function serializeTestnetAdmin(actions: any[]): Uint8Array {
  const parts: Uint8Array[] = [];

  parts.push(writeU64(actions.length));

  for (const action of actions) {
    if (action.whitelistFaucet) {
      parts.push(writeU32(ADMIN_ACTION_CODES.whitelistFaucet));
      parts.push(decodeAndValidateKey(action.whitelistFaucet.account));
      parts.push(writeBool(action.whitelistFaucet.whitelist));
    }
  }

  return concatBytes(...parts);
}

// ============================================================================
// Main Transaction Serialization
// ============================================================================

/**
 * Serialize transaction using wincode format for signing.
 *
 * Format: action_discriminant(u32) + action_data + nonce(u64) + account(32) + signer(32)
 */
export function serializeTransaction(
  action: any,
  account: string,
  signer: string
): Uint8Array {
  const actionType = action.type || '';

  if (!(actionType in ACTION_CODES)) {
    throw new Error(`Invalid action type: ${actionType}`);
  }

  const parts: Uint8Array[] = [];

  // 1. Action discriminant (u32)
  parts.push(writeU32(ACTION_CODES[actionType]));

  // 2. Action-specific data
  switch (actionType) {
    case 'order':
      parts.push(serializeOrders(action.orders || []));
      break;
    case 'oracle':
      parts.push(serializeOracle(action.oracles || []));
      break;
    case 'faucet':
      parts.push(serializeFaucet(action.faucet || {}));
      break;
    case 'updateUserSettings':
      parts.push(serializeUpdateUserSettings(action.settings || {}));
      break;
    case 'agentWalletCreation':
      parts.push(serializeAgentWalletCreation(action.agent || {}));
      break;
    case 'testnetAdmin':
      parts.push(serializeTestnetAdmin(action.actions || []));
      break;
  }

  // 3. Nonce (u64)
  parts.push(writeU64(action.nonce));

  // 4. Account (32 bytes)
  parts.push(decodeAndValidateKey(account));

  // 5. Signer (32 bytes)
  parts.push(decodeAndValidateKey(signer));

  return concatBytes(...parts);
}

/**
 * Sign a transaction action for the exchange
 */
export function signTransaction(
  secretKey: Uint8Array,
  action: any,
  account: string,
  signer: string
): string {
  // Serialize the transaction using wincode format
  const message = serializeTransaction(action, account, signer);

  // Sign with Ed25519
  const signature = nacl.sign.detached(message, secretKey);

  // Encode as base58
  return bs58.encode(signature);
}

// ============================================================================
// Usage Examples
// ============================================================================

// Example 1: Place a limit order
const orderAction = {
  type: "order",
  orders: [{
    order: {
      c: "BTC-USD",
      b: true,
      px: 100000.0,
      sz: 0.1,
      r: false,
      t: { limit: { tif: "GTC" } }
      // cloid: "Fpa3oVuL3UzjNANAMZZdmrn6D1Zhk83GmBuJpuAWG51F"  // optional
    }
  }],
  nonce: Date.now()
};

// Example 2: Cancel an order
const cancelAction = {
  type: "order",
  orders: [{
    cancel: {
      c: "BTC-USD",
      oid: "Fpa3oVuL3UzjNANAMZZdmrn6D1Zhk83GmBuJpuAWG51F"
    }
  }],
  nonce: Date.now()
};

// Example 3: Cancel all orders in a symbol
const cancelAllAction = {
  type: "order",
  orders: [{
    cancelAll: {
      c: ["BTC-USD"]  // or [] for all symbols
    }
  }],
  nonce: Date.now()
};

// Example 4: Mixed batch (cancel + new order)
const mixedAction = {
  type: "order",
  orders: [
    { cancel: { c: "BTC-USD", oid: "old_order_hash_base58" } },
    { order: { c: "BTC-USD", b: true, px: 100000.0, sz: 0.1, r: false, t: { limit: { tif: "GTC" } } } }
  ],
  nonce: Date.now()
};

// Example 5: Faucet request
const faucetAction = {
  type: "faucet",
  faucet: {
    u: "8DmyR3yJhpQHBqgSGua4c69PZ9ZMeaJddTumUdmTx7a",
    // amount: 10000.0  // optional
  },
  nonce: Date.now()
};

// Example 6: Agent wallet creation
const agentAction = {
  type: "agentWalletCreation",
  agent: {
    a: "5Am6JkEHAjYG1itNWRMGpQrxvY8AaqkXCo1TZvenqVux",
    d: false  // false = add, true = remove
  },
  nonce: Date.now()
};

// Example 7: Update user settings (leverage)
const settingsAction = {
  type: "updateUserSettings",
  settings: {
    m: [
      ["BTC-USD", 5.0],
      ["ETH-USD", 3.0]
    ]
  },
  nonce: Date.now()
};

// ============================================================================
// Full Signing Flow
// ============================================================================

const account = "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt";
const signer = "9J8TUdEWrrcADK913r1Cs7DdqX63VdVU88imfDzT1ypt";

// Get secret key (64 bytes: 32 bytes private key + 32 bytes public key)
const secretKey = bs58.decode("YOUR_SECRET_KEY_BASE58");

// Sign the transaction
const signature = signTransaction(secretKey, orderAction, account, signer);

// Create signed transaction for API
const signedTransaction = {
  action: orderAction,
  account,
  signer,
  signature
};

// Send to API
const response = await fetch('http://localhost:12001/api/v1/order', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify(signedTransaction)
});
```

### Serialization Format Details

#### Order Action Binary Layout

```
[4 bytes]  Action discriminant (0 = order)
[8 bytes]  Order count (u64)

For each order item:
  [4 bytes]  Order item discriminant (0=order, 1=cancel, 2=cancelAll)

  If order (0):
    [8 bytes + N]  Asset string (u64 length + UTF-8 bytes)
    [1 byte]       is_buy (bool)
    [8 bytes]      price (f64)
    [8 bytes]      size (f64)
    [1 byte]       reduce_only (bool)
    [4 bytes]      Order type discriminant (0=limit, 1=trigger)
    If limit:
      [4 bytes]    Time in force (0=GTC, 1=IOC, 2=ALO)
    If trigger:
      [1 byte]     is_market (bool)
      [8 bytes]    trigger_price (f64)
    [1 byte]       client_id Option (0=None, 1=Some)
    If Some:
      [32 bytes]   client_id Hash (raw bytes, NOT base58 string!)

  If cancel (1):
    [8 bytes + N]  Asset string
    [32 bytes]     Order ID Hash (raw bytes)

  If cancelAll (2):
    [8 bytes]      Asset count (u64)
    For each asset:
      [8 bytes + N]  Asset string

[8 bytes]  Nonce (u64)
[32 bytes] Account pubkey (raw bytes)
[32 bytes] Signer pubkey (raw bytes)
```

#### Faucet Action Binary Layout

```
[4 bytes]  Action discriminant (2 = faucet)
[32 bytes] User pubkey (raw bytes, NOT base58 string!)
[1 byte]   Amount Option (0=None, 1=Some)
If Some:
  [8 bytes] Amount (f64)
[8 bytes]  Nonce (u64)
[32 bytes] Account pubkey (raw bytes)
[32 bytes] Signer pubkey (raw bytes)
```

#### Agent Wallet Action Binary Layout

```
[4 bytes]  Action discriminant (4 = agentWalletCreation)
[32 bytes] Agent pubkey (raw bytes)
[1 byte]   Delete flag (bool)
[8 bytes]  Nonce (u64)
[32 bytes] Account pubkey (raw bytes)
[32 bytes] Signer pubkey (raw bytes)
```

### Agent Wallet Signing

When an agent wallet places orders on behalf of a user:

```json
{
  "action": {...},
  "account": "UserPubkey123...",   // ← The user's account
  "signer": "AgentPubkey456...",   // ← The agent's key (different!)
  "signature": "AgentSignature..."  // ← Signed by agent's private key
}
```

**Requirements**:
1. Agent must be pre-authorized via `/agent-wallet` endpoint
2. Agent signs with their own private key
3. Order executes against user's account
4. Useful for trading bots and automated strategies

### Common Issues

**"Invalid signature"**:
- Verify action discriminant is u32 (not length-prefixed string)
- Check that Pubkey/Hash fields are raw 32 bytes (not base58 strings)
- Ensure Option types have proper 1-byte discriminant
- Verify field order matches the binary layout exactly

**"Unauthorized signer"**:
- If `signer != account`, agent must be pre-authorized
- Use `/agent-wallet` to authorize agents first

**"Account not found"**:
- Account must be funded first (use `/private/faucet` for testnet)

---
