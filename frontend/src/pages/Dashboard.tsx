import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api, type CategoryStat, type Overview, type PartStat } from "../api/client";
import { useAuth } from "../auth";
import { t, type Lang } from "../i18n";

const pct = (a: number | null) => (a === null ? "—" : `${Math.round(a * 100)} %`);

/** Barre horizontale simple : la longueur code la précision, le texte donne la valeur exacte. */
function Bar({ value, label, detail }: { value: number | null; label: string; detail: string }) {
  return (
    <div className="bar-row">
      <span className="bar-label">{label}</span>
      <span className="bar-track" role="img" aria-label={`${label} ${pct(value)}`}>
        <span className="bar-fill" style={{ width: `${Math.round((value ?? 0) * 100)}%` }} />
      </span>
      <span className="bar-detail">{detail}</span>
    </div>
  );
}

/** Courbe des scores d'examens (SVG), avec la ligne d'objectif en pointillés. */
function ExamChart({ points, goal, lang }: { points: number[]; goal: number; lang: Lang }) {
  const W = 320;
  const H = 120;
  const pad = 22;
  const x = (i: number) => (points.length === 1 ? W / 2 : pad + (i * (W - 2 * pad)) / (points.length - 1));
  const y = (v: number) => H - pad - (v / 990) * (H - 2 * pad);
  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="chart" role="img" aria-label={t(lang, "evolution")}>
      <line x1={pad} x2={W - pad} y1={y(goal)} y2={y(goal)} className="goal-line" />
      <text x={W - pad} y={y(goal) - 3} textAnchor="end" className="chart-text">
        {goal}
      </text>
      <polyline fill="none" className="score-line" points={points.map((v, i) => `${x(i)},${y(v)}`).join(" ")} />
      {points.map((v, i) => (
        <g key={i}>
          <circle cx={x(i)} cy={y(v)} r={3} className="score-dot" />
          <text x={x(i)} y={y(v) - 6} textAnchor="middle" className="chart-text">
            {v}
          </text>
        </g>
      ))}
    </svg>
  );
}

export default function Dashboard() {
  const { user } = useAuth();
  const lang = user!.lang_pref;
  const [ov, setOv] = useState<Overview | null>(null);
  const [parts, setParts] = useState<PartStat[]>([]);
  const [cats, setCats] = useState<CategoryStat[]>([]);

  useEffect(() => {
    api.overview().then(setOv);
    api.byPart().then(setParts);
    api.byCategory().then(setCats);
  }, []);

  if (!ov) return <p>{t(lang, "loading")}</p>;
  const label = (c: CategoryStat) => (lang === "fr" ? c.label_fr : c.label_en);
  const examTotals = ov.exams
    .filter((e) => e.listening_scaled !== null && e.reading_scaled !== null)
    .map((e) => (e.listening_scaled ?? 0) + (e.reading_scaled ?? 0));

  return (
    <section>
      <h2>{t(lang, "dashboard")}</h2>
      {ov.questions_answered === 0 ? (
        <div className="card">
          <p>{t(lang, "emptyDashboard")}</p>
          <Link to="/train">{t(lang, "trainTitle")}</Link>
        </div>
      ) : (
        <>
          <div className="grid">
            <div className="card">
              <p className="muted">{t(lang, "estimatedScore")}</p>
              <p className="big">{ov.estimated.total ?? "—"} / 990</p>
              <p className="muted">
                L {ov.estimated.listening ?? "—"} · R {ov.estimated.reading ?? "—"}
              </p>
              {ov.parts_covered < 7 && (
                <p className="muted small">
                  {t(lang, "partsCovered")} : {ov.parts_covered}/7
                </p>
              )}
            </div>
            <div className="card">
              <p className="muted">{t(lang, "goal")}</p>
              <p className="big">{ov.goal_score}</p>
              <p>
                {ov.gap === null
                  ? t(lang, "needBothSections")
                  : ov.gap > 0
                    ? `${t(lang, "gapToGoal")} : ${ov.gap} pts`
                    : t(lang, "goalReached")}
              </p>
            </div>
            <div className="card">
              <p className="muted">{t(lang, "answeredTotal")}</p>
              <p className="big">{ov.questions_answered}</p>
              <p>
                {t(lang, "accuracy")} : {pct(ov.accuracy)}
              </p>
            </div>
          </div>

          {examTotals.length > 0 && (
            <div className="card">
              <h3>{t(lang, "evolution")}</h3>
              <ExamChart points={examTotals} goal={ov.goal_score} lang={lang} />
            </div>
          )}

          <div className="card">
            <h3>{t(lang, "byPart")}</h3>
            {parts.map((p) => (
              <Bar key={p.part} value={p.accuracy} label={`Part ${p.part}`} detail={`${p.correct}/${p.total} · ${pct(p.accuracy)}`} />
            ))}
          </div>

          <div className="card">
            <h3>{t(lang, "byCategory")}</h3>
            {cats.map((c) => (
              <Bar
                key={c.category}
                value={c.accuracy}
                label={label(c)}
                detail={`${c.correct}/${c.total} · ${pct(c.accuracy)}`}
              />
            ))}
          </div>

          {ov.weak_categories.length > 0 && (
            <div className="card">
              <h3>{t(lang, "recommended")}</h3>
              <ul>
                {ov.weak_categories.map((c) => (
                  <li key={c.category}>
                    {label(c)} ({pct(c.accuracy)}){" "}
                    <Link to={`/practice/all?category=${c.category}&limit=10`}>{t(lang, "practiceThis")}</Link>
                    {c.lesson_id && (
                      <>
                        {" · "}
                        <Link to={`/lessons/${c.lesson_id}`}>{t(lang, "readLesson")}</Link>
                      </>
                    )}
                  </li>
                ))}
              </ul>
            </div>
          )}
          <Link to="/mistakes">{t(lang, "reviewMistakes")}</Link>
        </>
      )}
    </section>
  );
}
