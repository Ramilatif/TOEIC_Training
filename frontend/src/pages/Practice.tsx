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
  const review = params.get("review") === "1";
  const category = params.get("category") ?? undefined;
  const difficulty = params.get("difficulty") ? Number(params.get("difficulty")) : undefined;
  const unseen = params.get("unseen") === "1" ? true : undefined;
  const { user } = useAuth();
  const lang = user!.lang_pref;

  const [questions, setQuestions] = useState<Question[] | null>(null);
  const [index, setIndex] = useState(0);
  const [chosen, setChosen] = useState<number | null>(null);
  const [result, setResult] = useState<AnswerResult | null>(null);
  const [score, setScore] = useState(0);
  const [round, setRound] = useState(0);
  const [vocabMsg, setVocabMsg] = useState<string | null>(null);
  const [log, setLog] = useState<{ id: string; stem: string; correct: boolean }[]>([]);
  const shownAt = useRef(Date.now());

  useEffect(() => {
    let cancelled = false;
    setQuestions(null);
    setIndex(0);
    setScore(0);
    setLog([]);
    const load = review
      ? api.mistakes({ part: part === "all" ? undefined : Number(part), category, limit }).then((m) => m.questions)
      : api.questions({ part: part === "all" ? undefined : Number(part), limit, category, difficulty, unseen });
    load.then((q) => {
      if (!cancelled) setQuestions(q);
    });
    return () => {
      cancelled = true;
    };
  }, [part, limit, round, review, category, difficulty, unseen]);

  useEffect(() => {
    setChosen(null);
    setResult(null);
    setVocabMsg(null);
    shownAt.current = Date.now();
  }, [index, questions]);

  const current = questions && index < questions.length ? questions[index] : null;
  useEffect(() => {
    if (!current) return;
    const onKey = (e: KeyboardEvent) => {
      const el = e.target as HTMLElement;
      if (el.tagName === "INPUT" || el.tagName === "SELECT" || el.tagName === "TEXTAREA") return;
      if (e.ctrlKey || e.metaKey || e.altKey) return;
      if (e.key === "Enter") {
        if (el.tagName === "BUTTON") return; // le bouton focalisé gère déjà Entrée
        e.preventDefault();
        document.querySelector<HTMLButtonElement>("button.primary[data-action]:not(:disabled)")?.click();
        return;
      }
      const letter = "abcd".indexOf(e.key.toLowerCase());
      const digit = "1234".indexOf(e.key);
      const choice = letter >= 0 ? letter : digit;
      if (choice >= 0 && choice < current.choices.length && !result) setChosen(choice);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [current, result]);

  if (!questions) return <p>{t(lang, "loading")}</p>;
  if (questions.length === 0) return <p>{t(lang, "noQuestions")}</p>;

  if (index >= questions.length) {
    return (
      <section className="card">
        <h2>{t(lang, "results")}</h2>
        <p className="big">
          {t(lang, "score")} : {score} / {questions.length}
        </p>
        <h3>{t(lang, "sessionReview")}</h3>
        <ol className="session-log">
          {log.map((l) => (
            <li key={l.id} className={l.correct ? "ok" : "ko"}>
              <span aria-hidden="true">{l.correct ? "✓" : "✗"}</span> {l.stem}
            </li>
          ))}
        </ol>
        <button className="primary" onClick={() => setRound((r) => r + 1)}>
          {t(lang, "again")}
        </button>{" "}
        {log.some((l) => !l.correct) && !review && (
          <>
            <Link to="/practice/all?review=1&limit=20">{t(lang, "reviewMyMistakes")}</Link>{" "}
          </>
        )}
        <Link to="/train">{t(lang, "trainTitle")}</Link>
      </section>
    );
  }

  const q = questions[index];
  const expl = result ? (lang === "fr" ? result.explanation_fr : result.explanation_en) : null;

  const check = async () => {
    if (chosen === null) return;
    const r = await api.answer(q.id, chosen, Date.now() - shownAt.current, review ? "review" : "practice");
    setResult(r);
    setLog((l) => [...l, { id: q.id, stem: q.stem, correct: r.correct }]);
    if (r.correct) setScore((s) => s + 1);
  };

  return (
    <section>
      <p className="muted">
        {t(lang, "question")} {index + 1} / {questions.length}
      </p>
      <span className="bar-track" role="progressbar" aria-valuemin={0} aria-valuemax={questions.length} aria-valuenow={index}>
        <span className="bar-fill" style={{ width: `${Math.round((index / questions.length) * 100)}%` }} />
      </span>
      <QuestionBody
        q={q}
        lang={lang}
        selected={chosen}
        onSelect={result ? undefined : setChosen}
        reveal={result ? { correctIndex: result.correct_index } : undefined}
      />

      {!result && (
        <>
          <button className="primary" data-action="check" disabled={chosen === null} onClick={check}>
            {t(lang, "check")}
          </button>
          <p className="muted small">{t(lang, "shortcutsHint")}</p>
        </>
      )}

      {result && expl && (
        <div className={`card feedback ${result.correct ? "ok" : "ko"}`}>
          <h3>{result.correct ? t(lang, "correct") : t(lang, "wrong")}</h3>
          <ExplanationBlock
            explanation={expl}
            transcript={q.part <= 4 ? (result.transcript ?? q.passage?.transcript ?? null) : null}
            lang={lang}
          />
          {q.part >= 5 && (
            <p>
              <button
                onClick={async () => {
                  const r = await api.cardsFromQuestion(q.id);
                  setVocabMsg(`${r.added} ${t(lang, "cardsAdded")}`);
                }}
              >
                {t(lang, "addVocabQuestion")}
              </button>{" "}
              {vocabMsg && <span role="status" className="muted">{vocabMsg}</span>}
            </p>
          )}
          {!result.correct && result.lesson_id && (
            <p>
              <Link to={`/lessons/${result.lesson_id}`}>{t(lang, "openLesson")}</Link>
            </p>
          )}
          <button className="primary" data-action="next" onClick={() => setIndex((i) => i + 1)}>
            {index + 1 < questions.length ? t(lang, "next") : t(lang, "finish")}
          </button>
        </div>
      )}
    </section>
  );
}
