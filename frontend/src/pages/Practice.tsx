import { useEffect, useRef, useState } from "react";
import { Link, useParams, useSearchParams } from "react-router-dom";
import { api, type AnswerResult, type Question } from "../api/client";
import { useAuth } from "../auth";
import { t } from "../i18n";
import AudioPlayer from "../audio/AudioPlayer";

const LETTERS = ["A", "B", "C", "D"];

export default function Practice() {
  const { part } = useParams();
  const [params] = useSearchParams();
  const limit = Number(params.get("limit") ?? 10);
  const { user } = useAuth();
  const lang = user!.lang_pref;

  const [questions, setQuestions] = useState<Question[] | null>(null);
  const [index, setIndex] = useState(0);
  const [chosen, setChosen] = useState<number | null>(null);
  const [result, setResult] = useState<AnswerResult | null>(null);
  const [score, setScore] = useState(0);
  const [round, setRound] = useState(0);
  const shownAt = useRef(Date.now());

  useEffect(() => {
    let cancelled = false;
    setQuestions(null);
    setIndex(0);
    setScore(0);
    api.questions({ part: Number(part), limit }).then((q) => {
      if (!cancelled) setQuestions(q);
    });
    return () => {
      cancelled = true;
    };
  }, [part, limit, round]);

  useEffect(() => {
    setChosen(null);
    setResult(null);
    shownAt.current = Date.now();
  }, [index, questions]);

  if (!questions) return <p>{t(lang, "loading")}</p>;
  if (questions.length === 0) return <p>{t(lang, "noQuestions")}</p>;

  if (index >= questions.length) {
    return (
      <section className="card">
        <h2>{t(lang, "results")}</h2>
        <p className="big">
          {t(lang, "score")} : {score} / {questions.length}
        </p>
        <button className="primary" onClick={() => setRound((r) => r + 1)}>
          {t(lang, "again")}
        </button>{" "}
        <Link to="/">{t(lang, "home")}</Link>
      </section>
    );
  }

  const q = questions[index];
  // Parts 1 et 2 : les réponses ne sont pas imprimées, seulement entendues.
  const hideText = q.part === 1 || q.part === 2;
  const expl = result ? (lang === "fr" ? result.explanation_fr : result.explanation_en) : null;

  const check = async () => {
    if (chosen === null) return;
    const r = await api.answer(q.id, chosen, Date.now() - shownAt.current);
    setResult(r);
    if (r.correct) setScore((s) => s + 1);
  };

  return (
    <section>
      <p className="muted">
        {t(lang, "question")} {index + 1} / {questions.length}
      </p>
      {q.part <= 4 && q.passage?.transcript && (
        <AudioPlayer
          key={q.passage.id}
          transcript={q.passage.transcript}
          autoPlay
          labels={{
            play: t(lang, "play"),
            stop: t(lang, "stop"),
            accent: t(lang, "accent"),
            unsupported: t(lang, "audioUnsupported"),
          }}
        />
      )}
      {q.passage?.image_url && <img className="photo" src={q.passage.image_url} alt="" />}
      {q.passage?.body && (
        <article className="passage">
          {q.passage.title && <h3>{q.passage.title}</h3>}
          <pre>{q.passage.body}</pre>
        </article>
      )}
      <h2 className="stem">{q.stem}</h2>
      <ul className="choices">
        {q.choices.map((c, i) => {
          let cls = "choice";
          if (result) {
            if (i === result.correct_index) cls += " right";
            else if (i === chosen) cls += " wrong";
          } else if (i === chosen) cls += " selected";
          return (
            <li key={i}>
              <button className={cls} disabled={!!result} onClick={() => setChosen(i)}>
                <strong>{LETTERS[i]}</strong> {hideText && !result ? "" : c}
              </button>
            </li>
          );
        })}
      </ul>

      {!result && (
        <button className="primary" disabled={chosen === null} onClick={check}>
          {t(lang, "check")}
        </button>
      )}

      {result && expl && (
        <div className={`card feedback ${result.correct ? "ok" : "ko"}`}>
          <h3>{result.correct ? t(lang, "correct") : t(lang, "wrong")}</h3>
          {q.part <= 4 && (result.transcript ?? q.passage?.transcript) && (
            <details>
              <summary>{t(lang, "transcript")}</summary>
              <pre className="transcript">{(result.transcript ?? q.passage?.transcript ?? "").replace(/^[MWN]:\s*/gm, (m) => (m[0] === "N" ? "" : m))}</pre>
            </details>
          )}
          <p>
            <strong>{t(lang, "rule")} :</strong> {expl.rule}
          </p>
          <p>
            <strong>{t(lang, "whyWrong")} :</strong>
          </p>
          <ul>
            {expl.why_wrong.map((w, i) => (
              <li key={i}>{w}</li>
            ))}
          </ul>
          <p>
            <strong>{t(lang, "example")} :</strong> <em>{expl.example}</em>
          </p>
          <button className="primary" onClick={() => setIndex((i) => i + 1)}>
            {index + 1 < questions.length ? t(lang, "next") : t(lang, "finish")}
          </button>
        </div>
      )}
    </section>
  );
}
