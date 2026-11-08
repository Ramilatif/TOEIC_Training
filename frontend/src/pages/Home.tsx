import { useEffect, useMemo, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { api, type CategoryItem, type QuestionSummary } from "../api/client";
import { useAuth } from "../auth";
import { t, type Key } from "../i18n";

const LISTENING: { part: number; key: Key }[] = [
  { part: 1, key: "part1" },
  { part: 2, key: "part2" },
  { part: 3, key: "part3" },
  { part: 4, key: "part4" },
];

const READING: { part: number; key: Key }[] = [
  { part: 5, key: "part5" },
  { part: 6, key: "part6" },
  { part: 7, key: "part7" },
];

const DIFFICULTIES: { value: string; key: Key }[] = [
  { value: "", key: "diffAll" },
  { value: "1", key: "diffEasy" },
  { value: "2", key: "diffMedium" },
  { value: "3", key: "diffHard" },
];

/** Écran d'entraînement : réglages de la séance, avancement par partie et accès aux parties. */
export default function Home() {
  const { user } = useAuth();
  const nav = useNavigate();
  const lang = user!.lang_pref;
  const [count, setCount] = useState(10);
  const [difficulty, setDifficulty] = useState("");
  const [category, setCategory] = useState("");
  const [onlyNew, setOnlyNew] = useState(false);
  const [summary, setSummary] = useState<QuestionSummary | null>(null);
  const [categories, setCategories] = useState<CategoryItem[]>([]);

  useEffect(() => {
    api.questionSummary().then(setSummary);
    api.categories().then(setCategories);
  }, []);

  // Catégories réellement présentes dans la banque, étiquetées dans la langue du profil.
  const options = useMemo(() => {
    const present = new Set(summary?.categories.map((c) => c.category) ?? []);
    return categories
      .filter((c) => present.has(c.id))
      .map((c) => ({ id: c.id, label: lang === "fr" ? c.label_fr : c.label_en }))
      .sort((a, b) => a.label.localeCompare(b.label));
  }, [summary, categories, lang]);

  /** Questions disponibles dans une partie avec la catégorie choisie. */
  const available = (part: number) => {
    if (!summary) return 0;
    if (!category) return summary.parts.find((p) => p.part === part)?.total ?? 0;
    return summary.categories.find((c) => c.part === part && c.category === category)?.total ?? 0;
  };

  const start = (part: number | "all") => {
    const qs = new URLSearchParams({ limit: String(count) });
    if (difficulty) qs.set("difficulty", difficulty);
    if (category) qs.set("category", category);
    if (onlyNew) qs.set("unseen", "1");
    nav(`/practice/${part}?${qs}`);
  };

  const tile = ({ part, key }: { part: number; key: Key }) => {
    const stat = summary?.parts.find((p) => p.part === part);
    const total = available(part);
    const disabled = !summary || total === 0;
    const pct = stat && stat.total > 0 ? Math.round((stat.seen / stat.total) * 100) : 0;
    return (
      <button key={part} className="card tile" disabled={disabled} onClick={() => start(part)}>
        <span>{t(lang, key)}</span>
        {summary && (
          <span className="tile-meta">
            {part === 1 && total === 0 ? (
              t(lang, "photosMissing")
            ) : (
              <>
                {stat?.seen ?? 0} / {stat?.total ?? 0} {t(lang, "seenLabel")} · {stat?.mastered ?? 0}{" "}
                {t(lang, "masteredLabel")}
              </>
            )}
          </span>
        )}
        {stat && stat.total > 0 && (
          <span className="bar-track" role="img" aria-label={`${pct} %`}>
            <span className="bar-fill" style={{ width: `${pct}%` }} />
          </span>
        )}
      </button>
    );
  };

  return (
    <section>
      <h2>{t(lang, "trainTitle")}</h2>
      <div className="card">
        <h3>{t(lang, "trainSettings")}</h3>
        <div className="form-grid">
          <label htmlFor="count">
            {t(lang, "count")}
            <input
              id="count"
              type="number"
              min={1}
              max={50}
              value={count}
              onChange={(e) => setCount(Math.max(1, Math.min(50, Number(e.target.value) || 1)))}
            />
          </label>
          <label htmlFor="difficulty">
            {t(lang, "difficulty")}
            <select id="difficulty" value={difficulty} onChange={(e) => setDifficulty(e.target.value)}>
              {DIFFICULTIES.map((d) => (
                <option key={d.value} value={d.value}>
                  {t(lang, d.key)}
                </option>
              ))}
            </select>
          </label>
          <label htmlFor="category">
            {t(lang, "categoryLabel")}
            <select id="category" value={category} onChange={(e) => setCategory(e.target.value)}>
              <option value="">{t(lang, "categoryAll")}</option>
              {options.map((o) => (
                <option key={o.id} value={o.id}>
                  {o.label}
                </option>
              ))}
            </select>
          </label>
        </div>
        <p>
          <label>
            <input type="checkbox" checked={onlyNew} onChange={(e) => setOnlyNew(e.target.checked)} />{" "}
            {t(lang, "onlyNew")}
          </label>
        </p>
      </div>

      <h2>{t(lang, "trainListening")}</h2>
      <div className="grid">{LISTENING.map(tile)}</div>
      <h2>{t(lang, "trainReading")}</h2>
      <div className="grid">{READING.map(tile)}</div>

      <p className="train-actions">
        <button className="primary" onClick={() => start("all")}>
          {t(lang, "mixAll")}
        </button>{" "}
        <Link to="/practice/all?review=1&limit=20">{t(lang, "reviewMyMistakes")}</Link>
      </p>
    </section>
  );
}
