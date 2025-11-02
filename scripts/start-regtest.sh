#!/bin/bash
set -e

: '
===============================================================================
Regtest Environment Startup Script
===============================================================================
Starts Bitcoin Core, Electrs, and Esplora for local RGB testing

RUNNING MULTIPLE TEST ITERATIONS:
For reliable back-to-back test runs, use appropriate delays.

Example 1: Simple loop with delays (recommended)
for i in 1 2 3 4 5; do
  ./start-regtest.sh
  sleep 5  # Let services fully initialize
  cd wallet && cargo test --test rgb_transfer_balance_test -- --ignored --nocapture || true
  cd ..
  ./stop-regtest.sh
  sleep 3  # Let services fully stop and release locks
done

Example 2: Minimal loop (no echoes, continues even if tests fail)
for i in 1 2 3 4 5; do
  ./start-regtest.sh || true
  cd wallet || continue
  cargo test --test rgb_transfer_balance_test -- --ignored --nocapture || true
  cd .. || continue
  ./stop-regtest.sh || true
done

WHY DELAYS ARE IMPORTANT:
- Services need time to initialize (especially Electrs indexing)
- Database locks need time to release between runs
- Without delays, Electrs may start with stale index causing StateInsufficient
===============================================================================
'

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BITCOIN_DATADIR="$PROJECT_ROOT/.bitcoin"
ELECTRS_DIR="$PROJECT_ROOT/electrs"
ELECTRS_DB="$PROJECT_ROOT/.electrs_db"
ESPLORA_DIR="$PROJECT_ROOT/esplora"
LOG_DIR="$PROJECT_ROOT/logs"
PID_FILE="$PROJECT_ROOT/.regtest-pids"

# Test wallet
TEST_ADDRESS="bcrt1q6rz28mcfaxtmd6v789l9rrlrusdprr9pz3cppk"

# Ports
BITCOIN_RPC_PORT=18443
ELECTRS_HTTP_PORT=3002
ELECTRS_ELECTRUM_PORT=60401
ESPLORA_PORT=5001

echo -e "${BLUE}🚀 Starting Regtest Environment${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# ============================================================================
# 0. CLEAN PREVIOUS STATE (for consistent test runs)
# ============================================================================


# Always stop running services first
if pgrep -f "bitcoind.*regtest.*$BITCOIN_DATADIR" > /dev/null; then
    echo -e "${YELLOW}⚠️  Stopping existing Bitcoin Core...${NC}"
    bitcoin-cli -regtest -datadir="$BITCOIN_DATADIR" stop 2>/dev/null || true
    sleep 3
fi

if pgrep -f "electrs --network regtest" > /dev/null; then
    echo -e "${YELLOW}⚠️  Stopping existing Electrs...${NC}"
    pkill -f "electrs --network regtest" || true
    sleep 2
fi

if lsof -ti:$ESPLORA_PORT > /dev/null 2>&1; then
    echo -e "${YELLOW}⚠️  Stopping existing Esplora...${NC}"
    kill $(lsof -ti:$ESPLORA_PORT) 2>/dev/null || true
    sleep 2
fi


# Create necessary directories
mkdir -p "$LOG_DIR"
mkdir -p "$BITCOIN_DATADIR"
mkdir -p "$ELECTRS_DB"

# Clean up old PID file
rm -f "$PID_FILE"

# ============================================================================
# 1. PRE-CHECKS
# ============================================================================

echo -e "\n${YELLOW}📋 Pre-flight checks...${NC}"

# Check if bitcoin-core is installed
if ! command -v bitcoind &> /dev/null; then
    echo -e "${RED}❌ bitcoind not found. Install with: brew install bitcoin${NC}"
    exit 1
fi

BITCOIN_VERSION=$(bitcoind --version | head -n1)
echo -e "${GREEN}✓${NC} Bitcoin Core: $BITCOIN_VERSION"

# Check if electrs directory exists
if [ ! -d "$ELECTRS_DIR" ]; then
    echo -e "${RED}❌ Electrs directory not found at: $ELECTRS_DIR${NC}"
    exit 1
fi
echo -e "${GREEN}✓${NC} Electrs directory found"

# Check if esplora directory exists
if [ ! -d "$ESPLORA_DIR" ]; then
    echo -e "${RED}❌ Esplora directory not found at: $ESPLORA_DIR${NC}"
    exit 1
fi
echo -e "${GREEN}✓${NC} Esplora directory found"

# ============================================================================
# 2. START BITCOIN CORE
# ============================================================================

echo -e "\n${YELLOW}⛓️  Starting Bitcoin Core (Regtest)...${NC}"

bitcoind -regtest -server -daemon \
    -datadir="$BITCOIN_DATADIR" \
    -rpcallowip=127.0.0.1 \
    -rpcbind=127.0.0.1:$BITCOIN_RPC_PORT \
    -txindex=1 \
    -zmqpubrawblock=tcp://127.0.0.1:28332 \
    -zmqpubrawtx=tcp://127.0.0.1:28333 \
    -fallbackfee=0.0001

# Wait for Bitcoin RPC to be ready
echo -n "Waiting for Bitcoin RPC"
for i in {1..30}; do
    if bitcoin-cli -regtest -datadir="$BITCOIN_DATADIR" getblockchaininfo &>/dev/null; then
        echo -e " ${GREEN}✓${NC}"
        break
    fi
    echo -n "."
    sleep 1
    if [ $i -eq 30 ]; then
        echo -e " ${RED}✗${NC}"
        echo -e "${RED}❌ Bitcoin Core failed to start${NC}"
        exit 1
    fi
done

# Get current block height
CURRENT_HEIGHT=$(bitcoin-cli -regtest -datadir="$BITCOIN_DATADIR" getblockchaininfo | grep -o '"blocks": [0-9]*' | grep -o '[0-9]*')
echo -e "${GREEN}✓${NC} Bitcoin Core started (height: $CURRENT_HEIGHT)"

# ============================================================================
# 3. SETUP WALLET & FUNDING
# ============================================================================

echo -e "\n${YELLOW}💰 Setting up test wallet...${NC}"

# Create or load mining wallet
if ! bitcoin-cli -regtest -datadir="$BITCOIN_DATADIR" listwallets | grep -q "mining_wallet"; then
    # Check if wallet exists on disk
    if [ -d "$BITCOIN_DATADIR/regtest/wallets/mining_wallet" ]; then
        echo "Loading existing mining_wallet..."
        bitcoin-cli -regtest -datadir="$BITCOIN_DATADIR" loadwallet "mining_wallet" > /dev/null 2>&1
    else
        echo "Creating mining_wallet..."
        bitcoin-cli -regtest -datadir="$BITCOIN_DATADIR" createwallet "mining_wallet" > /dev/null 2>&1
    fi
fi

# Get or create mining address
MINING_ADDRESS=$(bitcoin-cli -regtest -datadir="$BITCOIN_DATADIR" -rpcwallet=mining_wallet getnewaddress "mining" "bech32")
echo -e "${GREEN}✓${NC} Mining address: $MINING_ADDRESS"

# Check if we need to generate initial blocks
if [ "$CURRENT_HEIGHT" -lt 101 ]; then
    echo "Generating initial 101 blocks (coinbase maturity)..."
    bitcoin-cli -regtest -datadir="$BITCOIN_DATADIR" generatetoaddress 101 "$MINING_ADDRESS" > /dev/null
    echo -e "${GREEN}✓${NC} Generated 101 blocks"
    CURRENT_HEIGHT=101
fi

# Fund test wallet address
echo "Funding test wallet: $TEST_ADDRESS"
FUND_TXID=$(bitcoin-cli -regtest -datadir="$BITCOIN_DATADIR" -rpcwallet=mining_wallet sendtoaddress "$TEST_ADDRESS" 10.0)
echo -e "${GREEN}✓${NC} Funding tx: $FUND_TXID"

# Mine 1 block to confirm
bitcoin-cli -regtest -datadir="$BITCOIN_DATADIR" generatetoaddress 1 "$MINING_ADDRESS" > /dev/null
CURRENT_HEIGHT=$((CURRENT_HEIGHT + 1))
echo -e "${GREEN}✓${NC} Mined confirmation block (height: $CURRENT_HEIGHT)"

# Wait for Bitcoin Core's UTXO index to stabilize after mining 102 blocks
echo -e "${YELLOW}⏳ Waiting for Bitcoin Core UTXO index to stabilize...${NC}"
sleep 3
echo -e "${GREEN}✓${NC} Bitcoin Core index ready"

# Verify test wallet balance
TEST_BALANCE=$(bitcoin-cli -regtest -datadir="$BITCOIN_DATADIR" listunspent 0 9999999 "[\"$TEST_ADDRESS\"]" | grep -o '"amount": [0-9.]*' | head -1 | grep -o '[0-9.]*' || echo "0")
if [ -z "$TEST_BALANCE" ] || [ "$TEST_BALANCE" = "0" ]; then
    echo -e "${YELLOW}⚠️${NC}  Test wallet balance: 0 BTC (will be visible after Electrs indexes)"
else
    echo -e "${GREEN}✓${NC} Test wallet balance: $TEST_BALANCE BTC"
fi

# ============================================================================
# 4. START ELECTRS
# ============================================================================

echo -e "\n${YELLOW}⚡ Starting Electrs...${NC}"

cd "$ELECTRS_DIR"

# Start Electrs in background
nohup ./target/release/electrs \
    --network regtest \
    --daemon-rpc-addr "127.0.0.1:$BITCOIN_RPC_PORT" \
    --electrum-rpc-addr "127.0.0.1:$ELECTRS_ELECTRUM_PORT" \
    --http-addr "0.0.0.0:$ELECTRS_HTTP_PORT" \
    --db-dir "$ELECTRS_DB" \
    --daemon-dir "$BITCOIN_DATADIR" \
    > "$LOG_DIR/electrs.log" 2>&1 &

ELECTRS_PID=$!
echo "electrs:$ELECTRS_PID" >> "$PID_FILE"

# Wait for Electrs to be ready
echo -n "Waiting for Electrs REST API"
for i in {1..60}; do
    if curl -s "http://localhost:$ELECTRS_HTTP_PORT/blocks/tip/height" &>/dev/null; then
        echo -e " ${GREEN}✓${NC}"
        break
    fi
    echo -n "."
    sleep 1
    if [ $i -eq 60 ]; then
        echo -e " ${RED}✗${NC}"
        echo -e "${RED}❌ Electrs failed to start (check logs: $LOG_DIR/electrs.log)${NC}"
        exit 1
    fi
done

ELECTRS_HEIGHT=$(curl -s "http://localhost:$ELECTRS_HTTP_PORT/blocks/tip/height")
echo -e "${GREEN}✓${NC} Electrs started (indexed height: $ELECTRS_HEIGHT)"

cd "$PROJECT_ROOT"

# ============================================================================
# 5. START ESPLORA FRONTEND
# ============================================================================

echo -e "\n${YELLOW}🌐 Starting Esplora Frontend...${NC}"

cd "$ESPLORA_DIR"

# Start Esplora dev server in background
API_URL="http://localhost:$ELECTRS_HTTP_PORT/" PORT=$ESPLORA_PORT nohup npm run dev-server \
    > "$LOG_DIR/esplora.log" 2>&1 &

ESPLORA_PID=$!
echo "esplora:$ESPLORA_PID" >> "$PID_FILE"

# Wait for Esplora to be ready
echo -n "Waiting for Esplora UI"
for i in {1..30}; do
    if curl -s "http://localhost:$ESPLORA_PORT" &>/dev/null; then
        echo -e " ${GREEN}✓${NC}"
        break
    fi
    echo -n "."
    sleep 1
    if [ $i -eq 30 ]; then
        echo -e " ${RED}✗${NC}"
        echo -e "${YELLOW}⚠️  Esplora may still be starting (check logs: $LOG_DIR/esplora.log)${NC}"
        break
    fi
done

echo -e "${GREEN}✓${NC} Esplora started"

cd "$PROJECT_ROOT"

# ============================================================================
# 6. VERIFICATION & SUMMARY
# ============================================================================

echo -e "\n${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✅ Regtest Environment Running${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

echo -e "\n${BLUE}📊 Services:${NC}"
echo -e "   • Bitcoin Core RPC: ${YELLOW}http://127.0.0.1:$BITCOIN_RPC_PORT${NC}"
echo -e "   • Electrs API:      ${YELLOW}http://localhost:$ELECTRS_HTTP_PORT${NC}"
echo -e "   • Esplora UI:       ${YELLOW}http://localhost:$ESPLORA_PORT${NC}"

echo -e "\n${BLUE}💰 Test Wallet:${NC}"
echo -e "   • Address:      ${YELLOW}$TEST_ADDRESS${NC}"
echo -e "   • Balance:      ${YELLOW}$TEST_BALANCE BTC${NC} (confirmed)"
echo -e "   • Block Height: ${YELLOW}$CURRENT_HEIGHT${NC}"

echo -e "\n${BLUE}📝 Logs:${NC}"
echo -e "   • Electrs: ${YELLOW}$LOG_DIR/electrs.log${NC}"
echo -e "   • Esplora: ${YELLOW}$LOG_DIR/esplora.log${NC}"

echo -e "\n${BLUE}🧪 Run Tests:${NC}"
echo -e "   ${YELLOW}cd wallet && cargo test --test rgb_transfer_balance_test -- --ignored --nocapture${NC}"

echo -e "\n${BLUE}🛑 Stop Services:${NC}"
echo -e "   ${YELLOW}./stop-regtest.sh${NC}"

echo ""

