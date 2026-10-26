import { Link, Navigate, Route, Routes } from "react-router-dom";
import { useAuth } from "./auth";
import { api } from "./api/client";
import { t } from "./i18n";
import Login from "./pages/Login";
import Home from "./pages/Home";
import Practice from "./pages/Practice";

export default function App() {
  const { user, loading, logout, setUser } = useAuth();
  if (loading) return <p className="center">…</p>;
  if (!user) return <Login />;
  const lang = user.lang_pref;

  return (
    <>
      <header className="topbar">
        <Link to="/" className="brand">
          {t(lang, "appName")}
        </Link>
        <nav>
          <select
            aria-label={t(lang, "explanationLang")}
            value={lang}
            onChange={async (e) => setUser(await api.updateMe({ lang_pref: e.target.value as "fr" | "en" }))}
          >
            <option value="fr">FR</option>
            <option value="en">EN</option>
          </select>
          <span className="muted">{user.pseudo}</span>
          <button className="link" onClick={logout}>
            {t(lang, "logout")}
          </button>
        </nav>
      </header>
      <main className="container">
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/practice/:part" element={<Practice />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </main>
    </>
  );
}
