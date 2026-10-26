import { Link, Navigate, Route, Routes } from "react-router-dom";
import { useAuth } from "./auth";
import { api } from "./api/client";
import { t } from "./i18n";
import Login from "./pages/Login";
import Home from "./pages/Home";
import Dashboard from "./pages/Dashboard";
import Lessons from "./pages/Lessons";
import LessonView from "./pages/LessonView";
import Mistakes from "./pages/Mistakes";
import Practice from "./pages/Practice";
import ExamSetup from "./pages/ExamSetup";
import ExamRun from "./pages/ExamRun";
import ExamResults from "./pages/ExamResults";

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
          <Link to="/lessons" className="navlink">
            {t(lang, "lessonsTitle")}
          </Link>
          <Link to="/train" className="navlink">
            {t(lang, "trainTitle")}
          </Link>
          <Link to="/mistakes" className="navlink">
            {t(lang, "mistakes")}
          </Link>
          <Link to="/exams" className="navlink">
            {t(lang, "exams")}
          </Link>
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
          <Route path="/" element={<Dashboard />} />
          <Route path="/train" element={<Home />} />
          <Route path="/mistakes" element={<Mistakes />} />
          <Route path="/lessons" element={<Lessons />} />
          <Route path="/lessons/:id" element={<LessonView />} />
          <Route path="/practice/:part" element={<Practice />} />
          <Route path="/exams" element={<ExamSetup />} />
          <Route path="/exam/:id" element={<ExamRun />} />
          <Route path="/exam/:id/results" element={<ExamResults />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </main>
    </>
  );
}
