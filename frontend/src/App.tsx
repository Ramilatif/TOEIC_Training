import { Link, NavLink, Navigate, Route, Routes } from "react-router-dom";
import { useAuth } from "./auth";
import { api } from "./api/client";
import { t } from "./i18n";
import Icon from "./components/Icon";
import Login from "./pages/Login";
import Home from "./pages/Home";
import Dashboard from "./pages/Dashboard";
import Lessons from "./pages/Lessons";
import LessonView from "./pages/LessonView";
import Flashcards from "./pages/Flashcards";
import Planning from "./pages/Planning";
import Mistakes from "./pages/Mistakes";
import Practice from "./pages/Practice";
import ExamSetup from "./pages/ExamSetup";
import ExamRun from "./pages/ExamRun";
import ExamResults from "./pages/ExamResults";

/** Entrées de la barre latérale : route, icône, clé i18n. */
const NAV = [
  ["/", "dashboard", "dashboard"],
  ["/train", "train", "trainTitle"],
  ["/lessons", "lessons", "lessonsTitle"],
  ["/cards", "cards", "flashcards"],
  ["/planning", "planning", "planning"],
  ["/mistakes", "mistakes", "mistakes"],
  ["/exams", "exams", "exams"],
] as const;

export default function App() {
  const { user, loading, logout, setUser } = useAuth();
  if (loading) return <p className="center">…</p>;
  if (!user) return <Login />;
  const lang = user.lang_pref;

  return (
    <div className="shell">
      <aside className="sidebar">
        <Link to="/" className="brand">
          <span className="logo">C8</span>
          {t(lang, "appName")}
        </Link>
        <nav className="side-nav">
          {NAV.map(([to, icon, key]) => (
            <NavLink key={to} to={to} end={to === "/"} className="navlink">
              <Icon name={icon} />
              <span>{t(lang, key)}</span>
            </NavLink>
          ))}
        </nav>
        <div className="side-foot">
          <select
            aria-label={t(lang, "explanationLang")}
            value={lang}
            onChange={async (e) => setUser(await api.updateMe({ lang_pref: e.target.value as "fr" | "en" }))}
          >
            <option value="fr">FR</option>
            <option value="en">EN</option>
          </select>
          <span className="muted user">{user.pseudo}</span>
          <button className="link" onClick={logout}>
            {t(lang, "logout")}
          </button>
        </div>
      </aside>
      <main className="container">
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/train" element={<Home />} />
          <Route path="/mistakes" element={<Mistakes />} />
          <Route path="/cards" element={<Flashcards />} />
          <Route path="/planning" element={<Planning />} />
          <Route path="/lessons" element={<Lessons />} />
          <Route path="/lessons/:id" element={<LessonView />} />
          <Route path="/practice/:part" element={<Practice />} />
          <Route path="/exams" element={<ExamSetup />} />
          <Route path="/exam/:id" element={<ExamRun />} />
          <Route path="/exam/:id/results" element={<ExamResults />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </main>
    </div>
  );
}
