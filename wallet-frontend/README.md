# RGB Wallet Frontend

Modern React frontend for the RGB-compatible Bitcoin Signet wallet.

## Tech Stack

- **React 18** - UI framework
- **TypeScript** - Type safety
- **Vite** - Fast build tool
- **Tailwind CSS** - Utility-first styling
- **React Router** - Client-side routing
- **Axios** - HTTP client

## Getting Started

### Prerequisites

- Node.js 18+
- Backend server running on `http://localhost:3000`

### Installation

```bash
npm install
```

### Development

```bash
npm run dev
```

Runs on `http://localhost:5173`

### Build

```bash
npm run build
```

Output in `dist/` folder

## Environment Variables

- `VITE_API_URL` - Backend API URL (default: http://localhost:3000)

## Project Structure

```
src/
├── api/          # API client and types
├── components/   # Reusable components
├── pages/        # Page components
├── utils/        # Helper functions
└── main.tsx      # Entry point
```

## Features

- Create new wallets with BIP39 mnemonics
- Import existing wallets
- View wallet balance (confirmed/unconfirmed)
- List receive addresses
- Sync wallet with blockchain
- Display UTXOs
- Copy addresses and descriptors
