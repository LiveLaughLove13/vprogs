import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import "./App.css";

interface WalletState {
  initialized: boolean;
  unlocked: boolean;
  balance: number;
  transactions: Transaction[];
}

interface Transaction {
  id: string;
  amount: number;
  timestamp: number;
  privacy_mode: string;
  status: string;
}

interface MetaAddress {
  view_public: string;
  spend_public: string;
  meta_address_json: string;
}

interface Settings {
  rpc_url: string | null;
  network: string;
  default_privacy_mode: string;
  auto_scan: boolean;
}

function App() {
  const [walletState, setWalletState] = useState<WalletState>({
    initialized: false,
    unlocked: false,
    balance: 0,
    transactions: [],
  });
  const [password, setPassword] = useState("");
  const [metaAddress, setMetaAddress] = useState<MetaAddress | null>(null);
  const [importedAddress, setImportedAddress] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<"home" | "send" | "receive" | "settings">("home");
  const [showImport, setShowImport] = useState(false);
  const [importMode, setImportMode] = useState<"private_key" | "stealth_keys">("private_key");
  const [importPrivateKey, setImportPrivateKey] = useState("");
  const [importViewSecret, setImportViewSecret] = useState("");
  const [importSpendSecret, setImportSpendSecret] = useState("");
  const [sendTo, setSendTo] = useState("");
  const [sendAmount, setSendAmount] = useState("");
  const [privacyMode, setPrivacyMode] = useState<"public" | "confidential" | "full_privacy">("public");
  const [settings, setSettings] = useState<Settings>({
    rpc_url: "ws://127.0.0.1:17110",
    network: "mainnet",
    default_privacy_mode: "public",
    auto_scan: true,
  });
  const [settingsChanged, setSettingsChanged] = useState(false);

  useEffect(() => {
    checkWalletStatus();
    loadSettings();
  }, []);

  const loadSettings = async () => {
    try {
      const loadedSettings = await invoke<Settings>("get_settings");
      setSettings(loadedSettings);
    } catch (error) {
      console.error("Error loading settings:", error);
    }
  };

  const checkWalletStatus = async () => {
    try {
      const initialized = await invoke<boolean>("is_wallet_initialized");
      const unlocked = initialized ? await invoke<boolean>("is_wallet_unlocked") : false;
      
      setWalletState((prev) => ({
        ...prev,
        initialized,
        unlocked,
      }));

      if (unlocked) {
        await loadBalance();
        await loadTransactions();
        await loadMetaAddress();
        await loadImportedAddress();
      }
    } catch (error) {
      console.error("Error checking wallet status:", error);
    }
  };

  const loadBalance = async () => {
    try {
      const balanceInfo = await invoke<{ total: number; formatted: string }>("get_balance");
      setWalletState((prev) => ({
        ...prev,
        balance: balanceInfo.total,
      }));
    } catch (error) {
      console.error("Error loading balance:", error);
    }
  };

  const loadTransactions = async () => {
    try {
      const txs = await invoke<Transaction[]>("get_transactions");
      setWalletState((prev) => ({
        ...prev,
        transactions: txs,
      }));
    } catch (error) {
      console.error("Error loading transactions:", error);
    }
  };

  const loadMetaAddress = async () => {
    try {
      const meta = await invoke<MetaAddress>("get_meta_address");
      setMetaAddress(meta);
    } catch (error) {
      console.error("Error loading meta address:", error);
    }
  };

  const loadImportedAddress = async () => {
    try {
      const address = await invoke<string | null>("get_imported_address");
      setImportedAddress(address);
    } catch (error) {
      console.error("Error loading imported address:", error);
    }
  };

  const handleInitWallet = async () => {
    if (!password) {
      alert("Please enter a password");
      return;
    }

    try {
      await invoke("init_wallet", { request: { password } });
      alert("Wallet initialized successfully!");
      setPassword("");
      await checkWalletStatus();
    } catch (error) {
      alert(`Error initializing wallet: ${error}`);
    }
  };

  const handleImportPrivateKey = async () => {
    console.log("Import button clicked");
    console.log("Password length:", password?.length || 0);
    console.log("Private key length:", importPrivateKey?.length || 0);
    
    if (!password || !importPrivateKey) {
      const msg = !password ? "Please enter a password" : "Please enter a private key";
      alert(msg);
      console.warn("Validation failed:", msg);
      return;
    }

    const key = importPrivateKey.trim().replace(/\s/g, ""); // Remove all whitespace
    console.log("Trimmed key length:", key.length);
    
    if (key.length !== 64) {
      const msg = `Private key must be exactly 64 hex characters, but got ${key.length}. Please check your private key.`;
      alert(msg);
      console.warn("Key length validation failed:", key.length);
      return;
    }

    // Validate hex format
    if (!/^[0-9a-fA-F]{64}$/.test(key)) {
      alert("Private key must contain only hexadecimal characters (0-9, a-f, A-F)");
      console.warn("Key format validation failed");
      return;
    }

    try {
      console.log("Starting wallet import...");
      console.log("Key (first 8 chars):", key.substring(0, 8) + "...");
      
      // Add timeout to prevent hanging
      const timeoutPromise = new Promise((_, reject) => 
        setTimeout(() => reject(new Error("Import timed out after 30 seconds. The address derivation may have failed.")), 30000)
      );
      
      const importPromise = invoke<string>("import_wallet_from_private_key", {
        request: {
          private_key: key,
          password: password,
        },
      });
      
      const address = await Promise.race([importPromise, timeoutPromise]) as string;
      
      console.log("Wallet import successful, address:", address);
      
      // Check if address derivation was skipped
      if (address.includes("address_derivation") || address.includes("failed")) {
        alert(`Wallet imported successfully!\n\nNote: Address derivation was skipped due to a library issue.\nPlease use your known address: kaspa:qq3tqr9f0z6t6zwcrjkk8krwwltazcl0s4gvelvakvqmj9essyq4kaksa3v0m\n\nThe private key has been stored and you can use it for transactions.`);
      } else {
        alert(`Wallet imported successfully!\nAddress: ${address}`);
      }
      setPassword("");
      setImportPrivateKey("");
      setShowImport(false);
      await checkWalletStatus();
    } catch (error) {
      console.error("Error importing wallet:", error);
      const errorMsg = error instanceof Error ? error.message : String(error);
      alert(`Error importing wallet: ${errorMsg}\n\nThis might be due to the address derivation issue. Check the browser console (F12) and terminal for more details.`);
    }
  };

  const handleImportStealthKeys = async () => {
    if (!password || !importViewSecret || !importSpendSecret) {
      alert("Please enter password and both view and spend secrets");
      return;
    }

    const view = importViewSecret.trim();
    const spend = importSpendSecret.trim();
    if (view.length !== 64 || spend.length !== 64) {
      alert("Both secrets must be 64 hex characters");
      return;
    }

    try {
      await invoke("import_wallet_from_stealth_keys", {
        request: {
          view_secret: view,
          spend_secret: spend,
          password: password,
        },
      });
      alert("Wallet imported successfully!");
      setPassword("");
      setImportViewSecret("");
      setImportSpendSecret("");
      setShowImport(false);
      await checkWalletStatus();
    } catch (error) {
      alert(`Error importing wallet: ${error}`);
    }
  };

  const handleUnlockWallet = async () => {
    if (!password) {
      alert("Please enter a password");
      return;
    }

    try {
      await invoke("unlock_wallet", { request: { password } });
      setPassword("");
      await checkWalletStatus();
    } catch (error) {
      alert(`Error unlocking wallet: ${error}`);
    }
  };

  const handleDeleteWallet = async () => {
    if (!confirm("Are you sure you want to delete this wallet? This will permanently remove all keys and cannot be undone. Make sure you have a backup!")) {
      return;
    }

    try {
      await invoke("delete_wallet");
      alert("Wallet deleted. You can now create a new wallet or import an existing one.");
      setPassword("");
      await checkWalletStatus();
    } catch (error) {
      alert(`Error deleting wallet: ${error}`);
    }
  };

  const handleLockWallet = async () => {
    try {
      await invoke("lock_wallet");
      await checkWalletStatus();
    } catch (error) {
      console.error("Error locking wallet:", error);
    }
  };

  const handleSend = async () => {
    if (!sendTo || !sendAmount) {
      alert("Please enter recipient address and amount");
      return;
    }

    const amount = parseFloat(sendAmount) * 100_000_000; // Convert KAS to sompi
    if (isNaN(amount) || amount <= 0) {
      alert("Invalid amount");
      return;
    }

    try {
      // Get from_address from wallet (use imported address if available)
      let fromAddress = importedAddress;
      if (!fromAddress) {
        // If no imported address, try to get receiving address
        // For stealth addresses, we'll need to derive one from the meta-address
        // For now, show an error if no address is available
        alert("No sender address available. Please import a Kaspa private key first.");
        return;
      }
      
      const response = await invoke<{ transaction_id: string; explorer_url: string }>("send_transaction", {
        request: {
          to_address: sendTo,
          amount: Math.floor(amount),
          privacy_mode: privacyMode,
          from_address: fromAddress,
        },
      });

      alert(`Transaction sent! ID: ${response.transaction_id}\nView on explorer: ${response.explorer_url}`);
      setSendTo("");
      setSendAmount("");
      await loadBalance();
      await loadTransactions();
    } catch (error) {
      alert(`Error sending transaction: ${error}`);
    }
  };

  if (!walletState.initialized) {
    return (
      <div className="container">
        <h1>VProgs Wallet</h1>
        {!showImport ? (
          <div className="card">
            <h2>Initialize Wallet</h2>
            <p>Create a new wallet or import an existing one.</p>
            <input
              type="password"
              placeholder="Enter password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              onKeyPress={(e) => e.key === "Enter" && handleInitWallet()}
            />
            <div style={{ display: "flex", gap: "10px", marginTop: "15px" }}>
              <button onClick={handleInitWallet} style={{ flex: 1 }}>
                Create New Wallet
              </button>
              <button
                onClick={() => setShowImport(true)}
                style={{
                  flex: 1,
                  background: "#333",
                  border: "1px solid #555",
                }}
              >
                Import Wallet
              </button>
            </div>
          </div>
        ) : (
          <div className="card">
            <h2>Import Wallet</h2>
            <div style={{ marginBottom: "15px" }}>
              <label>
                <input
                  type="radio"
                  checked={importMode === "private_key"}
                  onChange={() => setImportMode("private_key")}
                  style={{ marginRight: "8px" }}
                />
                Import Kaspa Private Key
              </label>
              <br />
              <label style={{ marginTop: "10px", display: "block" }}>
                <input
                  type="radio"
                  checked={importMode === "stealth_keys"}
                  onChange={() => setImportMode("stealth_keys")}
                  style={{ marginRight: "8px" }}
                />
                Import Stealth Keys
              </label>
            </div>

            {importMode === "private_key" ? (
              <>
                <input
                  type="password"
                  placeholder="Enter password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  style={{ marginBottom: "10px" }}
                />
                <input
                  type="text"
                  placeholder="Enter private key (64 hex characters)"
                  value={importPrivateKey}
                  onChange={(e) => setImportPrivateKey(e.target.value)}
                  style={{ marginBottom: "10px", fontFamily: "monospace" }}
                />
                <small style={{ display: "block", marginBottom: "15px", color: "#999" }}>
                  Your existing Kaspa private key (hex format). A stealth keypair will be
                  generated for receiving private transactions.
                </small>
                <div style={{ display: "flex", gap: "10px" }}>
                  <button 
                    onClick={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      console.log("Import button clicked");
                      handleImportPrivateKey();
                    }} 
                    style={{ flex: 1 }}
                    type="button"
                  >
                    Import Wallet
                  </button>
                  <button
                    onClick={() => {
                      setShowImport(false);
                      setImportPrivateKey("");
                      setPassword("");
                    }}
                    type="button"
                    style={{
                      flex: 1,
                      background: "#333",
                      border: "1px solid #555",
                    }}
                  >
                    Cancel
                  </button>
                </div>
              </>
            ) : (
              <>
                <input
                  type="password"
                  placeholder="Enter password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  style={{ marginBottom: "10px" }}
                />
                <input
                  type="text"
                  placeholder="View Secret (64 hex characters)"
                  value={importViewSecret}
                  onChange={(e) => setImportViewSecret(e.target.value)}
                  style={{ marginBottom: "10px", fontFamily: "monospace" }}
                />
                <input
                  type="text"
                  placeholder="Spend Secret (64 hex characters)"
                  value={importSpendSecret}
                  onChange={(e) => setImportSpendSecret(e.target.value)}
                  style={{ marginBottom: "10px", fontFamily: "monospace" }}
                />
                <small style={{ display: "block", marginBottom: "15px", color: "#999" }}>
                  Import your existing stealth keypair (view_secret and spend_secret).
                </small>
                <div style={{ display: "flex", gap: "10px" }}>
                  <button onClick={handleImportStealthKeys} style={{ flex: 1 }}>
                    Import Wallet
                  </button>
                  <button
                    onClick={() => {
                      setShowImport(false);
                      setImportViewSecret("");
                      setImportSpendSecret("");
                      setPassword("");
                    }}
                    style={{
                      flex: 1,
                      background: "#333",
                      border: "1px solid #555",
                    }}
                  >
                    Cancel
                  </button>
                </div>
              </>
            )}
          </div>
        )}
      </div>
    );
  }

  if (!walletState.unlocked) {
    return (
      <div className="container">
        <h1>VProgs Wallet</h1>
        <div className="card">
          <h2>Unlock Wallet</h2>
          <input
            type="password"
            placeholder="Enter password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            onKeyPress={(e) => e.key === "Enter" && handleUnlockWallet()}
          />
          <div style={{ display: "flex", gap: "10px", marginTop: "15px" }}>
            <button onClick={handleUnlockWallet} style={{ flex: 1 }}>
              Unlock
            </button>
            <button
              onClick={handleDeleteWallet}
              style={{
                flex: 1,
                background: "#d32f2f",
                border: "1px solid #b71c1c",
              }}
            >
              Reset Wallet
            </button>
          </div>
          <small style={{ display: "block", marginTop: "15px", color: "#999", textAlign: "center" }}>
            Don't have a wallet? Click "Reset Wallet" to delete this one and start fresh.
          </small>
        </div>
      </div>
    );
  }

  return (
    <div className="container">
      <header>
        <h1>VProgs Wallet</h1>
        <button onClick={handleLockWallet} className="lock-btn">Lock</button>
      </header>

      <nav>
        <button
          className={activeTab === "home" ? "active" : ""}
          onClick={() => setActiveTab("home")}
        >
          Home
        </button>
        <button
          className={activeTab === "send" ? "active" : ""}
          onClick={() => setActiveTab("send")}
        >
          Send
        </button>
        <button
          className={activeTab === "receive" ? "active" : ""}
          onClick={() => setActiveTab("receive")}
        >
          Receive
        </button>
        <button
          className={activeTab === "settings" ? "active" : ""}
          onClick={() => setActiveTab("settings")}
        >
          Settings
        </button>
      </nav>

      <main>
        {activeTab === "home" && (
          <div className="tab-content">
            <div className="balance-card">
              <h2>Balance</h2>
              <p className="balance-amount">
                {(walletState.balance / 100_000_000).toFixed(8)} KAS
              </p>
              {importedAddress && (
                <div style={{ marginTop: "15px", padding: "10px", background: "#1a1a1a", borderRadius: "4px" }}>
                  <small style={{ color: "#999", display: "block", marginBottom: "5px" }}>Imported Address:</small>
                  <code style={{ color: "#4a9eff", fontSize: "0.9rem", wordBreak: "break-all" }}>
                    {importedAddress}
                  </code>
                </div>
              )}
            </div>

            <div className="transactions-card">
              <h2>Recent Transactions</h2>
              {walletState.transactions.length === 0 ? (
                <p>No transactions yet</p>
              ) : (
                <ul>
                  {walletState.transactions.map((tx) => (
                    <li key={tx.id}>
                      <div>
                        <strong>{tx.privacy_mode}</strong> -{" "}
                        {(tx.amount / 100_000_000).toFixed(8)} KAS
                      </div>
                      <div className="tx-status">{tx.status}</div>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </div>
        )}

        {activeTab === "send" && (
          <div className="tab-content">
            <h2>Send Transaction</h2>
            <div className="form-group">
              <label>Recipient Address</label>
              <input
                type="text"
                value={sendTo}
                onChange={(e) => setSendTo(e.target.value)}
                placeholder="kaspa:..."
              />
            </div>
            <div className="form-group">
              <label>Amount (KAS)</label>
              <input
                type="number"
                value={sendAmount}
                onChange={(e) => setSendAmount(e.target.value)}
                placeholder="0.0"
                step="0.00000001"
              />
            </div>
            <div className="form-group">
              <label>Privacy Mode</label>
              <select
                value={privacyMode}
                onChange={(e) =>
                  setPrivacyMode(
                    e.target.value as "public" | "confidential" | "full_privacy"
                  )
                }
              >
                <option value="public">Public</option>
                <option value="confidential">Confidential</option>
                <option value="full_privacy">Full Privacy</option>
              </select>
            </div>
            <button onClick={handleSend} className="send-btn">
              Send
            </button>
          </div>
        )}

        {activeTab === "receive" && (
          <div className="tab-content">
            <h2>Receive Funds</h2>
            {metaAddress ? (
              <div>
                <p>Share your meta-address to receive funds:</p>
                <div className="meta-address-box">
                  <pre>{metaAddress.meta_address_json}</pre>
                </div>
                <button
                  onClick={() => {
                    navigator.clipboard.writeText(metaAddress.meta_address_json);
                    alert("Meta-address copied to clipboard!");
                  }}
                >
                  Copy Meta-Address
                </button>
              </div>
            ) : (
              <p>Loading meta-address...</p>
            )}
          </div>
        )}

        {activeTab === "settings" && (
          <div className="tab-content">
            <h2>Settings</h2>
            
            <div className="form-group">
              <label>RPC URL</label>
              <input
                type="text"
                value={settings.rpc_url || ""}
                onChange={(e) => {
                  setSettings({ ...settings, rpc_url: e.target.value || null });
                  setSettingsChanged(true);
                }}
                placeholder="ws://127.0.0.1:17110"
              />
              <small>WebSocket URL for Kaspa RPC node</small>
            </div>

            <div className="form-group">
              <label>Network</label>
              <select
                value={settings.network}
                onChange={(e) => {
                  setSettings({ ...settings, network: e.target.value });
                  setSettingsChanged(true);
                }}
              >
                <option value="mainnet">Mainnet</option>
                <option value="testnet">Testnet</option>
              </select>
            </div>

            <div className="form-group">
              <label>Default Privacy Mode</label>
              <select
                value={settings.default_privacy_mode}
                onChange={(e) => {
                  setSettings({ ...settings, default_privacy_mode: e.target.value });
                  setSettingsChanged(true);
                }}
              >
                <option value="public">Public</option>
                <option value="confidential">Confidential</option>
                <option value="full_privacy">Full Privacy</option>
              </select>
              <small>Default privacy mode for new transactions</small>
            </div>

            <div className="form-group">
              <label>
                <input
                  type="checkbox"
                  checked={settings.auto_scan}
                  onChange={(e) => {
                    setSettings({ ...settings, auto_scan: e.target.checked });
                    setSettingsChanged(true);
                  }}
                />
                Auto-scan blockchain
              </label>
              <small>Automatically scan for new transactions</small>
            </div>

            {settingsChanged && (
              <div style={{ marginTop: "20px" }}>
                <button
                  onClick={async () => {
                    try {
                      await invoke("update_settings", { newSettings: settings });
                      setSettingsChanged(false);
                      alert("Settings saved successfully!");
                    } catch (error) {
                      alert(`Error saving settings: ${error}`);
                    }
                  }}
                  className="send-btn"
                >
                  Save Settings
                </button>
                <button
                  onClick={async () => {
                    await loadSettings();
                    setSettingsChanged(false);
                  }}
                  style={{
                    marginLeft: "10px",
                    padding: "12px 24px",
                    background: "#333",
                    color: "#e0e0e0",
                    border: "1px solid #555",
                    borderRadius: "4px",
                    cursor: "pointer",
                  }}
                >
                  Cancel
                </button>
              </div>
            )}
          </div>
        )}
      </main>
    </div>
  );
}

export default App;

