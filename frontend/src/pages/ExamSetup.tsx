import { useEffect, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { api, type ExamSummary } from "../api/client";
import { useAuth } from "../auth";
import { t } from "../i18n";

/** Lancement d'un examen complet ou d'un mini-test, et historique. */
export default function ExamSetup() {
  const { user } = useAuth();
  const lang = user!.lang_pref;
  const nav = useNavigate();
  const [parts, setParts] = useState<number[]>([1, 2, 3, 4, 5, 6, 7]);
  const [count, setCount] = useState(20);
  const [timed, setTimed] = useState(false);
  const [history, setHistory] = useState<ExamSummary[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.exams().then(setHistory).catch(() => undefined);
  }, []);

  const start = async (body: Parameters<typeof api.createExam>[0]) => {
    setError(null);
    try {
      const e = await api.createExam(body);
      nav(`/exam/${e.id}`);
    } catch (err) {
      setError((err as Error).message);
    }
  };

  const toggle = (p: number) =>
    setParts((cur) => (cur.includes(p) ? cur.filter((x) => x !== p) : [...cur, p].sort()));

  return (
    <section>
      <h2>{t(lang, "exams")}</h2>
      <div className="card">
        <button className="primary" onClick={() => start({ kind: "full" })}>
          {t(lang, "fullExam")}
        </button>
      </div>

      <div className="card">
        <h3>{t(lang, "miniTest")}</h3>
        <fieldset>
          <legend>{t(lang, "parts")}</legend>
          {[1, 2, 3, 4, 5, 6, 7].map((p) => (
            <label key={p} className="inline">
              <input type="checkbox" checked={parts.includes(p)} onChange={() => toggle(p)} /> {p}
            </label>
          ))}
        </fieldset>
        <label htmlFor="mini-count">{t(lang, "count")}</label>
        <input
          id="mini-count"
          type="number"
          min={1}
          max={100}
          value={count}
          onChange={(e) => setCount(Math.max(1, Math.min(100, Number(e.target.value) || 1)))}
        />
        <label className="inline">
          <input type="checkbox" checked={timed} onChange={(e) => setTimed(e.target.checked)} /> {t(lang, "timed")}
        </label>
        <div>
          <button className="primary" disabled={parts.length === 0} onClick={() => start({ kind: "mini", parts, count, timed })}>
            {t(lang, "startExam")}
          </button>
        </div>
        {error && (
          <p role="alert" className="error">
            {error}
          </p>
        )}
      </div>

      {history.length > 0 && (
        <>
          <h3>{t(lang, "history")}</h3>
          <ul>
            {history.map((h) => (
              <li key={h.id}>
                <Link to={h.finished_at ? `/exam/${h.id}/results` : `/exam/${h.id}`}>
                  {new Date(h.started_at).toLocaleString()} — {h.kind}
                </Link>{" "}
                {h.finished_at
                  ? h.listening_scaled !== null && h.reading_scaled !== null
                    ? `${h.listening_scaled + h.reading_scaled} / 990`
                    : `L ${h.listening_scaled ?? "–"} · R ${h.reading_scaled ?? "–"}`
                  : `(${t(lang, "resume")})`}
              </li>
            ))}
          </ul>
        </>
      )}
    </section>
  );
}
