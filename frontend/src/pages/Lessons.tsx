import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api, type LessonItem, type ModuleProgress } from "../api/client";
import { useAuth } from "../auth";
import { t, type Key } from "../i18n";

const TYPES = ["method", "grammar", "vocab"] as const;

/** Liste des cours par module, avec le pourcentage de progression de chaque module. */
export default function Lessons() {
  const { user } = useAuth();
  const lang = user!.lang_pref;
  const [lessons, setLessons] = useState<LessonItem[]>([]);
  const [progress, setProgress] = useState<ModuleProgress[]>([]);

  useEffect(() => {
    api.lessons().then(setLessons);
    api.lessonProgress().then(setProgress);
  }, []);

  return (
    <section>
      <h2>{t(lang, "lessonsTitle")}</h2>
      {TYPES.map((type) => {
        const items = lessons.filter((l) => l.type === type);
        if (items.length === 0) return null;
        const pg = progress.find((p) => p.type === type);
        return (
          <div key={type} className="card">
            <h3>
              {t(lang, type as Key)}{" "}
              <span className="muted">{pg ? `${pg.percent} %` : ""}</span>
            </h3>
            {pg && (
              <span className="bar-track" role="img" aria-label={`${pg.percent} %`}>
                <span className="bar-fill" style={{ width: `${pg.percent}%` }} />
              </span>
            )}
            <ul className="lesson-list">
              {items.map((l) => (
                <li key={l.id}>
                  <Link to={`/lessons/${l.id}`}>{lang === "fr" ? l.title_fr : l.title_en}</Link>{" "}
                  <span className="muted small">
                    {l.minutes} {t(lang, "minutesShort")}
                  </span>{" "}
                  {l.quiz_passed ? (
                    <span className="badge ok">✓ {t(lang, "quizPassed")}</span>
                  ) : l.read ? (
                    <span className="badge">{t(lang, "lessonRead")}</span>
                  ) : null}
                </li>
              ))}
            </ul>
          </div>
        );
      })}
    </section>
  );
}
