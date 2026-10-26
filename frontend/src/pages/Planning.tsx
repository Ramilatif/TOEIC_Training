import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api, type LessonItem, type PlanData, type PlanItem } from "../api/client";
import { useAuth } from "../auth";
import { t, type Key, type Lang } from "../i18n";

const KIND_KEY: Record<PlanItem["kind"], Key> = {
  lesson: "kindLesson",
  practice: "kindPractice",
  review: "kindReview",
  flashcards: "kindFlashcards",
  mock: "kindMock",
};

/** Lien vers l'écran correspondant à un élément du planning. */
export function itemLink(it: PlanItem): string {
  switch (it.kind) {
    case "lesson":
      return `/lessons/${it.ref_id}`;
    case "practice":
      return it.ref_id?.startsWith("part-")
        ? `/practice/${it.ref_id.slice(5)}?limit=10`
        : `/practice/all?category=${it.ref_id}&limit=10`;
    case "review":
      return "/mistakes";
    case "flashcards":
      return "/cards";
    case "mock":
      return "/exams";
  }
}

function label(it: PlanItem, lang: Lang, lessons: Record<string, LessonItem>, cats: Record<string, string>): string {
  const base = t(lang, KIND_KEY[it.kind]);
  if (it.kind === "lesson" && it.ref_id && lessons[it.ref_id]) {
    const l = lessons[it.ref_id];
    return `${base} — ${lang === "fr" ? l.title_fr : l.title_en}`;
  }
  if (it.kind === "practice" && it.ref_id) {
    return `${base} — ${cats[it.ref_id] ?? it.ref_id.replace("part-", "Part ")}`;
  }
  return base;
}

export default function Planning() {
  const { user, setUser } = useAuth();
  const lang = user!.lang_pref;
  const [data, setData] = useState<PlanData | null>(null);
  const [lessons, setLessons] = useState<Record<string, LessonItem>>({});
  const [cats, setCats] = useState<Record<string, string>>({});
  const [exam, setExam] = useState(user!.exam_date ?? "");
  const [minutes, setMinutes] = useState(user!.daily_minutes);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => {
    api.plan().then(setData);
  }, []);

  useEffect(() => {
    load();
    api.lessons().then((ls) => setLessons(Object.fromEntries(ls.map((l) => [l.id, l]))));
    api.byCategory().then((cs) =>
      setCats(Object.fromEntries(cs.map((c) => [c.category, lang === "fr" ? c.label_fr : c.label_en]))),
    );
  }, [load, lang]);

  const generate = async () => {
    setError(null);
    try {
      setUser(await api.updateMe({ exam_date: exam, daily_minutes: minutes }));
      setData(await api.generatePlan());
    } catch (e) {
      setError((e as Error).message);
    }
  };

  const toggle = async (it: PlanItem) => {
    await api.setPlanItemDone(it.id, !it.done);
    load();
  };

  const today = new Date().toISOString().slice(0, 10);
  const items = data?.items ?? [];
  const days = [...new Set(items.map((i) => i.day))].filter((d) => d >= today);
  const daysLeft = data?.plan ? Math.max(0, Math.round((Date.parse(data.plan.exam_date) - Date.parse(today)) / 86400000)) : null;

  return (
    <section>
      <h2>{t(lang, "planning")}</h2>
      <div className="card">
        <div className="form-grid">
          <label>
            {t(lang, "examDate")}
            <input type="date" value={exam} onChange={(e) => setExam(e.target.value)} />
          </label>
          <label>
            {t(lang, "minutesPerDay")}
            <input
              type="number"
              min={5}
              max={600}
              value={minutes}
              onChange={(e) => setMinutes(Math.max(5, Math.min(600, Number(e.target.value) || 5)))}
            />
          </label>
          <button className="primary" disabled={!exam} onClick={generate}>
            {data?.plan ? t(lang, "regenerate") : t(lang, "generatePlan")}
          </button>
        </div>
        {error && (
          <p role="alert" className="error">
            {error}
          </p>
        )}
        {daysLeft !== null && (
          <p className="muted">
            {daysLeft} {t(lang, "daysLeft")}
          </p>
        )}
      </div>

      {!data?.plan && <p>{t(lang, "planEmpty")}</p>}

      {days.slice(0, 30).map((day) => (
        <div key={day} className={`card ${day === today ? "today" : ""}`}>
          <h3>{day === today ? `${t(lang, "planToday")} — ${day}` : day}</h3>
          <ul className="plan-list">
            {items
              .filter((i) => i.day === day)
              .map((it) => (
                <li key={it.id} className={it.done ? "done" : ""}>
                  <input
                    type="checkbox"
                    checked={it.done}
                    onChange={() => toggle(it)}
                    aria-label={`${label(it, lang, lessons, cats)} ${t(lang, "planDone")}`}
                  />{" "}
                  <Link to={itemLink(it)}>{label(it, lang, lessons, cats)}</Link>{" "}
                  <span className="muted small">{it.minutes} min</span>
                </li>
              ))}
          </ul>
        </div>
      ))}
    </section>
  );
}
