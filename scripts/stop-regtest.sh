#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BITCOIN_DATADIR="$PROJECT_ROOT/.bitcoin"
ELECTRS_DB="$PROJECT_ROOT/.electrs_db"
LOG_DIR="$PROJECT_ROOT/logs"
PID_FILE="$PROJECT_ROOT/.regtest-pids"

echo -e "${BLUE}🛑 Stopping Regtest Environment${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# ============================================================================
# 1. STOP SERVICES
# ============================================================================

echo -e "\n${YELLOW}🔌 Stopping services...${NC}"

# Stop Bitcoin Core
echo -n "Stopping Bitcoin Core"
if bitcoin-cli -regtest -datadir="$BITCOIN_DATADIR" stop &>/dev/null; then
    # Wait for graceful shutdown (max 15 seconds - increased for clean shutdown)
    for i in {1..15}; do
        if ! pgrep -f "bitcoind.*regtest.*$BITCOIN_DATADIR" > /dev/null; then
            echo -e " ${GREEN}✓${NC}"
            break
        fi
        echo -n "."
        sleep 1
    done
    
    # Force kill if still running
    if pgrep -f "bitcoind.*regtest.*$BITCOIN_DATADIR" > /dev/null; then
        echo -e " ${YELLOW}(force)${NC}"
        pkill -9 -f "bitcoind.*regtest.*$BITCOIN_DATADIR" || true
        sleep 2  # Additional delay after force kill
    fi
else
    echo -e " ${YELLOW}(not running)${NC}"
fi

# Stop Electrs
echo -n "Stopping Electrs"
if [ -f "$PID_FILE" ] && grep -q "electrs:" "$PID_FILE"; then
    ELECTRS_PID=$(grep "electrs:" "$PID_FILE" | cut -d: -f2)
    if kill "$ELECTRS_PID" 2>/dev/null; then
        # Wait for graceful shutdown (max 10 seconds - Electrs needs time to flush database)
        for i in {1..10}; do
            if ! kill -0 "$ELECTRS_PID" 2>/dev/null; then
                echo -e " ${GREEN}✓${NC}"
                break
            fi
            echo -n "."
            sleep 1
        done
        
        # Force kill if still running
        if kill -0 "$ELECTRS_PID" 2>/dev/null; then
            echo -e " ${YELLOW}(force)${NC}"
            kill -9 "$ELECTRS_PID" 2>/dev/null || true
            sleep 2  # Additional delay after force kill to release locks
        fi
    else
        echo -e " ${YELLOW}(PID not found)${NC}"
    fi
else
    echo -e " ${YELLOW}(PID file not found)${NC}"
fi

# Fallback: kill any remaining electrs processes
if pgrep -f "electrs --network regtest" > /dev/null; then
    pkill -f "electrs --network regtest" || true
    sleep 3  # Increased delay to ensure clean shutdown
fi

# Stop Esplora
echo -n "Stopping Esplora"
if [ -f "$PID_FILE" ] && grep -q "esplora:" "$PID_FILE"; then
    ESPLORA_PID=$(grep "esplora:" "$PID_FILE" | cut -d: -f2)
    if kill "$ESPLORA_PID" 2>/dev/null; then
        # Wait for graceful shutdown (max 5 seconds)
        for i in {1..5}; do
            if ! kill -0 "$ESPLORA_PID" 2>/dev/null; then
                echo -e " ${GREEN}✓${NC}"
                break
            fi
            echo -n "."
            sleep 1
        done
        
        # Force kill if still running
        if kill -0 "$ESPLORA_PID" 2>/dev/null; then
            echo -e " ${YELLOW}(force)${NC}"
            kill -9 "$ESPLORA_PID" 2>/dev/null || true
        fi
    else
        echo -e " ${YELLOW}(PID not found)${NC}"
    fi
else
    echo -e " ${YELLOW}(PID file not found)${NC}"
fi

# Fallback: kill any remaining esplora processes on port 5001
if lsof -ti:5001 > /dev/null 2>&1; then
    kill $(lsof -ti:5001) 2>/dev/null || true
    sleep 1
fi

# ============================================================================
# 2. CLEAN DATA DIRECTORIES
# ============================================================================

echo -e "\n${YELLOW}🗑️  Cleaning data directories...${NC}"

# Remove Bitcoin regtest data
if [ -d "$BITCOIN_DATADIR/regtest" ]; then
    echo -n "Removing Bitcoin regtest data"
    rm -rf "$BITCOIN_DATADIR/regtest"
    echo -e " ${GREEN}✓${NC}"
else
    echo -e "Bitcoin regtest data ${YELLOW}(not found)${NC}"
fi

# Remove Electrs database
if [ -d "$ELECTRS_DB" ]; then
    echo -n "Removing Electrs database"
    rm -rf "$ELECTRS_DB"
    echo -e " ${GREEN}✓${NC}"
else
    echo -e "Electrs database ${YELLOW}(not found)${NC}"
fi

# Remove PID file
if [ -f "$PID_FILE" ]; then
    echo -n "Removing PID file"
    rm -f "$PID_FILE"
    echo -e " ${GREEN}✓${NC}"
fi

# Remove log files
if [ -d "$LOG_DIR" ]; then
    echo -n "Removing log files"
    rm -f "$LOG_DIR/electrs.log"
    rm -f "$LOG_DIR/esplora.log"
    # Remove log directory if empty
    rmdir "$LOG_DIR" 2>/dev/null || true
    echo -e " ${GREEN}✓${NC}"
fi

# ============================================================================
# 3. VERIFICATION
# ============================================================================

echo -e "\n${YELLOW}🔍 Verifying cleanup...${NC}"

# Check no processes running
BITCOIN_RUNNING=$(pgrep -f "bitcoind.*regtest" | wc -l | tr -d ' ')
ELECTRS_RUNNING=$(pgrep -f "electrs --network regtest" | wc -l | tr -d ' ')
ESPLORA_RUNNING=$(lsof -ti:5001 2>/dev/null | wc -l | tr -d ' ')

if [ "$BITCOIN_RUNNING" -eq 0 ]; then
    echo -e "${GREEN}✓${NC} Bitcoin Core stopped"
else
    echo -e "${RED}✗${NC} Bitcoin Core still running ($BITCOIN_RUNNING processes)"
fi

if [ "$ELECTRS_RUNNING" -eq 0 ]; then
    echo -e "${GREEN}✓${NC} Electrs stopped"
else
    echo -e "${RED}✗${NC} Electrs still running ($ELECTRS_RUNNING processes)"
fi

if [ "$ESPLORA_RUNNING" -eq 0 ]; then
    echo -e "${GREEN}✓${NC} Esplora stopped"
else
    echo -e "${RED}✗${NC} Esplora still running ($ESPLORA_RUNNING processes)"
fi

# Check directories cleaned
if [ ! -d "$BITCOIN_DATADIR/regtest" ]; then
    echo -e "${GREEN}✓${NC} Bitcoin regtest data removed"
else
    echo -e "${RED}✗${NC} Bitcoin regtest data still exists"
fi

if [ ! -d "$ELECTRS_DB" ]; then
    echo -e "${GREEN}✓${NC} Electrs database removed"
else
    echo -e "${RED}✗${NC} Electrs database still exists"
fi

# Final delay to ensure all filesystem locks released and processes fully terminated
sleep 2

# ============================================================================
# 4. SUMMARY
# ============================================================================

echo -e "\n${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✅ Regtest Environment Stopped & Cleaned${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

echo -e "\n${BLUE}🗑️  Cleaned:${NC}"
echo -e "   • Bitcoin regtest data"
echo -e "   • Electrs database"
echo -e "   • Process tracking files"
echo -e "   • Log files"

echo -e "\n${BLUE}🚀 To restart:${NC}"
echo -e "   ${YELLOW}./start-regtest.sh${NC}"

echo ""

