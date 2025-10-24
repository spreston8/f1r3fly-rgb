### Steps

1. Import or create wallet-1
2. Fund the wallet from faucet URLs (second one works better)
3. Create UTXO - just use default amount which is 30,000 sats
	- There is a bug where the balance shown is doubled during the creation of the UTXO. This will be resolved when TX is confirmed. (Only doubled if you have balance)
4. Issue Asset
	- Fill out all details. Select UTXO with minimum 20,000 saats. Do not use F1r3fly/Rholang Execution yet.
	- Click 'Done'
	- Asset should be shown and should see UTXO locked.
5. Click 'Export' -> 'Export Genesis' -> 'Download File'
6. Import or create wallet-2
7. Click 'Import Asset' -> 'Choose File' (genesis file you just downloaded) -> 'Import Asset'
   - Should see asset now. If not, click 'Sync'
8. Fund the wallet from faucet URLs (second one works better)
9. Create UTXO - just use default amount which is 30,000 sats
	- There is a bug where the balance shown is doubled during the creation of the UTXO. This will be resolved when TX is confirmed. (Only doubled if you have balance)
10. Click 'Receive'
	- Enter Amount
	- 'Invoice Type' -> Isolated
	- Select UTXO. This part is confusing with the wording. Will be ironed out.
  - Click 'Generate Invoice'
  - Copy invoice string
11. Go back to wallet-1 and click 'Send' for your asset
	- Paste invoice string
	- Click 'Send'
	- Download 'Transfer proof'
12. Go back to wallet 2 -> click 'Accept Transfer'
	- Upload transfer file (the one you just downloaded)
	- Click 'Accept Transfer'
13. Transfer completed. Should now see correct balances in wallet-1 and wallet-2

### Notes

- These are steps required to perform asset transfer using RGB natively.
- This flow can be simplified to manage the file downloading/uploading by having centralized service manage these.
- When incorporated with F1r3fly, this flow will be drastically improved.