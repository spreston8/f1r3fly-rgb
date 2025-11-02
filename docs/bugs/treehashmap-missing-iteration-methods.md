# TreeHashMap Missing Iteration and Search Methods

**Status:** Open  
**Severity:** Medium  
**Component:** F1r3fly/RChain Core - TreeHashMap System Contract  
**Affects:** All applications requiring data queries beyond key-based lookups  

## Problem

TreeHashMap (defined in `f1r3node/casper/src/main/resources/Registry.rho`) only supports **key-based operations**:
- `get(key)` - retrieve by exact key
- `set(key, value)` - store by key  
- `contains(key)` - check existence
- `delete(key)` - remove by key
- `getOrElse(key, foundCh, notFoundCh)` - retrieve with fallback
- `update(key, updateFn)` - update by key

**Missing critical methods:**
- ❌ No iteration (`forEach`, `keys`, `values`, `entries`)
- ❌ No search/filter by value
- ❌ No size/count
- ❌ No range queries
- ❌ No aggregation (map/reduce)

## Impact

Cannot implement common database queries:
- Search RGB contracts by ticker (e.g., find "BTC" contract)
- List all contracts owned by a user
- Count total allocations
- Query transitions by date range
- Export/backup all data

## Current Workaround

**Secondary Index Pattern** - Maintain multiple TreeHashMaps:
```rholang
contractMapCh    // contract_id → metadata (primary)
tickerIndexCh    // ticker → contract_id (index)
```

**Limitations:**
- Requires manual index synchronization
- Not atomic (indexes can become inconsistent)
- Increased storage overhead
- More complex contract logic

## Proposed Solution

Add iteration methods to TreeHashMap or provide `IterableTreeHashMap`:
```rholang
TreeHashMap!("forEach", map, *callbackCh)  // Iterate all entries
TreeHashMap!("keys", map, *retCh)          // Get all keys
TreeHashMap!("size", map, *retCh)          // Count entries
```

This would benefit the entire F1r3fly/RChain ecosystem, not just RGB applications.

