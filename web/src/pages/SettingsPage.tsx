import { useNavigate } from "react-router-dom";
import { useAuth } from "../store/auth-store";

export default function SettingsPage() {
  const { user, logout } = useAuth();
  const navigate = useNavigate();

  const handleLogout = () => {
    logout();
    navigate("/login");
  };

  return (
    <div className="page">
      <h2 style={{ marginBottom: 20 }}>Settings</h2>

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
