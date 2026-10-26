import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { api, type ExamResults as Results } from "../api/client";
import { useAuth } from "../auth";
import { t } from "../i18n";
import QuestionBody from "../components/QuestionBody";
import ExplanationBlock from "../components/ExplanationBlock";

export default function ExamResults() {
  const { id } = useParams();
  const { user } = useAuth();
  const lang = user!.lang_pref;
  const [r, setR] = useState<Results | null>(null);
  const [open, setOpen] = useState<number | null>(null);
  const [onlyWrong, setOnlyWrong] = useState(false);

  useEffect(() => {
    api.examResults(id!).then(setR);
  }, [id]);

  if (!r) return <p>{t(lang, "loading")}</p>;
  const gap = r.total_scaled !== null ? user!.goal_score - r.total_scaled : null;

  return (
    <section>
      <h2>{t(lang, "results")}</h2>
      <div className="card">
        <p className="big">
          {r.total_scaled !== null ? `${r.total_scaled} / 990` : "—"}{" "}
          <span className="muted">({t(lang, "estimate")})</span>
        </p>
        <p>
          Listening : {r.listening.correct}/{r.listening.total}
          {r.listening.scaled !== null && ` → ${r.listening.scaled}/495`}
          <br />
          Reading : {r.reading.correct}/{r.reading.total}
          {r.reading.scaled !== null && ` → ${r.reading.scaled}/495`}
        </p>
        {gap !== null && (
          <p>
            {gap > 0
              ? `${t(lang, "gapToGoal")} ${user!.goal_score} : ${gap} pts`
              : `${t(lang, "goalReached")} (${user!.goal_score})`}
          </p>
        )}
        <ul>
          {r.by_part.map((p) => (
            <li key={p.part}>
              Part {p.part} : {p.correct}/{p.total}
            </li>
          ))}
        </ul>
      </div>

      <h3>{t(lang, "correction")}</h3>
      <label>
        <input type="checkbox" checked={onlyWrong} onChange={(e) => setOnlyWrong(e.target.checked)} />{" "}
        {t(lang, "onlyWrong")}
      </label>
      <ol className="review">
        {r.items.map((it, i) => {
          if (onlyWrong && it.correct) return null;
          const expl = lang === "fr" ? it.explanation_fr : it.explanation_en;
          return (
            <li key={it.question.id} value={i + 1}>
              <button className={`review-row ${it.correct ? "ok" : "ko"}`} onClick={() => setOpen(open === i ? null : i)}>
                {it.correct ? "✓" : "✗"} Part {it.question.part} — {it.question.stem}
              </button>
              {open === i && (
                <div className="card">
                  <QuestionBody
                    q={it.question}
                    lang={lang}
                    selected={it.chosen_index}
                    reveal={{ correctIndex: it.correct_index }}
                  />
                  <ExplanationBlock
                    explanation={expl}
                    transcript={it.question.part <= 4 ? it.transcript : null}
                    lang={lang}
                  />
                </div>
              )}
            </li>
          );
        })}
      </ol>
      <Link to="/">{t(lang, "home")}</Link>
    </section>
  );
}
