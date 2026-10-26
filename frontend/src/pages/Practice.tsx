import { useEffect, useRef, useState } from "react";
import { Link, useParams, useSearchParams } from "react-router-dom";
import { api, type AnswerResult, type Question } from "../api/client";
import { useAuth } from "../auth";
import { t } from "../i18n";
import QuestionBody from "../components/QuestionBody";
import ExplanationBlock from "../components/ExplanationBlock";

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
      <QuestionBody
        q={q}
        lang={lang}
        selected={chosen}
        onSelect={result ? undefined : setChosen}
        reveal={result ? { correctIndex: result.correct_index } : undefined}
      />

      {!result && (
        <button className="primary" disabled={chosen === null} onClick={check}>
          {t(lang, "check")}
        </button>
      )}

      {result && expl && (
        <div className={`card feedback ${result.correct ? "ok" : "ko"}`}>
          <h3>{result.correct ? t(lang, "correct") : t(lang, "wrong")}</h3>
          <ExplanationBlock
            explanation={expl}
            transcript={q.part <= 4 ? (result.transcript ?? q.passage?.transcript ?? null) : null}
            lang={lang}
          />
          <button className="primary" onClick={() => setIndex((i) => i + 1)}>
            {index + 1 < questions.length ? t(lang, "next") : t(lang, "finish")}
          </button>
        </div>
      )}
    </section>
  );
}
