import { useState } from "react";
import { Link } from "react-router-dom";
import { usePortfolios, useCreatePortfolio, useDeletePortfolio } from "../hooks/usePortfolios";

export default function PortfolioListPage() {
  const { data: portfolios, isLoading, error } = usePortfolios();
  const createMutation = useCreatePortfolio();
  const deleteMutation = useDeletePortfolio();
  const [showModal, setShowModal] = useState(false);
  const [newName, setNewName] = useState("");

  const handleCreate = async () => {
    if (!newName.trim() || createMutation.isPending) return;
    await createMutation.mutateAsync(newName.trim());
    setNewName("");
    setShowModal(false);
  };

  if (isLoading) return <div className="loading">Loading portfolios...</div>;
  if (error) return <div className="error-msg">Failed to load portfolios</div>;

  return (
    <div className="page">
      <div className="page-header">
        <h2>Portfolios</h2>
        <button className="btn-primary" onClick={() => setShowModal(true)}>
          + New Portfolio
        </button>
      </div>

      {portfolios?.length === 0 && (
        <p className="empty-state">No portfolios yet. Create your first one!</p>
      )}

      <div className="card-grid">
        {portfolios?.map((p) => (
          <div key={p.id} className="portfolio-card" style={{ position: "relative" }}>
            <Link to={`/portfolio/${p.id}`} style={{ textDecoration: "none", color: "inherit", display: "block" }}>
              <h3>{p.name}</h3>
              <span className="meta">
                Created {new Date(p.created_at).toLocaleDateString()}
              </span>
            </Link>
            <button
              onClick={(e) => {
                e.preventDefault();
                if (confirm(`Удалить портфель "${p.name}"?`)) {
                  deleteMutation.mutate(p.id);
                }
              }}
              style={{
                position: "absolute",
                top: 8,
                right: 8,
                background: "transparent",
                border: "none",
                color: "#f44336",
                cursor: "pointer",
                fontSize: 18,
                padding: "4px 8px",
                borderRadius: 4,
              }}
              title="Удалить портфель"
            >
              ✕
            </button>
          </div>
        ))}
      </div>

      {showModal && (
        <div className="modal-overlay" onClick={() => setShowModal(false)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <h3>New Portfolio</h3>
            <input
              type="text"
              placeholder="Portfolio name"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleCreate()}
              autoFocus
            />
            <div className="modal-actions">
              <button onClick={() => setShowModal(false)}>Cancel</button>
              <button className="btn-primary" onClick={handleCreate} disabled={createMutation.isPending}>
                {createMutation.isPending ? "Creating..." : "Create"}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
