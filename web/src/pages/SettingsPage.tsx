import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useAuth } from "../store/auth-store";
import * as tinvestApi from "../api/tinvest";
import type { AccountInfo } from "../api/tinvest";

export default function SettingsPage() {
  const { user, logout } = useAuth();
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const { data: tinvestStatus, isLoading } = useQuery({
    queryKey: ["tinvest-status"],
    queryFn: tinvestApi.getStatus,
  });

  // Step 1: token input
  const [token, setToken] = useState("");
  const [endpoint, setEndpoint] = useState("sandbox");
  const [sandboxAmount, setSandboxAmount] = useState("1000000");
  const [fetchingAccounts, setFetchingAccounts] = useState(false);
  const [error, setError] = useState("");

  // Step 2: account selection
  const [accounts, setAccounts] = useState<AccountInfo[] | null>(null);
  const [connecting, setConnecting] = useState(false);

  const handleFetchAccounts = async () => {
    if (!token) {
      setError("Token is required");
      return;
    }
    setFetchingAccounts(true);
    setError("");
    setAccounts(null);
    try {
      const result = await tinvestApi.fetchAccounts(
        token,
        endpoint,
        endpoint === "sandbox" ? Number(sandboxAmount) || 1000000 : undefined
      );
      if (result.length === 0) {
        setError("No accounts found for this token");
      } else {
        setAccounts(result);
      }
    } catch (e: any) {
      setError(e.response?.data?.error || "Failed to fetch accounts. Check token and endpoint.");
    } finally {
      setFetchingAccounts(false);
    }
  };

  const handleSelectAccount = async (accountId: string) => {
    setConnecting(true);
    setError("");
    try {
      await tinvestApi.connect(token, accountId, endpoint);
      queryClient.invalidateQueries({ queryKey: ["tinvest-status"] });
      setToken("");
      setAccounts(null);
    } catch (e: any) {
      setError(e.response?.data?.error || "Connection failed");
    } finally {
      setConnecting(false);
    }
  };

  const handleDisconnect = async () => {
    await tinvestApi.disconnect();
    queryClient.invalidateQueries({ queryKey: ["tinvest-status"] });
  };

  const handleLogout = () => {
    logout();
    navigate("/login");
  };

  return (
    <div className="page">
      <h2 style={{ marginBottom: 20 }}>Settings</h2>

      {/* T-Invest Connection */}
      <section className="detail-section" style={{ marginBottom: 16 }}>
        <h3>T-Bank (T-Invest) Connection</h3>

        {isLoading ? (
          <p>Loading...</p>
        ) : tinvestStatus?.connected ? (
          <div>
            <p>
              Status: <strong style={{ color: "#4caf50" }}>Connected</strong>
            </p>
            <p>Account: <strong>{tinvestStatus.account_id}</strong></p>
            <p>Endpoint: <strong>{tinvestStatus.endpoint}</strong></p>
            <button
              onClick={handleDisconnect}
              style={{
                marginTop: 12,
                color: "#f44336",
                border: "1px solid #f44336",
                background: "transparent",
                padding: "8px 16px",
                borderRadius: 8,
                cursor: "pointer",
              }}
            >
              Disconnect
            </button>
          </div>
        ) : (
          <div>
            {error && (
              <div className="error-msg" style={{ marginBottom: 12 }}>
                {error}
              </div>
            )}

            {!accounts ? (
              /* Step 1: Enter token */
              <div style={{ display: "flex", flexDirection: "column", gap: 8, maxWidth: 400 }}>
                <p style={{ color: "#888", marginBottom: 4 }}>
                  Step 1: Enter your T-Invest API token
                </p>
                <input
                  type="password"
                  placeholder="T-Invest Token (t.xxx...)"
                  value={token}
                  onChange={(e) => setToken(e.target.value)}
                  style={{
                    padding: "10px 12px",
                    border: "1px solid #ddd",
                    borderRadius: 8,
                    fontSize: 14,
                  }}
                />
                <select
                  value={endpoint}
                  onChange={(e) => setEndpoint(e.target.value)}
                  style={{
                    padding: "10px 12px",
                    border: "1px solid #ddd",
                    borderRadius: 8,
                    fontSize: 14,
                  }}
                >
                  <option value="sandbox">Sandbox</option>
                  <option value="production">Production</option>
                </select>
                {endpoint === "sandbox" && (
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <input
                      type="number"
                      placeholder="Сумма (₽)"
                      value={sandboxAmount}
                      onChange={(e) => setSandboxAmount(e.target.value)}
                      min={1000}
                      step={1000}
                      style={{
                        padding: "10px 12px",
                        border: "1px solid #ddd",
                        borderRadius: 8,
                        fontSize: 14,
                        flex: 1,
                      }}
                    />
                    <span style={{ color: "#888", fontSize: 13, whiteSpace: "nowrap" }}>₽ на виртуальный счёт</span>
                  </div>
                )}
                <button
                  className="btn-primary"
                  onClick={handleFetchAccounts}
                  disabled={fetchingAccounts}
                  style={{ alignSelf: "flex-start" }}
                >
                  {fetchingAccounts ? "Fetching accounts..." : "Get Accounts"}
                </button>
              </div>
            ) : (
              /* Step 2: Select account */
              <div>
                <p style={{ color: "#888", marginBottom: 8 }}>
                  Step 2: Select an account
                </p>
                <div style={{ display: "flex", flexDirection: "column", gap: 8, maxWidth: 400 }}>
                  {accounts.map((acc) => (
                    <button
                      key={acc.id}
                      className="strategy-card"
                      onClick={() => handleSelectAccount(acc.id)}
                      disabled={connecting}
                      style={{ width: "100%" }}
                    >
                      <strong>{acc.name}</strong>
                      <span className="meta">
                        {acc.account_type} &middot; ID: {acc.id}
                      </span>
                    </button>
                  ))}
                </div>
                <button
                  onClick={() => {
                    setAccounts(null);
                    setError("");
                  }}
                  style={{
                    marginTop: 12,
                    background: "transparent",
                    border: "1px solid #ddd",
                    padding: "8px 16px",
                    borderRadius: 8,
                    cursor: "pointer",
                  }}
                >
                  Back
                </button>
              </div>
            )}
          </div>
        )}
      </section>

      {/* User */}
      <section className="detail-section">
        <h3>Account</h3>
        <p style={{ marginBottom: 12 }}>
          Signed in as <strong>{user?.email}</strong>
        </p>
        <button
          onClick={handleLogout}
          style={{
            color: "#f44336",
            border: "1px solid #f44336",
            padding: "8px 16px",
            borderRadius: 8,
            background: "transparent",
            cursor: "pointer",
          }}
        >
          Sign Out
        </button>
      </section>
    </div>
  );
}
