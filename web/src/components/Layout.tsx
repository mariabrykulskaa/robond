import { Outlet, useNavigate } from "react-router-dom";
import { useAuth } from "../store/auth-store";

export default function Layout() {
  const { user } = useAuth();
  const navigate = useNavigate();

  return (
    <div className="app-layout">
      <header className="app-header">
        <div className="header-left">
          <h1 onClick={() => navigate("/")} style={{ cursor: "pointer" }}>
            Robond
          </h1>
        </div>
        <div className="header-right">
          <span className="user-email">{user?.email}</span>
          <button className="btn-logout" onClick={() => navigate("/settings")}>
            Settings
          </button>
        </div>
      </header>
      <main className="app-main">
        <Outlet />
      </main>
    </div>
  );
}
